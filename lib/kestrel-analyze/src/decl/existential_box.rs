//! # Existential Box Analyzer (existentials)
//!
//! `any[B] P` lowers to `B[rawany P]`, so `B` must be able to hold an
//! unsized payload: exactly one type parameter declared `not Sized`, a
//! conformance to `ExistentialBox` (its `init(consuming:)` is how a concrete
//! value gets boxed), and every stored field holding `T` only behind a
//! pointer (docs/plans/existentials/existentials-design.md §3). The rules
//! live in `kestrel_semantics::ExistentialBoxDefect`; this analyzer finds the
//! `any[B]` types and reports.
//!
//! Signatures are checked per declaration ([`ExistentialBoxAnalyzer`]),
//! `let` annotations per body ([`ExistentialBoxBodyAnalyzer`], which borrows
//! E402). Inferred local types are not re-checked: every `any[B]` a body can
//! name was written in some annotation that is.
//!
//! ## Diagnostics
//!
//! ### E402 -- `invalid_existential_box` (Error, Correctness)
//!
//! **Message:** "'{B}' cannot box an existential: {reason}"
//!
//! **Labels:**
//! - Primary: the `any[B] P` type — "'{B}' used as an existential box"

use crate::context::{BodyContext, DeclContext};
use crate::decl::unsized_param::signature_types;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, BodyCheck, DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::NodeKind;
use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::{HirStmt, HirTy};
use kestrel_semantics::{BoxDefect, ExistentialBoxDefect};
use kestrel_span::Span;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E402",
    name: "invalid_existential_box",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

static BORROWED: &[&DiagnosticDescriptor] = &[&DESCRIPTORS[0]];

pub struct ExistentialBoxAnalyzer;

impl Describe for ExistentialBoxAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::ExistentialBox
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for ExistentialBoxAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[
            NodeKind::Field,
            NodeKind::Function,
            NodeKind::Initializer,
            NodeKind::Subscript,
            NodeKind::EnumCase,
        ]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut out = Vec::new();
        for (ty, _) in signature_types(cx) {
            check_ty(cx.query, cx.root, &ty, &mut out);
        }
        out
    }
}

/// Borrows E402 for `let x: any[B] P` annotations inside bodies.
pub struct ExistentialBoxBodyAnalyzer;

impl Describe for ExistentialBoxBodyAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::ExistentialBoxBody
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        &[]
    }
    fn borrowed_descriptors(&self) -> &'static [&'static DiagnosticDescriptor] {
        BORROWED
    }
}

impl BodyCheck for ExistentialBoxBodyAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut out = Vec::new();
        for (_, stmt) in cx.hir.stmts.iter() {
            if let HirStmt::Let { ty: Some(ty), .. } = stmt {
                check_ty(cx.query, cx.root, ty, &mut out);
            }
        }
        out
    }
}

/// Report every `B[rawany P]` inside `ty` whose `B` is not a valid box.
fn check_ty(ctx: &QueryContext<'_>, root: Entity, ty: &HirTy, out: &mut Vec<AnalyzeDiagnostic>) {
    if let HirTy::Struct { entity, args, span } | HirTy::Enum { entity, args, span } = ty
        && let [HirTy::RawExistential { .. }] = args.as_slice()
        && let Some(defect) = ctx.query(ExistentialBoxDefect {
            entity: *entity,
            root,
        })
    {
        out.push(diagnostic(ctx, *entity, &defect, span));
    }
    for child in ty_children(ty) {
        check_ty(ctx, root, child, out);
    }
}

/// The immediate type children of `ty`.
fn ty_children(ty: &HirTy) -> Vec<&HirTy> {
    match ty {
        HirTy::Struct { args, .. }
        | HirTy::Enum { args, .. }
        | HirTy::Protocol { args, .. } => args.iter().collect(),
        HirTy::Alias { expanded, .. } => vec![expanded.as_ref()],
        HirTy::Tuple(elems, _) => elems.iter().collect(),
        HirTy::Function { params, ret, .. } => {
            params.iter().chain(std::iter::once(ret.as_ref())).collect()
        },
        HirTy::AssocProjection { base, .. } => vec![base.as_ref()],
        HirTy::Ref { inner, .. } => vec![inner.as_ref()],
        HirTy::RawExistential {
            bounds, bindings, ..
        } => bounds
            .iter()
            .chain(bindings.iter().map(|b| &b.ty))
            .collect(),
        HirTy::Opaque { bounds, .. } => bounds.iter().collect(),
        HirTy::Param(..)
        | HirTy::AbstractAssoc { .. }
        | HirTy::SelfType(..)
        | HirTy::Never(_)
        | HirTy::Infer(_)
        | HirTy::Error(_) => vec![],
    }
}

fn diagnostic(
    ctx: &QueryContext<'_>,
    box_entity: Entity,
    defect: &BoxDefect,
    span: &Span,
) -> AnalyzeDiagnostic {
    let name = util::entity_name(ctx, box_entity);
    let reason = match defect {
        BoxDefect::ParamCount => "it must have exactly one type parameter".to_string(),
        BoxDefect::ParamNotRelaxed => "its type parameter is not declared 'not Sized'".to_string(),
        BoxDefect::NotExistentialBox => "it does not conform to 'ExistentialBox'".to_string(),
        BoxDefect::FieldByValue { field, param } => format!(
            "field '{}' stores '{}' by value",
            util::entity_name(ctx, *field),
            util::entity_name(ctx, *param)
        ),
    };
    AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[0].id,
        severity: DESCRIPTORS[0].default_severity,
        message: format!("'{name}' cannot box an existential: {reason}"),
        labels: vec![DiagLabel {
            span: span.clone(),
            message: format!("'{name}' used as an existential box"),
            is_primary: true,
        }],
        notes: vec![],
    }
}
