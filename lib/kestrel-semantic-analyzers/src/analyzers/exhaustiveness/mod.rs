//! Exhaustiveness analyzer for match expressions.
//!
//! This analyzer checks that:
//! - Match expressions cover all possible values of the scrutinee type
//! - Match arms are not redundant (unreachable)
//! - Empty matches on inhabited types are rejected
//!
//! # Examples
//!
//! ```ignore
//! // OK: Exhaustive match
//! match color {
//!     .Red => 1,
//!     .Green => 2,
//!     .Blue => 3
//! }
//!
//! // ERROR: Non-exhaustive
//! match color {
//!     .Red => 1,
//!     .Green => 2
//!     // Missing .Blue
//! }
//!
//! // WARNING: Unreachable pattern
//! match color {
//!     .Red => 1,
//!     _ => 0,
//!     .Green => 2  // Unreachable!
//! }
//! ```

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use kestrel_semantic_pattern_matching::ExhaustivenessChecker;
use kestrel_semantic_tree::expr::{ExprKind, Expression};

mod diagnostics;
use diagnostics::{EmptyMatchError, NonExhaustiveMatchError, UnreachablePatternWarning};

pub struct ExhaustivenessAnalyzer;

impl ExhaustivenessAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ExhaustivenessAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ExhaustivenessAnalyzer {
    fn name(&self) -> &'static str {
        "exhaustiveness"
    }

    fn visit_expression(&mut self, expr: &Expression, ctx: &mut AnalysisContext) {
        if let ExprKind::Match { scrutinee, arms } = &expr.kind {
            // Get the scrutinee type
            let scrutinee_type = &scrutinee.ty;

            // Handle empty match specially
            if arms.is_empty() {
                // Empty match is only valid for uninhabited types (Never)
                if !scrutinee_type.is_never() {
                    ctx.report(EmptyMatchError {
                        match_span: expr.span.clone(),
                        scrutinee_type: format!("{}", scrutinee_type),
                    });
                }
                return;
            }

            // Collect patterns and guard info
            let patterns: Vec<_> = arms.iter().map(|arm| &arm.pattern).collect();
            let has_guards: Vec<bool> = arms.iter().map(|arm| arm.guard.is_some()).collect();

            // Run exhaustiveness check
            let checker = ExhaustivenessChecker::new(scrutinee_type);
            let result = checker.check(&patterns, &has_guards);

            // Report non-exhaustive match
            if !result.is_exhaustive {
                let missing_patterns: Vec<String> = result
                    .missing_patterns
                    .iter()
                    .map(|w| w.display())
                    .collect();

                ctx.report(NonExhaustiveMatchError {
                    match_span: expr.span.clone(),
                    missing_patterns,
                });
            }

            // Report unreachable patterns
            for &arm_index in &result.redundant_arms {
                if let Some(arm) = arms.get(arm_index) {
                    ctx.report(UnreachablePatternWarning {
                        pattern_span: arm.pattern.span.clone(),
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_has_correct_name() {
        let analyzer = ExhaustivenessAnalyzer::new();
        assert_eq!(analyzer.name(), "exhaustiveness");
    }
}
