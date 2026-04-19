//! # Irrefutable Pattern Warning Analyzer
//!
//! Warns when irrefutable patterns appear in contexts that expect refutable
//! patterns. An irrefutable if-let means the condition is always true (dead
//! else branch), and an irrefutable non-last match arm makes subsequent
//! arms unreachable.
//!
//! ## Diagnostics
//!
//! ### E302 — `irrefutable_if_let` (Warning, Correctness)
//!
//! **Message:** "irrefutable pattern in if-let condition"
//!
//! **Labels:**
//! - Primary: the irrefutable pattern in the if-let
//!   - Span source: `util::pat_span` on the `HirPatId`
//!   - Message: "this pattern always matches"
//!
//! **Notes:** "help: use a plain 'let' binding instead"
//!
//! ### E303 — `irrefutable_match_arm` (Warning, Correctness)
//!
//! **Message:** "irrefutable pattern in match arm makes {n} subsequent arms unreachable"
//!
//! **Labels:**
//! - Primary: the irrefutable pattern in the non-last arm
//!   - Span source: `util::pat_span` on the `HirPatId`
//!   - Message: "this pattern always matches"
//!
//! **Notes:** (none)

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};
use crate::util;
use kestrel_hir::body::*;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E302",
        name: "irrefutable_if_let",
        default_severity: Severity::Warning,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E303",
        name: "irrefutable_match_arm",
        default_severity: Severity::Warning,
        category: Category::Correctness,
    },
];

pub struct IrrefutablePatternAnalyzer;

impl Describe for IrrefutablePatternAnalyzer {
    fn id(&self) -> &'static str {
        "irrefutable_pattern"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for IrrefutablePatternAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        for (_expr_id, expr) in cx.hir.exprs.iter() {
            let HirExpr::Match { scrutinee, arms, source, .. } = expr else {
                continue;
            };

            // Skip desugared matches — the exhaustiveness analyzer handles the
            // source-specific irrefutable diagnostic (E302/E308/E309) for
            // if-let/while-let/guard-let; other desugared forms don't warn.
            if source.is_desugared() {
                continue;
            }

            // TODO: E302 (if-let) — needs a marker to distinguish desugared if-let
            // from regular match. Skip for now.

            // E303: check match arms for irrefutable patterns before the last arm
            if arms.len() <= 1 {
                continue;
            }

            // Get scrutinee type for type-aware irrefutability check
            let scrutinee_ty = cx.typed.expr_types.get(scrutinee);

            for (i, arm) in arms.iter().enumerate() {
                // Skip the last arm — it's fine for it to be a catch-all
                if i == arms.len() - 1 {
                    break;
                }
                // Only warn if there's no guard (guards make patterns conditional)
                if arm.guard.is_some() {
                    continue;
                }

                // Use type-aware check if we have the scrutinee type
                let is_irrefutable = match scrutinee_ty {
                    Some(ty) => kestrel_pattern_matching::is_irrefutable(
                        cx.hir, cx.query, arm.pattern, ty,
                    ),
                    None => crate::body::refutable_pattern::is_pattern_irrefutable(
                        cx.hir, arm.pattern,
                    ),
                };

                if is_irrefutable {
                    let unreachable_count = arms.len() - i - 1;
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[1].id,
                        severity: DESCRIPTORS[1].default_severity,
                        message: format!(
                            "irrefutable pattern in match arm makes {} subsequent arm{} unreachable",
                            unreachable_count,
                            if unreachable_count == 1 { "" } else { "s" }
                        ),
                        labels: vec![DiagLabel {
                            span: util::pat_span(cx.hir, arm.pattern),
                            message: "this pattern always matches".into(),
                            is_primary: true,
                        }],
                        notes: vec![],
                    });
                }
            }
        }

        diags
    }
}
