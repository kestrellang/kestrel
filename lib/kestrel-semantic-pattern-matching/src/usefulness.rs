//! Usefulness analysis for patterns.
//!
//! A pattern is **useful** with respect to a set of existing patterns if there
//! exists at least one value that:
//! 1. Matches the new pattern
//! 2. Does NOT match any of the existing patterns
//!
//! This is the core algorithm behind both:
//! - **Exhaustiveness checking**: A match is exhaustive if a wildcard is NOT useful
//! - **Redundancy detection**: A pattern is redundant if it's NOT useful
//!
//! # Algorithm
//!
//! This implements a simplified version of Maranget's usefulness algorithm.
//! The key insight is to recursively decompose patterns by their constructors:
//!
//! 1. If the existing patterns are empty, the new pattern is useful (matches everything)
//! 2. If the new pattern is a wildcard, check if any constructor is not fully covered
//! 3. If the new pattern is a specific constructor, check usefulness for that constructor
//!
//! # References
//!
//! - Luc Maranget, "Warnings for pattern matching" (JFP 2007)

use kestrel_semantic_tree::expr::LiteralValue;
use kestrel_semantic_tree::pattern::{Pattern, PatternKind};
use kestrel_semantic_tree::ty::{Ty, TyKind};

/// Check if a pattern is useful given a set of previous patterns.
///
/// Returns `true` if there exists a value that:
/// - Matches `pattern`
/// - Does NOT match any pattern in `previous_patterns`
///
/// # Arguments
///
/// * `pattern` - The pattern to check for usefulness
/// * `previous_patterns` - Patterns that have already been matched against
/// * `ty` - The type of values being matched
///
/// # Returns
///
/// `true` if the pattern is useful (can match something new),
/// `false` if the pattern is redundant (all its matches are covered).
pub fn is_useful(pattern: &Pattern, previous_patterns: &[&Pattern], ty: &Ty) -> bool {
    // Base case: if there are no previous patterns, this pattern is useful
    // (it can match any value)
    if previous_patterns.is_empty() {
        return true;
    }

    // Check if any previous pattern is a "catch-all" (wildcard or binding)
    // If so, this pattern is NOT useful - everything is already covered
    let has_catch_all = previous_patterns.iter().any(|p| is_catch_all(p));
    if has_catch_all {
        return false;
    }

    // Check based on the pattern kind
    match &pattern.kind {
        PatternKind::Wildcard | PatternKind::Local { .. } => {
            // Wildcard/binding is useful if previous patterns don't cover all constructors
            is_wildcard_useful(previous_patterns, ty)
        }

        PatternKind::Literal { value } => {
            // Literal is useful if no previous pattern matches this exact value
            is_literal_useful(value, previous_patterns)
        }

        PatternKind::EnumVariant {
            case_name,
            bindings,
            ..
        } => {
            // Enum variant is useful if either:
            // 1. No previous pattern matches this case
            // 2. This pattern's sub-patterns are useful within the case
            is_enum_variant_useful(case_name, bindings, previous_patterns, ty)
        }

        PatternKind::Tuple { elements } => {
            // Tuple is useful if any element pattern adds new coverage
            is_tuple_useful(elements, previous_patterns, ty)
        }

        PatternKind::Error => {
            // Error patterns are always considered useful to avoid cascading errors
            true
        }
    }
}

/// Check if a pattern is a "catch-all" that matches any value.
fn is_catch_all(pattern: &Pattern) -> bool {
    matches!(
        &pattern.kind,
        PatternKind::Wildcard | PatternKind::Local { .. }
    )
}

/// Check if a wildcard/binding pattern is useful.
///
/// A wildcard is useful if the previous patterns don't cover all possible
/// constructors of the type. For example:
/// - For Bool: need both `true` and `false` to cover
/// - For enum: need all cases
/// - For Int/String: infinite constructors, always need wildcard
fn is_wildcard_useful(previous_patterns: &[&Pattern], ty: &Ty) -> bool {
    use semantic_tree::symbol::Symbol;

    match ty.kind() {
        TyKind::Bool => {
            // Check if both true and false are covered
            let has_true = previous_patterns.iter().any(|p| {
                matches!(
                    &p.kind,
                    PatternKind::Literal {
                        value: LiteralValue::Bool(true)
                    }
                )
            });
            let has_false = previous_patterns.iter().any(|p| {
                matches!(
                    &p.kind,
                    PatternKind::Literal {
                        value: LiteralValue::Bool(false)
                    }
                )
            });
            !(has_true && has_false)
        }

        TyKind::Enum { symbol, .. } => {
            // Get all cases from the enum symbol
            let cases = symbol.cases();

            // Check if all cases are covered
            let covered_cases: Vec<&str> = previous_patterns
                .iter()
                .filter_map(|p| match &p.kind {
                    PatternKind::EnumVariant { case_name, .. } => Some(case_name.as_str()),
                    _ => None,
                })
                .collect();

            // Wildcard is useful if any case is not covered
            cases.iter().any(|case| {
                let name = case.metadata().name();
                let case_name = name.value.as_str();
                !covered_cases.contains(&case_name)
            })
        }

        TyKind::Tuple(elements) => {
            // For tuples, check if the combination of patterns covers all possibilities
            // Simplified: if all elements have catch-all patterns, tuple is covered
            // This is a conservative approximation
            if previous_patterns.is_empty() {
                return true;
            }

            // Check if any previous pattern is a tuple that fully covers
            for prev in previous_patterns {
                if let PatternKind::Tuple {
                    elements: prev_elements,
                } = &prev.kind
                {
                    if prev_elements.len() == elements.len() {
                        // Check if all elements in this tuple pattern are catch-alls
                        let all_catch_alls = prev_elements.iter().all(|e| is_catch_all(e));
                        if all_catch_alls {
                            return false; // This tuple covers everything
                        }
                    }
                }
            }
            true
        }

        // Types with infinite constructors (Int, String, Float)
        // A wildcard is always useful unless there's already a catch-all
        TyKind::Int(_) | TyKind::Float(_) | TyKind::String => {
            // We already checked for catch-alls above, so if we're here,
            // the wildcard can match values not covered by literals
            true
        }

        // Unit type has only one value
        TyKind::Unit => {
            // Unit is covered if there's any pattern (since all patterns match ())
            previous_patterns.is_empty()
        }

        // Never type has no values - patterns are never useful
        TyKind::Never => false,

        // For other types, be conservative and say wildcard is useful
        _ => true,
    }
}

/// Check if a literal pattern is useful.
fn is_literal_useful(value: &LiteralValue, previous_patterns: &[&Pattern]) -> bool {
    // Literal is useful if no previous pattern matches this exact value
    !previous_patterns.iter().any(|p| {
        match &p.kind {
            PatternKind::Literal { value: prev_value } => value == prev_value,
            // Wildcards/bindings would catch this, but we already checked for those
            _ => false,
        }
    })
}

/// Check if an enum variant pattern is useful.
fn is_enum_variant_useful(
    case_name: &str,
    bindings: &[kestrel_semantic_tree::pattern::EnumPatternBinding],
    previous_patterns: &[&Pattern],
    _ty: &Ty,
) -> bool {
    // Collect all previous patterns that match this case
    let matching_patterns: Vec<&[kestrel_semantic_tree::pattern::EnumPatternBinding]> =
        previous_patterns
            .iter()
            .filter_map(|p| match &p.kind {
                PatternKind::EnumVariant {
                    case_name: prev_name,
                    bindings: prev_bindings,
                    ..
                } if prev_name == case_name => Some(prev_bindings.as_slice()),
                _ => None,
            })
            .collect();

    // If no previous pattern matches this case, it's useful
    if matching_patterns.is_empty() {
        return true;
    }

    // If this pattern has no bindings (simple case), check if it's already covered
    if bindings.is_empty() {
        return false; // Already covered by a previous pattern for this case
    }

    // For patterns with bindings, check if the bindings add coverage
    // This is a simplified check - full implementation would recurse into bindings
    for prev_bindings in &matching_patterns {
        if prev_bindings.is_empty() {
            // Previous pattern with no bindings means it catches all instances of this case
            // But wait - if this case HAS associated values, an empty binding list means
            // we're ignoring them, which should still match everything
            return false;
        }

        // Check if all previous bindings are catch-alls
        let all_catch_alls = prev_bindings.iter().all(|b| is_catch_all(&b.pattern));

        if all_catch_alls {
            return false; // Previous pattern catches all instances of this case
        }
    }

    // If we get here, the bindings might add new coverage
    true
}

/// Check if a tuple pattern is useful.
fn is_tuple_useful(elements: &[Pattern], previous_patterns: &[&Pattern], ty: &Ty) -> bool {
    // Get the tuple element types
    let element_types = match ty.kind() {
        TyKind::Tuple(elements) => elements.clone(),
        _ => return true, // Type mismatch - be conservative
    };

    // Collect previous tuple patterns
    let prev_tuples: Vec<&[Pattern]> = previous_patterns
        .iter()
        .filter_map(|p| match &p.kind {
            PatternKind::Tuple {
                elements: prev_elements,
            } => Some(prev_elements.as_slice()),
            _ => None,
        })
        .collect();

    if prev_tuples.is_empty() {
        return true;
    }

    // Check each element position
    for (i, element) in elements.iter().enumerate() {
        // Get previous patterns for this position
        let prev_at_position: Vec<&Pattern> = prev_tuples.iter().filter_map(|t| t.get(i)).collect();

        let element_ty = element_types.get(i).cloned().unwrap_or_else(|| ty.clone());

        // If this element is useful given the previous patterns at this position,
        // then the whole tuple might be useful
        // This is a simplified heuristic - full analysis would be more complex
        if is_useful(element, &prev_at_position, &element_ty) {
            // But we need to also check that the other positions don't exclude this
            // For now, use a conservative approximation
            return true;
        }
    }

    // All elements are covered by previous patterns
    false
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_first_pattern_always_useful() {
        let pattern = Pattern::wildcard(int_ty(), test_span());
        let previous: Vec<&Pattern> = vec![];
        assert!(is_useful(&pattern, &previous, &int_ty()));
    }

    #[test]
    fn test_pattern_after_wildcard_not_useful() {
        let wildcard = Pattern::wildcard(int_ty(), test_span());
        let literal = Pattern::literal(LiteralValue::Integer(42), int_ty(), test_span());
        let previous = vec![&wildcard];
        assert!(!is_useful(&literal, &previous, &int_ty()));
    }

    #[test]
    fn test_different_literals_useful() {
        let lit1 = Pattern::literal(LiteralValue::Integer(1), int_ty(), test_span());
        let lit2 = Pattern::literal(LiteralValue::Integer(2), int_ty(), test_span());
        let previous = vec![&lit1];
        assert!(is_useful(&lit2, &previous, &int_ty()));
    }

    #[test]
    fn test_same_literal_not_useful() {
        let lit1 = Pattern::literal(LiteralValue::Integer(42), int_ty(), test_span());
        let lit2 = Pattern::literal(LiteralValue::Integer(42), int_ty(), test_span());
        let previous = vec![&lit1];
        assert!(!is_useful(&lit2, &previous, &int_ty()));
    }

    #[test]
    fn test_wildcard_after_bool_true_useful() {
        let true_pat = Pattern::literal(LiteralValue::Bool(true), bool_ty(), test_span());
        let wildcard = Pattern::wildcard(bool_ty(), test_span());
        let previous = vec![&true_pat];
        assert!(is_useful(&wildcard, &previous, &bool_ty()));
    }

    #[test]
    fn test_wildcard_after_both_bools_not_useful() {
        let true_pat = Pattern::literal(LiteralValue::Bool(true), bool_ty(), test_span());
        let false_pat = Pattern::literal(LiteralValue::Bool(false), bool_ty(), test_span());
        let wildcard = Pattern::wildcard(bool_ty(), test_span());
        let previous = vec![&true_pat, &false_pat];
        assert!(!is_useful(&wildcard, &previous, &bool_ty()));
    }

    #[test]
    fn test_binding_after_wildcard_not_useful() {
        let wildcard = Pattern::wildcard(int_ty(), test_span());
        let binding = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "x".to_string(),
            int_ty(),
            test_span(),
        );
        let previous = vec![&wildcard];
        assert!(!is_useful(&binding, &previous, &int_ty()));
    }

    #[test]
    fn test_wildcard_useful_for_infinite_types() {
        // For Int with just a few literals, wildcard is always useful
        let lit1 = Pattern::literal(LiteralValue::Integer(1), int_ty(), test_span());
        let lit2 = Pattern::literal(LiteralValue::Integer(2), int_ty(), test_span());
        let wildcard = Pattern::wildcard(int_ty(), test_span());
        let previous = vec![&lit1, &lit2];
        assert!(is_useful(&wildcard, &previous, &int_ty()));
    }

    #[test]
    fn test_tuple_pattern_useful() {
        let tuple_ty = Ty::tuple(vec![int_ty(), int_ty()], test_span());

        let pat1 = Pattern::tuple(
            vec![
                Pattern::literal(LiteralValue::Integer(1), int_ty(), test_span()),
                Pattern::wildcard(int_ty(), test_span()),
            ],
            tuple_ty.clone(),
            test_span(),
        );

        let pat2 = Pattern::tuple(
            vec![
                Pattern::literal(LiteralValue::Integer(2), int_ty(), test_span()),
                Pattern::wildcard(int_ty(), test_span()),
            ],
            tuple_ty.clone(),
            test_span(),
        );

        let previous = vec![&pat1];
        assert!(is_useful(&pat2, &previous, &tuple_ty));
    }
}
