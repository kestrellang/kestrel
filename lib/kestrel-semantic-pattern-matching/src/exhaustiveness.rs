//! Exhaustiveness checking for match expressions.
//!
//! This module implements Maranget's algorithm for checking if a set of patterns
//! is exhaustive (covers all possible values of the scrutinee type).
//!
//! # Algorithm Overview
//!
//! The algorithm works by:
//! 1. Building a pattern matrix where each row is a match arm
//! 2. Checking if the wildcard pattern `_` is "useful" with respect to this matrix
//! 3. If `_` is useful, the match is non-exhaustive (some values are not covered)
//! 4. Also checking each arm for redundancy by testing if it's useful against prior arms
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
//! - Rust's pattern exhaustiveness checking (`rustc_pattern_analysis`)

use crate::constructor::Constructor;
use crate::matrix::{PatternMatrix, PatternRow};
use crate::usefulness::is_useful_impl;
use crate::witness::Witness;
use kestrel_semantic_tree::pattern::{Pattern, PatternKind, RangeBound};
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

    /// Indices of arms with overlapping range patterns.
    /// A range pattern overlaps when part of its range is covered by
    /// previous patterns but the pattern is still partially reachable.
    pub overlapping_arms: Vec<usize>,
}

impl ExhaustivenessResult {
    /// Create a result indicating the match is exhaustive
    pub fn exhaustive() -> Self {
        ExhaustivenessResult {
            is_exhaustive: true,
            missing_patterns: vec![],
            redundant_arms: vec![],
            overlapping_arms: vec![],
        }
    }

    /// Create a result indicating the match is non-exhaustive
    pub fn non_exhaustive(missing: Vec<Witness>) -> Self {
        ExhaustivenessResult {
            is_exhaustive: false,
            missing_patterns: missing,
            redundant_arms: vec![],
            overlapping_arms: vec![],
        }
    }

    /// Add a redundant arm index
    pub fn with_redundant_arm(mut self, index: usize) -> Self {
        self.redundant_arms.push(index);
        self
    }

    /// Add an overlapping arm index
    pub fn with_overlapping_arm(mut self, index: usize) -> Self {
        self.overlapping_arms.push(index);
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
    /// - Overlapping arm indices
    pub fn check(&self, patterns: &[&Pattern], has_guards: &[bool]) -> ExhaustivenessResult {
        if patterns.is_empty() {
            // Empty match - need at least one pattern for any type except Never
            if self.scrutinee_type.is_never() {
                return ExhaustivenessResult::exhaustive();
            }
            return ExhaustivenessResult::non_exhaustive(vec![Witness::any()]);
        }

        // Build the pattern matrix and check for redundancy
        let mut matrix = PatternMatrix::single_column(self.scrutinee_type.clone());
        let mut redundant_arms = Vec::new();
        let mut overlapping_arms = Vec::new();

        // Track previous range patterns for overlap detection
        let mut previous_ranges: Vec<(usize, i64, i64)> = Vec::new();

        for (i, (pattern, &has_guard)) in patterns.iter().zip(has_guards.iter()).enumerate() {
            // Check if this pattern is useful given the previous patterns
            // For or-patterns, we check usefulness of the whole or-pattern
            let query = PatternRow::new(vec![(*pattern).clone()], i, has_guard);
            let usefulness = is_useful_impl(&matrix, &query);

            // Check for overlapping ranges before checking redundancy
            if let Some((current_start, current_end)) = extract_int_range(pattern) {
                // Check if this range overlaps with any previous range
                let has_overlap = previous_ranges.iter().any(|&(_, prev_start, prev_end)| {
                    ranges_overlap(current_start, current_end, prev_start, prev_end)
                });

                if has_overlap && !has_guard {
                    // This range overlaps with a previous range
                    // Note: We detect overlaps independently of the usefulness check because
                    // the usefulness algorithm doesn't handle partial range overlaps correctly
                    overlapping_arms.push(i);
                }

                // Track this range for future overlap checks
                if !has_guard {
                    previous_ranges.push((i, current_start, current_end));
                }
            }

            if !usefulness.is_useful && !has_guard {
                // Pattern is not useful (redundant)
                // Guarded patterns might still be useful even if structurally redundant
                redundant_arms.push(i);
            }

            // Add pattern to matrix (guards don't cover cases for exhaustiveness)
            // For or-patterns, we expand them to multiple rows
            if !has_guard {
                let expanded = expand_or_patterns(pattern);
                for alt in expanded {
                    let row = PatternRow::new(vec![alt], i, has_guard);
                    matrix.push(row);
                }
            }
        }

        // Check exhaustiveness: is a wildcard useful?
        let wildcard =
            Pattern::wildcard(self.scrutinee_type.clone(), self.scrutinee_type.span().clone());
        let wildcard_row = PatternRow::new(vec![wildcard.clone()], patterns.len(), false);

        let exhaustiveness_result = is_useful_impl(&matrix, &wildcard_row);
        let is_exhaustive = !exhaustiveness_result.is_useful;

        let mut result = if is_exhaustive {
            ExhaustivenessResult::exhaustive()
        } else {
            // Generate witnesses for missing patterns
            let witnesses = self.generate_witnesses(&matrix, &wildcard);
            ExhaustivenessResult::non_exhaustive(witnesses)
        };

        result.redundant_arms = redundant_arms;
        result.overlapping_arms = overlapping_arms;
        result
    }

    /// Generate witnesses for missing patterns.
    ///
    /// Uses the constructor-based approach to find uncovered constructors.
    fn generate_witnesses(&self, matrix: &PatternMatrix, _wildcard: &Pattern) -> Vec<Witness> {
        use std::collections::HashSet;

        // Get covered constructors from the matrix
        let covered: HashSet<Constructor> =
            matrix.unique_head_constructors().into_iter().collect();

        // Get all constructors for the type
        match Constructor::all_constructors(self.scrutinee_type) {
            Some(all_ctors) => {
                // Find uncovered constructors
                let missing: Vec<_> = all_ctors
                    .into_iter()
                    .filter(|c| !covered.contains(c))
                    .collect();

                if missing.is_empty() {
                    // All top-level constructors covered, but there might be
                    // uncovered nested patterns. For now, return generic witness.
                    vec![Witness::any()]
                } else {
                    // Convert missing constructors to witnesses
                    missing
                        .iter()
                        .map(|c| self.constructor_to_witness(c))
                        .collect()
                }
            }
            None => {
                // Infinite constructor type - need a wildcard
                vec![Witness::any()]
            }
        }
    }

    /// Convert a constructor to a witness.
    fn constructor_to_witness(&self, ctor: &Constructor) -> Witness {
        match ctor {
            Constructor::True => Witness::bool(true),
            Constructor::False => Witness::bool(false),
            Constructor::Unit => Witness::tuple(vec![]),
            Constructor::Variant { name, arity } => {
                if *arity == 0 {
                    Witness::enum_case(name)
                } else {
                    Witness::enum_case_with_args(name, vec![Witness::any(); *arity])
                }
            }
            Constructor::Tuple { arity } => Witness::tuple(vec![Witness::any(); *arity]),
            Constructor::Struct { name, .. } => {
                Witness::struct_witness(name, vec![])
            }
            Constructor::IntLiteral(n) => Witness::integer(*n),
            Constructor::IntRange { start, end } => {
                Witness::range(start.to_string(), end.to_string(), true)
            }
            Constructor::CharLiteral(c) => Witness::Literal(format!("'{}'", c)),
            Constructor::CharRange { start, end } => {
                Witness::range(format!("'{}'", start), format!("'{}'", end), true)
            }
            Constructor::StringLiteral(s) => Witness::string(s),
            Constructor::Array { prefix_len, suffix_len, .. } => {
                Witness::array(vec![Witness::any(); prefix_len + suffix_len])
            }
            Constructor::Wildcard | Constructor::NonExhaustive | Constructor::Missing => {
                Witness::any()
            }
        }
    }
}

/// Extract integer range bounds from a pattern, if it is a range pattern.
///
/// Returns `Some((start, end))` for inclusive ranges.
fn extract_int_range(pattern: &Pattern) -> Option<(i64, i64)> {
    match &pattern.kind {
        PatternKind::Range { start, end, inclusive } => {
            match (start, end) {
                (RangeBound::Integer(s), RangeBound::Integer(e)) => {
                    let end_val = if *inclusive { *e } else { e - 1 };
                    Some((*s, end_val))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Check if two ranges overlap.
///
/// Two ranges [a, b] and [c, d] overlap if a <= d && c <= b.
fn ranges_overlap(start1: i64, end1: i64, start2: i64, end2: i64) -> bool {
    start1 <= end2 && start2 <= end1
}

/// Expand or-patterns into a list of alternative patterns.
///
/// For a pattern like `.Red or .Green`, this returns `[.Red, .Green]`.
/// For a non-or-pattern, this returns the pattern itself.
fn expand_or_patterns(pattern: &Pattern) -> Vec<Pattern> {
    match &pattern.kind {
        PatternKind::Or { alternatives } => {
            // Recursively expand nested or-patterns
            alternatives.iter()
                .flat_map(expand_or_patterns)
                .collect()
        }
        _ => vec![pattern.clone()],
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
            .any(|w| matches!(w, Witness::Bool(false))));
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

    #[test]
    fn test_tuple_exhaustive() {
        let tuple_ty = Ty::tuple(vec![bool_ty(), bool_ty()], test_span());

        // (true, _)
        let pat1 = Pattern::tuple(
            vec![
                Pattern::literal(LiteralValue::Bool(true), bool_ty(), test_span()),
                Pattern::wildcard(bool_ty(), test_span()),
            ],
            tuple_ty.clone(),
            test_span(),
        );

        // (false, _)
        let pat2 = Pattern::tuple(
            vec![
                Pattern::literal(LiteralValue::Bool(false), bool_ty(), test_span()),
                Pattern::wildcard(bool_ty(), test_span()),
            ],
            tuple_ty.clone(),
            test_span(),
        );

        let patterns = vec![&pat1, &pat2];
        let result = check_exhaustiveness(&patterns, &tuple_ty);
        assert!(result.is_exhaustive);
    }

    #[test]
    fn test_tuple_non_exhaustive() {
        let tuple_ty = Ty::tuple(vec![bool_ty(), bool_ty()], test_span());

        // (true, _) - only covers true case
        let pat1 = Pattern::tuple(
            vec![
                Pattern::literal(LiteralValue::Bool(true), bool_ty(), test_span()),
                Pattern::wildcard(bool_ty(), test_span()),
            ],
            tuple_ty.clone(),
            test_span(),
        );

        let patterns = vec![&pat1];
        let result = check_exhaustiveness(&patterns, &tuple_ty);
        assert!(!result.is_exhaustive);
    }
}
