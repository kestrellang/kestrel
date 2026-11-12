//! # View Conversion Analyzer
//!
//! Reports a borrowed parameter that converts a view argument to its owned
//! type. Type inference converts an unsized view (`&StringSlice`,
//! `&Slice[T]`) to the owned type a parameter names through `DecaysFrom`
//! (`see(x: String)` given `text.trimmed()`). For a `consuming` parameter
//! that is the point: the callee takes ownership. For a borrowed one the
//! callee only reads, so the copy is avoidable by taking the view itself.
//!
//! ## Diagnostics
//!
//! ### E512 — `borrowed_param_copies_view` (Warning, Performance)
//!
//! **Message:** "passing a '{view}' to a borrowed '{owned}' parameter copies it"
//!
//! **Labels:**
//! - Primary: the converted argument
//!   - Span source: `util::expr_span` on the argument `HirExprId` (the key
//!     of its `TypedBody::promotions` entry)
//!   - Message: "copied into a new '{owned}'"
//!
//! **Notes:**
//! - "a parameter that only reads can take `&{view}` and borrow it in place"

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, BodyCheck, Describe};
use crate::util;
use kestrel_type_infer::result::ResolvedTy;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E512",
    name: "borrowed_param_copies_view",
    default_severity: Severity::Warning,
    category: Category::Performance,
}];

pub struct ViewConversionAnalyzer;

impl Describe for ViewConversionAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::ViewConversion
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for ViewConversionAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut out: Vec<AnalyzeDiagnostic> = cx
            .typed
            .promotions
            .iter()
            .filter(|(_, p)| p.borrowed_view)
            .map(|(&arg, p)| {
                let view = cx
                    .typed
                    .expr_types
                    .get(&arg)
                    .map_or("view".into(), |t| nominal_name(cx, t));
                let owned = nominal_name(cx, &p.target);
                AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message: format!(
                        "passing a '{view}' to a borrowed '{owned}' parameter copies it"
                    ),
                    labels: vec![DiagLabel {
                        span: util::expr_span(cx.hir, arg),
                        message: format!("copied into a new '{owned}'"),
                        is_primary: true,
                    }],
                    notes: vec![format!(
                        "a parameter that only reads can take `&{view}` and borrow it in place"
                    )],
                }
            })
            .collect();
        // Deterministic order: promotions is a hash map.
        out.sort_by_key(|d| d.labels[0].span.start);
        out
    }
}

/// The written name of a (possibly referenced) nominal type: `StringSlice`
/// for `&StringSlice`, `Slice` for `Slice[Int64]`.
fn nominal_name(cx: &BodyContext<'_>, ty: &ResolvedTy) -> String {
    match ty {
        ResolvedTy::Ref { pointee, .. } => nominal_name(cx, pointee),
        ResolvedTy::Named { entity, .. } => util::entity_name(cx.query, *entity),
        _ => "view".into(),
    }
}
