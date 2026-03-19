//! # Exhaustiveness Analyzer
//!
//! Checks that match expressions cover all possible values, detects
//! unreachable arms, and warns about overlapping range patterns.
//!
//! Uses the pattern matrix algorithm (Maranget 2007) via the `pattern`
//! module for full exhaustiveness, redundancy, and overlap checking.
//!
//! ## Diagnostics
//!
//! ### KS304 — `empty_match` (Error, Correctness)
//!
//! **Message:** "empty match on inhabited type"
//!
//! **Labels:**
//! - Primary: the match expression
//!   - Span source: `util::expr_span` on the match `HirExprId`
//!   - Message: "match has no arms"
//!
//! ### KS305 — `non_exhaustive_match` (Error, Correctness)
//!
//! **Message:** "non-exhaustive match: missing {witnesses}"
//!
//! **Labels:**
//! - Primary: the match expression
//!   - Span source: `util::expr_span` on the match `HirExprId`
//!   - Message: "not all cases covered"
//!
//! **Notes:** "help: add a wildcard pattern '_' or cover the missing cases"
//!
//! ### KS306 — `unreachable_pattern` (Warning, Correctness)
//!
//! **Message:** "unreachable pattern"
//!
//! **Labels:**
//! - Primary: the unreachable pattern
//!   - Span source: `util::pat_span` on the `HirPatId`
//!   - Message: "this pattern is unreachable"
//!
//! ### KS307 — `overlapping_range` (Warning, Correctness)
//!
//! **Message:** "overlapping range patterns"
//!
//! **Labels:**
//! - Primary: the overlapping range pattern
//!   - Span source: `util::pat_span` on the `HirPatId`
//!   - Message: "this range overlaps with a previous pattern"

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};
use crate::util;
use kestrel_hir::body::*;
use kestrel_type_infer::result::ResolvedTy;

use kestrel_pattern_matching as pattern;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "KS304",
        name: "empty_match",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "KS305",
        name: "non_exhaustive_match",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "KS306",
        name: "unreachable_pattern",
        default_severity: Severity::Warning,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "KS307",
        name: "overlapping_range",
        default_severity: Severity::Warning,
        category: Category::Correctness,
    },
];

pub struct ExhaustivenessAnalyzer;

impl Describe for ExhaustivenessAnalyzer {
    fn id(&self) -> &'static str {
        "exhaustiveness"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for ExhaustivenessAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        for (expr_id, expr) in cx.hir.exprs.iter() {
            let HirExpr::Match {
                scrutinee, arms, ..
            } = expr
            else {
                continue;
            };

            // Skip for-loop desugared matches — always exhaustive by construction
            if cx.hir.for_loop_matches.contains(&expr_id) {
                continue;
            }

            // Get scrutinee type from inference results
            let Some(scrutinee_ty) = cx.typed.expr_types.get(scrutinee) else {
                continue;
            };

            // KS304: empty match on inhabited type
            if arms.is_empty() {
                let is_never = matches!(scrutinee_ty, ResolvedTy::Never);
                if !is_never {
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[0].id,
                        severity: DESCRIPTORS[0].default_severity,
                        message: "empty match on inhabited type".into(),
                        labels: vec![DiagLabel {
                            span: util::expr_span(cx.hir, expr_id),
                            message: "match has no arms".into(),
                            is_primary: true,
                        }],
                        notes: vec![],
                    });
                }
                continue;
            }

            // Run full exhaustiveness analysis via pattern module
            let result = pattern::check_match(cx.hir, cx.query, scrutinee_ty, arms);

            // KS305: non-exhaustive match
            if !result.is_exhaustive {
                let witnesses: Vec<String> =
                    result.missing_patterns.iter().map(|w| w.to_string()).collect();
                let witness_str = witnesses.join(", ");

                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[1].id,
                    severity: DESCRIPTORS[1].default_severity,
                    message: format!("non-exhaustive match: missing {}", witness_str),
                    labels: vec![DiagLabel {
                        span: util::expr_span(cx.hir, expr_id),
                        message: "not all cases covered".into(),
                        is_primary: true,
                    }],
                    notes: vec![
                        "help: add a wildcard pattern '_' or cover the missing cases".into(),
                    ],
                });
            }

            // KS306: unreachable patterns
            for &arm_idx in &result.redundant_arms {
                if let Some(arm) = arms.get(arm_idx) {
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[2].id,
                        severity: DESCRIPTORS[2].default_severity,
                        message: "unreachable pattern".into(),
                        labels: vec![DiagLabel {
                            span: util::pat_span(cx.hir, arm.pattern),
                            message: "this pattern is unreachable".into(),
                            is_primary: true,
                        }],
                        notes: vec![],
                    });
                }
            }

            // KS307: overlapping ranges
            for &arm_idx in &result.overlapping_arms {
                if let Some(arm) = arms.get(arm_idx) {
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[3].id,
                        severity: DESCRIPTORS[3].default_severity,
                        message: "overlapping range patterns".into(),
                        labels: vec![DiagLabel {
                            span: util::pat_span(cx.hir, arm.pattern),
                            message: "this range overlaps with a previous pattern".into(),
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
