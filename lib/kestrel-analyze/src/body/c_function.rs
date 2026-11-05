//! # CFunction Analyzer
//!
//! `CFunction(f)` (`@builtin(.CFunctionInit)`) takes the C-ABI address of
//! `f`, so `f` must name a function that has one: an `@export(.C)` Kestrel
//! function or an `@extern(.C)` import. A closure, a local holding a
//! function value, or an ordinary Kestrel function has no C entry point.
//!
//! ## Diagnostics
//!
//! ### E646 — `cfunction_needs_c_function` (Error, Correctness)
//!
//! **Message:** "'CFunction' needs a function declared '@export(.C)' or
//! '@extern(.C)'"
//!
//! **Labels:**
//! - Primary: the argument
//!   - Span source: `util::expr_span` on the argument
//!   - Message: "has no C entry point"
//!
//! **Notes:** "mark the function '@export(.C)' to give it one" (when the
//! argument names a Kestrel function).

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, BodyCheck, Describe};
use crate::util;
use kestrel_ast_builder::NodeKind;
use kestrel_ast_builder::attrs::attrs_of;
use kestrel_hir::Builtin;
use kestrel_hir::body::*;
use kestrel_name_res::EntityBuiltin;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E646",
    name: "cfunction_needs_c_function",
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

impl BodyCheck for CFunctionAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();
        for (id, expr) in cx.hir.exprs.iter() {
            let HirExpr::Call { callee, args, .. } = expr else {
                continue;
            };
            // An init call resolves on the call expression itself.
            let Some(&init) = cx
                .typed
                .resolutions
                .get(&id)
                .or(cx.typed.resolutions.get(callee))
            else {
                continue;
            };
            if cx.query.query(EntityBuiltin { entity: init }) != Some(Builtin::CFunctionInit) {
                continue;
            }
            let Some(arg) = args.first() else { continue };
            let named = cx
                .typed
                .resolutions
                .get(&arg.value)
                .copied()
                .or(match &cx.hir.exprs[arg.value] {
                    HirExpr::Def(e, _, _) => Some(*e),
                    _ => None,
                })
                .filter(|&e| cx.query.get::<NodeKind>(e) == Some(&NodeKind::Function));
            if let Some(f) = named {
                let attrs = attrs_of(cx.query, f);
                if attrs.is_extern() || attrs.is_export() {
                    continue;
                }
            }
            diags.push(AnalyzeDiagnostic {
                descriptor_id: "E646",
                severity: Severity::Error,
                message: "'CFunction' needs a function declared '@export(.C)' or '@extern(.C)'"
                    .into(),
                labels: vec![DiagLabel {
                    span: util::expr_span(cx.hir, arg.value),
                    message: "has no C entry point".into(),
                    is_primary: true,
                }],
                notes: named
                    .map(|_| "mark the function '@export(.C)' to give it one".to_string())
                    .into_iter()
                    .collect(),
            });
        }
        diags
    }
}
