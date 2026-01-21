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
use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::expr::{ExprKind, Expression};
use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::ty::{Ty, TyKind};

mod diagnostics;
use diagnostics::{
    EmptyMatchError, NonExhaustiveMatchError, OverlappingRangeWarning, UnreachablePatternWarning,
};

fn resolve_match_scrutinee_type(scrutinee_ty: &Ty, ctx: &AnalysisContext) -> Ty {
    // Prefer working with expanded aliases so constructor enumeration sees the real type.
    let scrutinee_ty = scrutinee_ty.expand_aliases();

    if !matches!(scrutinee_ty.kind(), TyKind::SelfType) {
        return scrutinee_ty;
    }

    // `Self` for values (e.g. `self`) is represented as TyKind::SelfType.
    // For exhaustiveness, we want a concrete enum/struct type so we can enumerate constructors.
    let Some(mut symbol) = ctx.current_symbol() else {
        return scrutinee_ty;
    };

    loop {
        match symbol.metadata().kind() {
            // Extension methods: `Self` is the extension target type (preserves substitutions).
            KestrelSymbolKind::Extension => {
                let ty = symbol
                    .metadata()
                    .get_behavior::<ExtensionTargetBehavior>()
                    .map(|b| b.target_type().clone())
                    .unwrap_or(scrutinee_ty);
                return ty.expand_aliases();
            },

            // Methods inside a struct/enum: `Self` is the container type.
            KestrelSymbolKind::Struct => {
                let ty = symbol
                    .clone()
                    .downcast_arc::<StructSymbol>()
                    .map(|sym| Ty::r#struct(sym, scrutinee_ty.span().clone()))
                    .unwrap_or(scrutinee_ty);
                return ty.expand_aliases();
            },
            KestrelSymbolKind::Enum => {
                let ty = symbol
                    .clone()
                    .downcast_arc::<EnumSymbol>()
                    .map(|sym| Ty::r#enum(sym, scrutinee_ty.span().clone()))
                    .unwrap_or(scrutinee_ty);
                return ty.expand_aliases();
            },

            // Protocol methods keep `Self` abstract (can't enumerate constructors).
            KestrelSymbolKind::Protocol => return scrutinee_ty,

            _ => {},
        }

        let Some(parent) = symbol.metadata().parent() else {
            return scrutinee_ty;
        };
        symbol = parent;
    }
}

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
            let scrutinee_type = resolve_match_scrutinee_type(&scrutinee.ty, ctx);

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
            let checker = ExhaustivenessChecker::new(&scrutinee_type);
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

            // Report overlapping range patterns
            for &arm_index in &result.overlapping_arms {
                if let Some(arm) = arms.get(arm_index) {
                    ctx.report(OverlappingRangeWarning {
                        pattern_span: arm.pattern.span.clone(),
                    });
                }
            }

            // Report unreachable patterns (only if not already reported as overlapping)
            for &arm_index in &result.redundant_arms {
                // Skip if already reported as overlapping
                if result.overlapping_arms.contains(&arm_index) {
                    continue;
                }
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
