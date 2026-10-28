//! # Closure Field Analyzer
//!
//! A stored field or enum case payload cannot hold a view-kind closure
//! (normal or `mutating`), directly or inside a tuple. A view closure holds
//! views into its defining frame; once stored in a value, a `mutating`
//! method can carry it out of that frame unseen by the escape check
//! (docs/design/closure-fields.md). Owning kinds (`escaping`, `consuming`)
//! snapshot their captures, so they are always safe to store. Storage
//! through a type parameter (`[() -> ()]`, `Optional[() -> ()]`) is the
//! `Static` bound's job, not this check's.
//!
//! ## Diagnostics
//!
//! ### E626 — `view_closure_field` (Error, Correctness)
//!
//! **Message:** "stored {what} '{name}' cannot hold a '{kind}' closure"
//!
//! **Labels:**
//! - Primary: the field or enum case
//!   - Span source: `util::entity_span` on the Field / EnumCase entity
//!     (declaration name)
//!   - Message: "this closure would keep views into the frame that created it"
//!
//! **Notes:**
//! - "write `escaping` (shared, callable many times) or `consuming` (called
//!   once) in the type, so the closure owns copies of what it captures"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast::FnTypeKind;
use kestrel_ast_builder::{FieldClass, NodeKind};
use kestrel_hir::ty::HirTy;
use kestrel_hir_lower::{LowerCallableTypes, LowerTypeAnnotation};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E626",
    name: "view_closure_field",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ClosureFieldAnalyzer;

impl Describe for ClosureFieldAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::ClosureField
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for ClosureFieldAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Field, NodeKind::EnumCase]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let (what, types) = match cx.query.get::<NodeKind>(cx.entity) {
            Some(NodeKind::Field) => {
                // Instance storage only; globals are the `Static` bound's (E505).
                if !cx
                    .query
                    .get::<FieldClass>(cx.entity)
                    .is_some_and(|c| c.is_stored_instance())
                {
                    return vec![];
                }
                let ty = cx.query.query(LowerTypeAnnotation {
                    entity: cx.entity,
                    root: cx.root,
                });
                ("field", ty.into_iter().collect::<Vec<_>>())
            },
            Some(NodeKind::EnumCase) => {
                let tys = cx
                    .query
                    .query(LowerCallableTypes {
                        entity: cx.entity,
                        root: cx.root,
                    })
                    .unwrap_or_default();
                ("enum case", tys.into_iter().flatten().collect())
            },
            _ => return vec![],
        };
        let Some(kind) = types.iter().find_map(view_closure_kind) else {
            return vec![];
        };
        let name = util::entity_name(cx.query, cx.entity);
        let kind = kind.keyword().unwrap_or("normal");
        vec![AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: format!("stored {what} '{name}' cannot hold a '{kind}' closure"),
            labels: vec![DiagLabel {
                span: util::entity_span(cx.query, cx.entity),
                message: "this closure would keep views into the frame that created it".into(),
                is_primary: true,
            }],
            notes: vec![
                "write `escaping` (shared, callable many times) or `consuming` (called once) in \
                 the type, so the closure owns copies of what it captures"
                    .into(),
            ],
        }]
    }
}

/// The kind of the first view closure stored directly in `ty`: the type
/// itself, a tuple element, or the expansion of a bounded type alias (plain
/// aliases arrive expanded). Type arguments are not walked (the `Static`
/// bound covers them).
fn view_closure_kind(ty: &HirTy) -> Option<FnTypeKind> {
    match ty {
        HirTy::Function { kind, .. } => kind.is_view().then_some(*kind),
        HirTy::Tuple(elems, _) => elems.iter().find_map(view_closure_kind),
        HirTy::Alias { expanded, .. } => view_closure_kind(expanded),
        _ => None,
    }
}
