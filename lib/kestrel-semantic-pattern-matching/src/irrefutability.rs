//! Irrefutability analysis for patterns.
//!
//! A pattern is **irrefutable** if it matches all possible values of its type.
//! Irrefutable patterns are required for:
//! - `let` bindings: `let x = expr` (x must match any value)
//! - `var` bindings: `var x = expr`
//! - `for` loops: `for x in iter` (x must match any element)
//!
//! # Irrefutable Patterns
//!
//! - `_` (wildcard) - always matches
//! - `x` (binding) - always matches, binds the value
//! - `(p1, p2, ...)` (tuple) - irrefutable if ALL elements are irrefutable
//!
//! # Refutable Patterns
//!
//! - `42` (literal) - only matches that specific value
//! - `.Some(x)` (enum variant) - only matches that variant (unless single-variant enum)
//!
//! # Example
//!
//! ```ignore
//! // Irrefutable - x always matches
//! let x = getValue()
//!
//! // Irrefutable - tuple elements are wildcards/bindings
//! let (a, b) = getTuple()
//!
//! // REFUTABLE - 42 doesn't match other values
//! let 42 = getValue()  // Error!
//!
//! // REFUTABLE - .Some doesn't match .None
//! let .Some(x) = getOption()  // Error!
//! ```

use kestrel_semantic_tree::pattern::{Pattern, PatternKind};

/// Returns true if the pattern is irrefutable (always matches any value of the appropriate type).
///
/// This is a conservative analysis: if we can't prove a pattern is irrefutable,
/// we return false (treating it as refutable).
///
/// # Arguments
///
/// * `pattern` - The pattern to check
///
/// # Returns
///
/// `true` if the pattern is guaranteed to match any value of its type,
/// `false` if the pattern might fail to match some values.
///
/// # Example
///
/// ```ignore
/// use kestrel_semantic_pattern_matching::is_irrefutable;
///
/// // Wildcard is always irrefutable
/// assert!(is_irrefutable(&wildcard_pattern));
///
/// // Literal is never irrefutable
/// assert!(!is_irrefutable(&literal_pattern));
/// ```
pub fn is_irrefutable(pattern: &Pattern) -> bool {
    match &pattern.kind {
        // Wildcard always matches any value
        PatternKind::Wildcard => true,

        // Local binding always matches (binds any value to a name)
        PatternKind::Local { .. } => true,

        // Tuple is irrefutable if ALL elements (prefix + suffix) are irrefutable
        PatternKind::Tuple { prefix, suffix, .. } => {
            prefix.iter().chain(suffix.iter()).all(is_irrefutable)
        },

        // Literal patterns are REFUTABLE - they only match one specific value
        // e.g., `42` doesn't match `43`
        PatternKind::Literal { .. } => false,

        // Enum variant patterns are REFUTABLE by default - they only match one case
        // e.g., `.Some(x)` doesn't match `.None`
        //
        // NOTE: Single-variant enums would technically be irrefutable, but we're
        // conservative here. This could be enhanced with type information to check
        // if the enum has only one case.
        PatternKind::EnumVariant { .. } => false,

        // Range patterns are REFUTABLE - they only match values within the range
        // e.g., `0..=9` doesn't match 10
        PatternKind::Range { .. } => false,

        // Struct patterns are irrefutable if all field patterns are irrefutable
        PatternKind::Struct { fields, .. } => fields.iter().all(|f| is_irrefutable(&f.pattern)),

        // Array patterns: [..] and [..rest] are irrefutable (capture all elements)
        // But [a, ..] or [.., z] require at least one element, so they're refutable
        PatternKind::Array {
            prefix,
            rest,
            suffix,
        } => {
            // Irrefutable only if: has rest AND no prefix AND no suffix
            // AND all nested patterns (if any) are irrefutable
            if rest.is_some() && prefix.is_empty() && suffix.is_empty() {
                true
            } else {
                false
            }
        },

        // Or-patterns are irrefutable if ANY alternative is irrefutable
        PatternKind::Or { alternatives } => alternatives.iter().any(is_irrefutable),

        // At-patterns are irrefutable if the subpattern is irrefutable
        PatternKind::At { subpattern, .. } => is_irrefutable(subpattern),

        // Rest patterns are always irrefutable (they match any remaining elements)
        PatternKind::Rest => true,

        // Error patterns are treated as irrefutable to avoid cascading errors.
        // If we already have an error in the pattern, don't complain about
        // refutability too.
        PatternKind::Error => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_semantic_tree::expr::LiteralValue;
    use kestrel_semantic_tree::pattern::{EnumPatternBinding, Mutability};
    use kestrel_semantic_tree::symbol::local::LocalId;
    use kestrel_semantic_tree::ty::{IntBits, Ty};
    use kestrel_span::Span;

    fn test_span() -> Span {
        Span::new(0, 0..1)
    }

    fn int_ty() -> Ty {
        Ty::int(IntBits::I64, test_span())
    }

    #[test]
    fn test_wildcard_is_irrefutable() {
        let pattern = Pattern::wildcard(int_ty(), test_span());
        assert!(is_irrefutable(&pattern));
    }

    #[test]
    fn test_binding_is_irrefutable() {
        let pattern = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "x".to_string(),
            int_ty(),
            test_span(),
        );
        assert!(is_irrefutable(&pattern));
    }

    #[test]
    fn test_mutable_binding_is_irrefutable() {
        let pattern = Pattern::local(
            LocalId(0),
            Mutability::Mutable,
            "x".to_string(),
            int_ty(),
            test_span(),
        );
        assert!(is_irrefutable(&pattern));
    }

    #[test]
    fn test_tuple_of_wildcards_is_irrefutable() {
        let elements = vec![
            Pattern::wildcard(int_ty(), test_span()),
            Pattern::wildcard(int_ty(), test_span()),
        ];
        let tuple_ty = Ty::tuple(vec![int_ty(), int_ty()], test_span());
        let pattern = Pattern::tuple(elements, tuple_ty, test_span());
        assert!(is_irrefutable(&pattern));
    }

    #[test]
    fn test_tuple_of_bindings_is_irrefutable() {
        let elements = vec![
            Pattern::local(
                LocalId(0),
                Mutability::Immutable,
                "a".to_string(),
                int_ty(),
                test_span(),
            ),
            Pattern::local(
                LocalId(1),
                Mutability::Immutable,
                "b".to_string(),
                int_ty(),
                test_span(),
            ),
        ];
        let tuple_ty = Ty::tuple(vec![int_ty(), int_ty()], test_span());
        let pattern = Pattern::tuple(elements, tuple_ty, test_span());
        assert!(is_irrefutable(&pattern));
    }

    #[test]
    fn test_tuple_with_literal_is_refutable() {
        let elements = vec![
            Pattern::local(
                LocalId(0),
                Mutability::Immutable,
                "a".to_string(),
                int_ty(),
                test_span(),
            ),
            Pattern::literal(LiteralValue::Integer(42), int_ty(), test_span()),
        ];
        let tuple_ty = Ty::tuple(vec![int_ty(), int_ty()], test_span());
        let pattern = Pattern::tuple(elements, tuple_ty, test_span());
        assert!(!is_irrefutable(&pattern));
    }

    #[test]
    fn test_literal_is_refutable() {
        let pattern = Pattern::literal(LiteralValue::Integer(42), int_ty(), test_span());
        assert!(!is_irrefutable(&pattern));
    }

    #[test]
    fn test_string_literal_is_refutable() {
        let pattern = Pattern::literal(
            LiteralValue::String("hello".to_string()),
            Ty::string(test_span()),
            test_span(),
        );
        assert!(!is_irrefutable(&pattern));
    }

    #[test]
    fn test_bool_literal_is_refutable() {
        let pattern =
            Pattern::literal(LiteralValue::Bool(true), Ty::bool(test_span()), test_span());
        assert!(!is_irrefutable(&pattern));
    }

    #[test]
    fn test_enum_variant_is_refutable() {
        let pattern = Pattern::unresolved_enum_variant("Some".to_string(), vec![], test_span());
        assert!(!is_irrefutable(&pattern));
    }

    #[test]
    fn test_enum_variant_with_bindings_is_refutable() {
        let inner_pattern = Pattern::local(
            LocalId(0),
            Mutability::Immutable,
            "value".to_string(),
            int_ty(),
            test_span(),
        );
        let binding = EnumPatternBinding::unlabeled(inner_pattern, test_span());
        let pattern =
            Pattern::unresolved_enum_variant("Some".to_string(), vec![binding], test_span());
        assert!(!is_irrefutable(&pattern));
    }

    #[test]
    fn test_error_is_irrefutable() {
        // Error patterns are treated as irrefutable to avoid cascading errors
        let pattern = Pattern::error(test_span());
        assert!(is_irrefutable(&pattern));
    }

    #[test]
    fn test_nested_tuple_is_irrefutable() {
        // ((a, b), c) where all are bindings
        let inner_elements = vec![
            Pattern::local(
                LocalId(0),
                Mutability::Immutable,
                "a".to_string(),
                int_ty(),
                test_span(),
            ),
            Pattern::local(
                LocalId(1),
                Mutability::Immutable,
                "b".to_string(),
                int_ty(),
                test_span(),
            ),
        ];
        let inner_tuple_ty = Ty::tuple(vec![int_ty(), int_ty()], test_span());
        let inner_tuple = Pattern::tuple(inner_elements, inner_tuple_ty.clone(), test_span());

        let outer_elements = vec![
            inner_tuple,
            Pattern::local(
                LocalId(2),
                Mutability::Immutable,
                "c".to_string(),
                int_ty(),
                test_span(),
            ),
        ];
        let outer_tuple_ty = Ty::tuple(vec![inner_tuple_ty, int_ty()], test_span());
        let pattern = Pattern::tuple(outer_elements, outer_tuple_ty, test_span());

        assert!(is_irrefutable(&pattern));
    }

    #[test]
    fn test_nested_tuple_with_literal_is_refutable() {
        // ((a, 42), c) - has a literal in nested tuple
        let inner_elements = vec![
            Pattern::local(
                LocalId(0),
                Mutability::Immutable,
                "a".to_string(),
                int_ty(),
                test_span(),
            ),
            Pattern::literal(LiteralValue::Integer(42), int_ty(), test_span()),
        ];
        let inner_tuple_ty = Ty::tuple(vec![int_ty(), int_ty()], test_span());
        let inner_tuple = Pattern::tuple(inner_elements, inner_tuple_ty.clone(), test_span());

        let outer_elements = vec![
            inner_tuple,
            Pattern::local(
                LocalId(1),
                Mutability::Immutable,
                "c".to_string(),
                int_ty(),
                test_span(),
            ),
        ];
        let outer_tuple_ty = Ty::tuple(vec![inner_tuple_ty, int_ty()], test_span());
        let pattern = Pattern::tuple(outer_elements, outer_tuple_ty, test_span());

        assert!(!is_irrefutable(&pattern));
    }

    #[test]
    fn test_empty_tuple_is_irrefutable() {
        // () - unit tuple
        let pattern = Pattern::tuple(vec![], Ty::unit(test_span()), test_span());
        assert!(is_irrefutable(&pattern));
    }
}
