//! # Closure Kind / Parameter Convention Pairing (closure kinds)
//!
//! "The kind also dictates the parameter convention needed to call it: a
//! `mutating (T) -> U` parameter must itself be `mutating`; a
//! `consuming (T) -> U` parameter must be `consuming`. The compiler enforces
//! the pairing." — docs/design/closures.md, "Passing: What Fits Where".
//!
//! Calling a `mutating`-kind closure is an exclusive use of the binding that
//! holds it, and calling a `consuming`-kind closure consumes it — neither can
//! be honoured through a borrowing parameter. `escaping` and normal kinds
//! impose nothing: their calls are shared and non-consuming.
//!
//! This is a **signature-level** fact, so it lives here rather than in the
//! solver: a bodiless declaration (a protocol requirement, an `extern`) never
//! generates a `Coerce` and so could never reach E624's coerce-path reporting.
//! The pairing is EXACT, not "at least" — `consuming` does not satisfy a
//! `mutating`-kind parameter and vice versa.
//!
//! Only the parameter's OWN type is inspected. A kind nested inside a function
//! type (`(mutating (T) -> R) -> U`) describes that function type's parameter,
//! and parameter conventions inside function types are out of scope for this
//! version of the grammar.
//!
//! ## Diagnostics
//!
//! ### E625 -- `closure_kind_convention_pairing` (Error, Correctness)

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast::{AstType, FnTypeKind};
use kestrel_ast_builder::{AstParam, Callable, NodeKind};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E625",
    name: "closure_kind_convention_pairing",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ClosureKindConventionAnalyzer;

impl Describe for ClosureKindConventionAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::ClosureKindConvention
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

/// The parameter access mode a closure kind demands, or `None` when the kind
/// imposes no pairing. `AstParam::is_mut` covers *both* `mutating` and
/// `consuming`; `is_consuming` disambiguates.
fn required_convention(kind: FnTypeKind) -> Option<&'static str> {
    match kind {
        FnTypeKind::Mutating => Some("mutating"),
        FnTypeKind::Consuming => Some("consuming"),
        FnTypeKind::Normal | FnTypeKind::Escaping => None,
    }
}

/// How the parameter is actually declared — the wording used in the label.
fn declared_convention(param: &AstParam) -> &'static str {
    match (param.is_mut, param.is_consuming) {
        (_, true) => "consuming",
        (true, false) => "mutating",
        (false, false) => "borrowing",
    }
}

impl DeclCheck for ClosureKindConventionAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[
            NodeKind::Function,
            NodeKind::Initializer,
            NodeKind::Subscript,
        ]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let Some(callable) = cx.query.get::<Callable>(cx.entity) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for param in &callable.params {
            let Some(AstType::Function { kind, .. }) = &param.ty else {
                continue;
            };
            let Some(required) = required_convention(*kind) else {
                continue;
            };
            let declared = declared_convention(param);
            if declared == required {
                continue;
            }
            let name = &param.name;
            out.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[0].id,
                severity: DESCRIPTORS[0].default_severity,
                message: format!(
                    "a '{required}' closure parameter must have the '{required}' access mode"
                ),
                labels: vec![DiagLabel {
                    // Params carry no span of their own; anchor on the
                    // declaration (a synthetic span renders as nothing).
                    span: util::entity_span(cx.query, cx.entity),
                    message: format!(
                        "parameter '{name}' has type '{required} (…) -> …' but is declared \
                         {declared}"
                    ),
                    is_primary: true,
                }],
                notes: vec![match *kind {
                    FnTypeKind::Mutating => {
                        "calling a 'mutating' closure is an exclusive use of the binding that \
                         holds it"
                            .into()
                    },
                    _ => "calling a 'consuming' closure consumes it, so the callee must own it"
                        .into(),
                }],
            });
        }
        out
    }
}
