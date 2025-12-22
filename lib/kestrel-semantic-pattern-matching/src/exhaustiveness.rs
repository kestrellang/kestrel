//! Exhaustiveness checking for match expressions.
//!
//! This module implements Maranget's algorithm for checking if a set of patterns
//! is exhaustive (covers all possible values of the scrutinee type).
//!
//! # Algorithm Overview
//!
//! The algorithm works by:
//! 1. Building a "pattern matrix" where each row is a match arm
//! 2. Checking if the wildcard pattern `_` is "useful" with respect to this matrix
//! 3. If `_` is useful, the match is non-exhaustive (some values are not covered)
//!
//! # Guards and Exhaustiveness
//!
//! Patterns with guards are treated specially: a guarded pattern does NOT cover
//! its cases for exhaustiveness purposes (the guard might fail at runtime).
//!
//! ```text
//! match opt {
//!     .Some(n) if n > 0 => "positive"  // Guard might fail!
//!     .None => "nothing"
//! }
//! // Non-exhaustive: .Some(n) where n <= 0 is not covered
//! ```
//!
//! # References
//!
//! - Luc Maranget, "Warnings for pattern matching" (JFP 2007)
//! - Rust's pattern exhaustiveness checking

use crate::usefulness::is_useful;
use crate::witness::Witness;
use kestrel_semantic_tree::pattern::Pattern;
use kestrel_semantic_tree::ty::Ty;

/// Result of exhaustiveness checking.
#[derive(Debug, Clone)]
pub struct ExhaustivenessResult {
    /// Whether all possible values are covered by the patterns
    pub is_exhaustive: bool,

    /// Examples of uncovered values (witnesses) if non-exhaustive.
    /// Each witness demonstrates a value that would not be matched.
    pub missing_patterns: Vec<Witness>,

    /// Indices of redundant (unreachable) match arms.
    /// A pattern is redundant if it can never match because all its
    /// cases are already covered by previous patterns.
    pub redundant_arms: Vec<usize>,
}

impl ExhaustivenessResult {
    /// Create a result indicating the match is exhaustive
    pub fn exhaustive() -> Self {
        ExhaustivenessResult {
            is_exhaustive: true,
            missing_patterns: vec![],
            redundant_arms: vec![],
        }
    }

    /// Create a result indicating the match is non-exhaustive
    pub fn non_exhaustive(missing: Vec<Witness>) -> Self {
        ExhaustivenessResult {
            is_exhaustive: false,
            missing_patterns: missing,
            redundant_arms: vec![],
        }
    }

    /// Add a redundant arm index
    pub fn with_redundant_arm(mut self, index: usize) -> Self {
        self.redundant_arms.push(index);
        self
    }
}

/// Context for exhaustiveness checking.
///
/// Provides type information needed to enumerate constructors
/// (e.g., all cases of an enum type).
pub struct ExhaustivenessChecker<'a> {
    /// The type of the scrutinee being matched
    scrutinee_type: &'a Ty,
}

impl<'a> ExhaustivenessChecker<'a> {
    /// Create a new exhaustiveness checker for the given scrutinee type.
    pub fn new(scrutinee_type: &'a Ty) -> Self {
        ExhaustivenessChecker { scrutinee_type }
    }

    /// Check if the given patterns are exhaustive for the scrutinee type.
    ///
    /// # Arguments
    ///
    /// * `patterns` - The patterns from match arms (in order)
    /// * `has_guards` - Whether each arm has a guard (parallel to patterns)
    ///
    /// # Returns
    ///
    /// An `ExhaustivenessResult` indicating:
    /// - Whether the match is exhaustive
    /// - Missing patterns (witnesses) if non-exhaustive
    /// - Redundant arm indices
    pub fn check(&self, patterns: &[&Pattern], has_guards: &[bool]) -> ExhaustivenessResult {
        if patterns.is_empty() {
            // Empty match - need at least one pattern for any type except Never
            if self.scrutinee_type.is_never() {
                return ExhaustivenessResult::exhaustive();
            }
            return ExhaustivenessResult::non_exhaustive(vec![Witness::any()]);
        }

        // Check for redundant patterns and build the effective pattern list
        let mut redundant_arms = Vec::new();
        let mut effective_patterns: Vec<&Pattern> = Vec::new();

        for (i, (pattern, &has_guard)) in patterns.iter().zip(has_guards.iter()).enumerate() {
            // Check if this pattern is useful given the previous patterns
            if !effective_patterns.is_empty()
                && !is_useful(pattern, &effective_patterns, self.scrutinee_type)
            {
                // Pattern is not useful (redundant) - but only if it doesn't have a guard
                // Guarded patterns might still be useful even if structurally redundant
                if !has_guard {
                    redundant_arms.push(i);
                }
            }

            // Add pattern to effective list (guards don't cover cases for exhaustiveness)
            if !has_guard {
                effective_patterns.push(pattern);
            }
        }

        // Check if the patterns are exhaustive
        // We do this by checking if a wildcard pattern would be useful
        let wildcard =
            Pattern::wildcard(self.scrutinee_type.clone(), self.scrutinee_type.span().clone());

        let is_exhaustive = !is_useful(&wildcard, &effective_patterns, self.scrutinee_type);

        let mut result = if is_exhaustive {
            ExhaustivenessResult::exhaustive()
        } else {
            // Generate witnesses for missing patterns
            let witnesses = self.generate_witnesses(&effective_patterns);
            ExhaustivenessResult::non_exhaustive(witnesses)
        };

        result.redundant_arms = redundant_arms;
        result
    }

    /// Generate witnesses for missing patterns.
    ///
    /// This is a simplified implementation that generates basic witnesses.
    /// A full implementation would use Maranget's witness generation algorithm.
    fn generate_witnesses(&self, patterns: &[&Pattern]) -> Vec<Witness> {
        use kestrel_semantic_tree::pattern::PatternKind;
        use kestrel_semantic_tree::ty::TyKind;
        use semantic_tree::symbol::Symbol;

        // Simple witness generation based on type
        match self.scrutinee_type.kind() {
            TyKind::Bool => {
                // Check which boolean values are covered
                let has_true = patterns.iter().any(|p| {
                    matches!(
                        &p.kind,
                        PatternKind::Literal { value }
                            if matches!(
                                value,
                                kestrel_semantic_tree::expr::LiteralValue::Bool(true)
                            )
                    ) || matches!(
                        &p.kind,
                        PatternKind::Wildcard | PatternKind::Local { .. }
                    )
                });
                let has_false = patterns.iter().any(|p| {
                    matches!(
                        &p.kind,
                        PatternKind::Literal { value }
                            if matches!(
                                value,
                                kestrel_semantic_tree::expr::LiteralValue::Bool(false)
                            )
                    ) || matches!(
                        &p.kind,
                        PatternKind::Wildcard | PatternKind::Local { .. }
                    )
                });

                let mut witnesses = Vec::new();
                if !has_true {
                    witnesses.push(Witness::bool(true));
                }
                if !has_false {
                    witnesses.push(Witness::bool(false));
                }
                if witnesses.is_empty() {
                    // Should be exhaustive, but we're here so something's wrong
                    witnesses.push(Witness::any());
                }
                witnesses
            }

            TyKind::Enum { symbol, .. } => {
                // Get all cases from the enum symbol
                let cases = symbol.cases();

                // Check which enum cases are covered
                let covered_cases: Vec<&str> = patterns
                    .iter()
                    .filter_map(|p| match &p.kind {
                        PatternKind::EnumVariant { case_name, .. } => Some(case_name.as_str()),
                        _ => None,
                    })
                    .collect();

                // If any pattern is a wildcard/binding, all cases are covered
                let has_catch_all = patterns.iter().any(|p| {
                    matches!(
                        &p.kind,
                        PatternKind::Wildcard | PatternKind::Local { .. }
                    )
                });

                if has_catch_all {
                    return vec![Witness::any()];
                }

                // Generate witnesses for uncovered cases
                let mut witnesses = Vec::new();
                for case in &cases {
                    let name = case.metadata().name();
                    let case_name = name.value.as_str();
                    if !covered_cases.contains(&case_name) {
                        // TODO: Include associated value witnesses if the case has them
                        witnesses.push(Witness::enum_case(case_name));
                    }
                }

                if witnesses.is_empty() {
                    witnesses.push(Witness::any());
                }
                witnesses
            }

            TyKind::Tuple(elements) => {
                // For tuples, we'd need to recursively check each position
                // Simplified: just return a tuple of wildcards
                let witnesses: Vec<Witness> = (0..elements.len()).map(|_| Witness::any()).collect();
                vec![Witness::tuple(witnesses)]
            }

            // For types with infinite constructors (Int, String, etc.),
            // we need a wildcard to be exhaustive
            TyKind::Int(_) | TyKind::Float(_) | TyKind::String => {
                vec![Witness::any()]
            }

            _ => {
                // Default: return a generic witness
                vec![Witness::any()]
            }
        }
    }
}

/// Convenience function to check exhaustiveness.
///
/// # Arguments
///
/// * `patterns` - The patterns from match arms
/// * `scrutinee_type` - The type being matched
///
/// # Returns
///
/// An `ExhaustivenessResult` indicating exhaustiveness and any issues.
pub fn check_exhaustiveness(patterns: &[&Pattern], scrutinee_type: &Ty) -> ExhaustivenessResult {
    let has_guards = vec![false; patterns.len()];
    let checker = ExhaustivenessChecker::new(scrutinee_type);
    checker.check(patterns, &has_guards)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_semantic_tree::expr::LiteralValue;
    use kestrel_semantic_tree::pattern::Mutability;
    use kestrel_semantic_tree::symbol::local::LocalId;
    use kestrel_semantic_tree::ty::IntBits;
    use kestrel_span::Span;

    fn test_span() -> Span {
        Span::from(0..1)
    }

    fn int_ty() -> Ty {
        Ty::int(IntBits::I64, test_span())
    }

    fn bool_ty() -> Ty {
        Ty::bool(test_span())
    }

    #[test]
    fn test_empty_match_non_exhaustive() {
        let patterns: Vec<&Pattern> = vec![];
        let result = check_exhaustiveness(&patterns, &int_ty());
        assert!(!result.is_exhaustive);
        assert!(!result.missing_patterns.is_empty());
    }

    #[test]
    fn test_wildcard_exhaustive() {
        let pattern = Pattern::wildcard(int_ty(), test_span());
        let patterns = vec![&pattern];
        let result = check_exhaustiveness(&patterns, &int_ty());
        assert!(result.is_exhaustive);
        assert!(result.missing_patterns.is_empty());
    }

    #[test]
    fn test_binding_exhaustive() {
        let pattern = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "x".to_string(),
            int_ty(),
            test_span(),
        );
        let patterns = vec![&pattern];
        let result = check_exhaustiveness(&patterns, &int_ty());
        assert!(result.is_exhaustive);
    }

    #[test]
    fn test_bool_both_values_exhaustive() {
        let true_pattern = Pattern::literal(LiteralValue::Bool(true), bool_ty(), test_span());
        let false_pattern = Pattern::literal(LiteralValue::Bool(false), bool_ty(), test_span());
        let patterns = vec![&true_pattern, &false_pattern];

        let result = check_exhaustiveness(&patterns, &bool_ty());
        assert!(result.is_exhaustive);
    }

    #[test]
    fn test_bool_one_value_non_exhaustive() {
        let true_pattern = Pattern::literal(LiteralValue::Bool(true), bool_ty(), test_span());
        let patterns = vec![&true_pattern];

        let result = check_exhaustiveness(&patterns, &bool_ty());
        assert!(!result.is_exhaustive);
        assert!(result
            .missing_patterns
            .iter()
            .any(|w| { matches!(w, Witness::Bool(false)) }));
    }

    #[test]
    fn test_redundant_pattern_detected() {
        let wildcard = Pattern::wildcard(int_ty(), test_span());
        let literal = Pattern::literal(LiteralValue::Integer(42), int_ty(), test_span());
        let patterns = vec![&wildcard, &literal];

        let result = check_exhaustiveness(&patterns, &int_ty());
        assert!(result.is_exhaustive);
        assert!(result.redundant_arms.contains(&1));
    }

    #[test]
    fn test_duplicate_patterns_redundant() {
        let lit1 = Pattern::literal(LiteralValue::Integer(42), int_ty(), test_span());
        let lit2 = Pattern::literal(LiteralValue::Integer(42), int_ty(), test_span());
        let wildcard = Pattern::wildcard(int_ty(), test_span());
        let patterns = vec![&lit1, &lit2, &wildcard];

        let result = check_exhaustiveness(&patterns, &int_ty());
        assert!(result.is_exhaustive);
        assert!(result.redundant_arms.contains(&1)); // Second 42 is redundant
    }

    #[test]
    fn test_guards_dont_cover() {
        // When all patterns have guards, match should be non-exhaustive
        // (even if patterns would otherwise be exhaustive)
        let bool_type = bool_ty();
        let true_pattern = Pattern::literal(LiteralValue::Bool(true), bool_ty(), test_span());
        let false_pattern = Pattern::literal(LiteralValue::Bool(false), bool_ty(), test_span());
        let patterns = vec![&true_pattern, &false_pattern];
        let has_guards = vec![true, true]; // Both have guards

        let checker = ExhaustivenessChecker::new(&bool_type);
        let result = checker.check(&patterns, &has_guards);

        // Should be non-exhaustive because guards might fail
        assert!(!result.is_exhaustive);
    }

    #[test]
    fn test_partial_guards_need_fallback() {
        // One guarded, one not - the unguarded one covers its case
        let bool_type = bool_ty();
        let true_pattern = Pattern::literal(LiteralValue::Bool(true), bool_ty(), test_span());
        let false_pattern = Pattern::literal(LiteralValue::Bool(false), bool_ty(), test_span());
        let patterns = vec![&true_pattern, &false_pattern];
        let has_guards = vec![true, false]; // Only first has guard

        let checker = ExhaustivenessChecker::new(&bool_type);
        let result = checker.check(&patterns, &has_guards);

        // Non-exhaustive: true case has guard so doesn't count
        assert!(!result.is_exhaustive);
    }
}
