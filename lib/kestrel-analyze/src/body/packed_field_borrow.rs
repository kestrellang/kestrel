//! # Packed Field Borrow Analyzer
//!
//! A field of a `@layout(.Packed)` struct can sit at a misaligned address, so
//! nothing may hold a reference to it. Reads are fine: they copy the field
//! (every packed field is bitwise `Copyable`, E911), and a plain parameter
//! borrows that copy. So are whole-field writes (`h.length = 7`) and borrows
//! of the whole struct, which is aligned to 1.
//!
//! What needs the field's own address is an error:
//! - `&h.length` and `&mutating h.length`
//! - a `mutating` method called on `h.length`
//! - `h.length` passed to a `mutating` parameter
//!
//! The same holds for anything inside a packed field (`h.inner.x`), which is
//! just as misaligned.
//!
//! ## Diagnostics
//!
//! ### E912 — `packed_field_borrow` (Error, Correctness)
//!
//! **Message:** "cannot borrow '{place}' in place: '{Struct}' is packed, so the field may be misaligned"
//!
//! **Labels:**
//! - Primary: the place expression, "needs the field's address"
//!
//! **Notes:**
//! - "copy it first: 'var copy = {place}'"

use kestrel_ast_builder::attrs::{LayoutOption, attrs_of};
use kestrel_ast_builder::{Callable, FieldClass, ReceiverKind};
use kestrel_hecs::Entity;
use kestrel_hir::body::*;

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, BodyCheck, Describe};
use crate::util;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E912",
    name: "packed_field_borrow",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct PackedFieldBorrowAnalyzer;

impl Describe for PackedFieldBorrowAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::PackedFieldBorrow
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for PackedFieldBorrowAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();
        for (expr_id, expr) in cx.hir.exprs.iter() {
            for place in address_uses(cx, expr_id, expr) {
                if let Some(owner) = packed_owner(cx, place) {
                    diags.push(diagnostic(cx, place, owner));
                }
            }
        }
        diags
    }
}

/// The places `expr` needs the address of: a borrow's operand, a `mutating`
/// receiver, and arguments at `mutating` (non-consuming) parameters.
fn address_uses(cx: &BodyContext<'_>, expr_id: HirExprId, expr: &HirExpr) -> Vec<HirExprId> {
    let (callee, receiver, args) = match expr {
        HirExpr::Borrow { inner, .. } => return vec![*inner],
        HirExpr::Call { callee, args, .. } => {
            let entity = match &cx.hir.exprs[*callee] {
                HirExpr::Def(entity, _, _) => Some(*entity),
                _ => cx.typed.resolutions.get(callee).copied(),
            };
            (entity, None, args)
        },
        HirExpr::MethodCall { receiver, args, .. }
        | HirExpr::ProtocolCall { receiver, args, .. } => (
            cx.typed.resolutions.get(&expr_id).copied(),
            Some(*receiver),
            args,
        ),
        _ => return Vec::new(),
    };
    let Some(callable) = callee.and_then(|e| cx.query.get::<Callable>(e)) else {
        return Vec::new();
    };
    let mut uses = Vec::new();
    if let Some(recv) = receiver
        && matches!(callable.receiver, Some(ReceiverKind::Mutating))
    {
        uses.push(recv);
    }
    uses.extend(
        args.iter()
            .zip(&callable.params)
            .filter(|(_, p)| p.is_mut && !p.is_consuming)
            .map(|(a, _)| a.value),
    );
    uses
}

/// The packed struct `place` is a field of, directly or through enclosing
/// field projections (`h.inner.x`).
fn packed_owner(cx: &BodyContext<'_>, mut place: HirExprId) -> Option<Entity> {
    while let HirExpr::Field { base, .. } = &cx.hir.exprs[place] {
        if let Some(&field) = cx.typed.resolutions.get(&place)
            && cx
                .query
                .get::<FieldClass>(field)
                .is_some_and(FieldClass::is_stored_instance)
            && let Some(owner) = cx.query.parent_of(field)
            && attrs_of(cx.query, owner).layout() == Some(LayoutOption::Packed)
        {
            return Some(owner);
        }
        place = *base;
    }
    None
}

fn diagnostic(cx: &BodyContext<'_>, place: HirExprId, owner: Entity) -> AnalyzeDiagnostic {
    let text = place_text(cx, place);
    let d = &DESCRIPTORS[0];
    AnalyzeDiagnostic {
        descriptor_id: d.id,
        severity: d.default_severity,
        message: format!(
            "cannot borrow '{text}' in place: '{}' is packed, so the field may be misaligned",
            util::entity_name(cx.query, owner)
        ),
        labels: vec![DiagLabel {
            span: util::expr_span(cx.hir, place),
            message: "needs the field's address".into(),
            is_primary: true,
        }],
        notes: vec![format!("copy it first: 'var copy = {text}'")],
    }
}

/// `h.inner.x`, rebuilt from the projection chain.
fn place_text(cx: &BodyContext<'_>, place: HirExprId) -> String {
    match &cx.hir.exprs[place] {
        HirExpr::Field { base, name, .. } => {
            format!("{}.{}", place_text(cx, *base), name.as_str_or_empty())
        },
        HirExpr::Local(local, _) => cx.hir.locals[*local].name.clone(),
        _ => "self".into(),
    }
}
