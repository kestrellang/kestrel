//! # C Function Analyzer
//!
//! A value of a C function type (`@extern(.C) (…) -> …`) is the C-ABI
//! address of a function, so it can only come from naming one that has a C
//! entry point: an `@export(.C)` Kestrel function or an `@extern(.C)`
//! import. Inference lets any named function adopt the C kind (closure
//! literals are rejected there, E624); this check rejects the rest.
//!
//! `NullableCFunction[F]` reinterprets its pointer as an `F`, so `F` must be
//! a C function type.
//!
//! ## Diagnostics
//!
//! ### E647 — `c_function_needs_c_entry` (Error, Correctness)
//!
//! **Message:** "'{name}' has no C entry point; a C function type needs a
//! function declared '@export(.C)' or '@extern(.C)'", or for
//! `NullableCFunction`: "'NullableCFunction' needs a C function type
//! '@extern(.C) (…) -> …'"
//!
//! **Labels:**
//! - Primary: the function name ("has no C entry point"), or the
//!   `NullableCFunction` expression ("not a C function type")
//!   - Span source: `util::expr_span`
//!
//! **Notes:** "mark the function '@export(.C)' to give it one" (function
//! names only).

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, BodyCheck, Describe};
use crate::util;
use kestrel_ast::FnTypeKind;
use kestrel_ast_builder::NodeKind;
use kestrel_ast_builder::attrs::attrs_of;
use kestrel_hir::Builtin;
use kestrel_hir::body::*;
use kestrel_name_res::EntityBuiltin;
use kestrel_type_infer::result::ResolvedTy;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E647",
    name: "c_function_needs_c_entry",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct CFunctionAnalyzer;

impl Describe for CFunctionAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::CFunction
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

fn is_c_fn(ty: &ResolvedTy) -> bool {
    matches!(
        ty,
        ResolvedTy::Function {
            kind: FnTypeKind::C,
            ..
        }
    )
}

fn e646(
    cx: &BodyContext<'_>,
    expr: HirExprId,
    message: String,
    label: &str,
    note: Option<&str>,
) -> AnalyzeDiagnostic {
    AnalyzeDiagnostic {
        descriptor_id: "E647",
        severity: Severity::Error,
        message,
        labels: vec![DiagLabel {
            span: util::expr_span(cx.hir, expr),
            message: label.into(),
            is_primary: true,
        }],
        notes: note.map(str::to_string).into_iter().collect(),
    }
}

impl BodyCheck for CFunctionAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();
        for (id, expr) in cx.hir.exprs.iter() {
            let Some(ty) = cx.typed.expr_types.get(&id) else {
                continue;
            };
            // A function named as a C function value.
            if let HirExpr::Def(entity, _, _) = expr
                && is_c_fn(ty)
                && cx.query.get::<NodeKind>(*entity) == Some(&NodeKind::Function)
            {
                let attrs = attrs_of(cx.query, *entity);
                if !attrs.is_extern() && !attrs.is_export() {
                    let name = cx.query.get::<kestrel_ast_builder::Name>(*entity);
                    let name = name.map(|n| n.0.as_str()).unwrap_or("function");
                    diags.push(e646(
                        cx,
                        id,
                        format!(
                            "'{name}' has no C entry point; a C function type needs a function \
                             declared '@export(.C)' or '@extern(.C)'"
                        ),
                        "has no C entry point",
                        Some("mark the function '@export(.C)' to give it one"),
                    ));
                }
                continue;
            }
            // `NullableCFunction[F]` built from a non-C `F`.
            if let HirExpr::Call { .. } = expr
                && let ResolvedTy::Named { entity, args } = ty
                && cx.query.query(EntityBuiltin { entity: *entity })
                    == Some(Builtin::NullableCFunctionStruct)
                && args
                    .first()
                    .is_some_and(|f| !is_c_fn(f) && !matches!(f, ResolvedTy::Error))
            {
                diags.push(e646(
                    cx,
                    id,
                    "'NullableCFunction' needs a C function type '@extern(.C) (…) -> …'".into(),
                    "not a C function type",
                    None,
                ));
            }
        }
        diags
    }
}
