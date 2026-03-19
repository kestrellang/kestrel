//! # Irrefutable Pattern Warning Analyzer
//!
//! Warns when irrefutable patterns appear in contexts that expect refutable
//! patterns. An irrefutable if-let means the condition is always true (dead
//! else branch), and an irrefutable non-last match arm makes subsequent
//! arms unreachable.
//!
//! ## Diagnostics
//!
//! ### KS302 — `irrefutable_if_let` (Warning, Correctness)
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
//! ### KS303 — `irrefutable_match_arm` (Warning, Correctness)
//!
//! **Message:** "irrefutable pattern in match arm makes {n} subsequent arms unreachable"
//!
//! **Labels:**
//! - Primary: the irrefutable pattern in the non-last arm
//!   - Span source: `util::pat_span` on the `HirPatId`
//!   - Message: "this pattern always matches"
//!
//! **Notes:** (none)

use crate::body::refutable_pattern::is_pattern_irrefutable;
use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};
use crate::util;
use kestrel_hir::body::*;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "KS302",
        name: "irrefutable_if_let",
        default_severity: Severity::Warning,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "KS303",
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

        for (_id, expr) in cx.hir.exprs.iter() {
            match expr {
                // Check if-let conditions for irrefutable patterns.
                // In lib2 HIR, if-let is desugared to:
                //   if { match condition { pattern => true, _ => false } }
                // But if the condition is a plain pattern match, the HIR
                // may represent it directly. We check match expressions
                // that serve as if-let conditions by looking at the match
                // structure.
                //
                // TODO: if-let detection requires distinguishing desugared
                // if-let from regular match. In the current HIR, all if-let
                // conditions are desugared. We'd need a marker to identify
                // them. Skip KS302 for now.

                // Check match arms for irrefutable patterns before the last arm
                HirExpr::Match { arms, .. } => {
                    if arms.len() > 1 {
                        for (i, arm) in arms.iter().enumerate() {
                            // Skip the last arm — it's fine for it to be a catch-all
                            if i == arms.len() - 1 {
                                break;
                            }
                            // Only warn if there's no guard (guards make patterns conditional)
                            if arm.guard.is_none()
                                && is_pattern_irrefutable(cx.hir, arm.pattern)
                            {
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
                }
                _ => {}
            }
        }

        diags
    }
}
