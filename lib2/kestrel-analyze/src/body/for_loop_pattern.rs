//! # For-Loop Pattern Analyzer
//!
//! Checks that for-loop bindings use irrefutable patterns. In lib2 HIR,
//! for-loops are desugared to `loop { match iter.next() { .Some(pat) => body, .None => break } }`.
//! The user's pattern is embedded inside the `.Some` variant. This analyzer
//! finds these desugared matches by checking `MatchSource::ForLoop` and
//! extracts the user's pattern to check irrefutability.
//!
//! ## Diagnostics
//!
//! ### E301 -- `refutable_for_loop_pattern` (Error, Correctness)
//!
//! **Message:** "refutable pattern in for-loop binding"
//!
//! **Labels:**
//! - Primary: the user's pattern inside the desugared for-loop
//!   - Span source: `util::pat_span` on the `HirPatId`
//!   - Message: "this pattern may not match all iterator elements"
//!
//! **Notes:** "help: for-loop patterns must match every element from the iterator"

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};
use crate::util;
use kestrel_hir::body::*;
use kestrel_type_infer::result::ResolvedTy;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E301",
    name: "refutable_for_loop_pattern",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ForLoopPatternAnalyzer;

impl Describe for ForLoopPatternAnalyzer {
    fn id(&self) -> &'static str {
        "for_loop_pattern"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for ForLoopPatternAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        for (_match_id, expr) in cx.hir.exprs.iter() {
            let HirExpr::Match {
                scrutinee,
                arms,
                source: MatchSource::ForLoop,
                ..
            } = expr
            else {
                continue;
            };

            // First arm is .Some(user_pattern)
            let Some(some_arm) = arms.first() else { continue };

            // Extract the user's pattern from .Some(user_pattern)
            let Some(user_pat) = extract_user_pattern(cx.hir, some_arm.pattern) else {
                continue;
            };

            // Get the element type T from the scrutinee type Optional[T]
            let Some(element_ty) = extract_element_type(cx, *scrutinee) else {
                // Fall back to syntactic check if we can't resolve the type
                if !crate::body::refutable_pattern::is_pattern_irrefutable(cx.hir, user_pat) {
                    diags.push(make_diagnostic(cx, user_pat));
                }
                continue;
            };

            // Type-aware irrefutability check
            if !kestrel_pattern_matching::is_irrefutable(cx.hir, cx.query, cx.root, user_pat, &element_ty) {
                diags.push(make_diagnostic(cx, user_pat));
            }
        }

        diags
    }
}

/// Extract the user's pattern from inside `.Some(user_pattern)`.
/// The desugared for-loop creates `ImplicitVariant { name: "Some", args: [{ pattern }] }`.
fn extract_user_pattern(hir: &HirBody, pat_id: HirPatId) -> Option<HirPatId> {
    let HirPat::ImplicitVariant { name, args, .. } = &hir.pats[pat_id] else {
        return None;
    };
    if name != "Some" || args.len() != 1 {
        return None;
    }
    Some(args[0].pattern)
}

/// Extract the element type T from the scrutinee type Optional[T].
/// The scrutinee of the desugared match is `iter.next()` which returns `Optional[T]`.
fn extract_element_type(cx: &BodyContext<'_>, scrutinee: HirExprId) -> Option<ResolvedTy> {
    let scrutinee_ty = cx.typed.expr_types.get(&scrutinee)?;
    let ResolvedTy::Named { args, .. } = scrutinee_ty else {
        return None;
    };
    // Optional[T] has one type arg
    args.first().cloned()
}

fn make_diagnostic(cx: &BodyContext<'_>, user_pat: HirPatId) -> AnalyzeDiagnostic {
    AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[0].id,
        severity: DESCRIPTORS[0].default_severity,
        message: "refutable pattern in for-loop binding".into(),
        labels: vec![DiagLabel {
            span: util::pat_span(cx.hir, user_pat),
            message: "this pattern may not match all iterator elements".into(),
            is_primary: true,
        }],
        notes: vec![
            "help: for-loop patterns must match every element from the iterator".into(),
        ],
    }
}
