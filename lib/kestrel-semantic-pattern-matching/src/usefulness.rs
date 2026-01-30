//! Usefulness analysis for patterns using Maranget's algorithm.
//!
//! A pattern vector `q` is **useful** with respect to a pattern matrix `P` if there
//! exists at least one value that:
//! 1. Matches `q`
//! 2. Does NOT match any row in `P`
//!
//! This is the core algorithm behind both:
//! - **Exhaustiveness checking**: A match is exhaustive if a wildcard row is NOT useful
//! - **Redundancy detection**: A pattern arm is redundant if it's NOT useful
//!
//! # Maranget's Algorithm
//!
//! The algorithm works by recursively decomposing the pattern matrix:
//!
//! 1. **Base case (empty matrix)**: If `P` is empty, `q` is useful (matches anything)
//! 2. **Base case (unit width)**: If width is 0, check if `P` has any unguarded rows
//! 3. **Constructor case**: If `q[0]` has constructor `c`:
//!    - Specialize `P` and `q` for `c`
//!    - Recursively check usefulness
//! 4. **Wildcard case**: If `q[0]` is a wildcard:
//!    - For each constructor `c` of the type:
//!      - Specialize and recurse
//!    - If type has infinite constructors, use default matrix
//!
//! # References
//!
//! - Luc Maranget, "Warnings for pattern matching" (JFP 2007)
//! - Rust's pattern exhaustiveness checking (`rustc_pattern_analysis`)

use crate::constructor::Constructor;
use crate::matrix::{PatternMatrix, PatternRow};
use crate::witness::Witness;
use kestrel_semantic_tree::pattern::{Pattern, PatternKind};
use kestrel_semantic_tree::symbol::field::FieldSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use std::collections::HashSet;

/// Check if a pattern acts like a catch-all (matches any value).
fn is_catch_all_pattern(pattern: &Pattern) -> bool {
    match &pattern.kind {
        PatternKind::Wildcard | PatternKind::Local { .. } | PatternKind::Rest => true,
        PatternKind::At { subpattern, .. } => is_catch_all_pattern(subpattern),
        PatternKind::Or { alternatives } => alternatives.iter().any(is_catch_all_pattern),
        _ => false,
    }
}

/// Result of usefulness analysis.
#[derive(Debug, Clone)]
pub struct UsefulnessResult {
    /// Whether the pattern is useful
    pub is_useful: bool,
    /// Witness value if useful (for generating error messages)
    pub witness: Option<Witness>,
}

impl UsefulnessResult {
    /// Create a result indicating the pattern is not useful.
    pub fn not_useful() -> Self {
        UsefulnessResult {
            is_useful: false,
            witness: None,
        }
    }

    /// Create a result indicating the pattern is useful.
    pub fn useful(witness: Witness) -> Self {
        UsefulnessResult {
            is_useful: true,
            witness: Some(witness),
        }
    }
}

/// Check if a pattern is useful given a set of previous patterns.
///
/// This is the main entry point for usefulness checking.
/// It's a wrapper around the matrix-based algorithm.
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
    // Build the pattern matrix from previous patterns
    let mut matrix = PatternMatrix::single_column(ty.clone());
    for (i, p) in previous_patterns.iter().enumerate() {
        matrix.push_row(vec![(*p).clone()], i, false);
    }

    // The pattern to check as a single-row matrix
    let query_row = PatternRow::new(vec![pattern.clone()], 0, false);

    // Run the usefulness algorithm
    let result = is_useful_impl(&matrix, &query_row);
    result.is_useful
}

/// Check if a pattern row is useful with respect to a matrix.
///
/// This is the core implementation of Maranget's usefulness algorithm.
pub fn is_useful_impl(matrix: &PatternMatrix, query: &PatternRow) -> UsefulnessResult {
    // Base case 1: If the matrix is empty, the query is useful
    // (it matches values that "fall through" all existing patterns)
    if matrix.is_empty() {
        return UsefulnessResult::useful(Witness::any());
    }

    // Base case 2: If the matrix has no columns (unit width)
    if matrix.is_unit() || query.is_empty() {
        // Check if any existing row (without guard) would catch the value
        let has_unguarded_catch = matrix.rows.iter().any(|row| !row.has_guard);
        return if has_unguarded_catch {
            UsefulnessResult::not_useful()
        } else {
            // All rows have guards, so they might fail
            UsefulnessResult::useful(Witness::any())
        };
    }

    // Get the first column type
    let first_type = match matrix.first_column_type() {
        Some(ty) => ty.clone(),
        None => return UsefulnessResult::useful(Witness::any()),
    };

    // Get the first pattern in the query
    let first_pattern = match query.first() {
        Some(p) => p,
        None => return UsefulnessResult::useful(Witness::any()),
    };

    let query_ctor = Constructor::from_pattern(first_pattern);

    if query_ctor.is_wildcard() {
        // Wildcard case: need to check all constructors
        is_wildcard_useful(matrix, query, &first_type)
    } else {
        // Constructor case: specialize and recurse
        is_constructor_useful(matrix, query, &query_ctor, &first_type)
    }
}

/// Check if a wildcard pattern (at the head) is useful.
fn is_wildcard_useful(matrix: &PatternMatrix, query: &PatternRow, ty: &Ty) -> UsefulnessResult {
    // Check if any row in the matrix starts with a catch-all (wildcard/binding)
    // If so, this wildcard is not useful for the first column
    let has_catch_all = matrix.rows.iter().any(|row| {
        if let Some(first) = row.first() {
            is_catch_all_pattern(first)
        } else {
            false
        }
    });

    if has_catch_all {
        // There's already a wildcard covering the first column
        // We need to check if the query is useful in the default matrix
        let default = matrix.default_matrix();
        let default_query =
            PatternRow::new(query.rest().to_vec(), query.arm_index, query.has_guard);
        return is_useful_impl(&default, &default_query);
    }

    // Get all constructors covered by the matrix's first column
    let covered_ctors: HashSet<Constructor> =
        matrix.unique_head_constructors().into_iter().collect();

    // Get all constructors for the type
    match Constructor::all_constructors(ty) {
        Some(all_ctors) => {
            // Finite constructor set: check each uncovered constructor
            for ctor in &all_ctors {
                if !covered_ctors.contains(ctor) {
                    // Found an uncovered constructor - query is useful!
                    let witness = ctor_to_witness(ctor, ty);
                    return UsefulnessResult::useful(witness);
                }
            }

            // All constructors covered: check if wildcard is useful within each
            for ctor in &all_ctors {
                let result = is_constructor_useful(matrix, query, ctor, ty);
                if result.is_useful {
                    return result;
                }
            }

            UsefulnessResult::not_useful()
        },
        None => {
            // Infinite constructor set: first check if missing_constructors can determine exhaustiveness
            // This handles arrays with rest patterns specially
            if let Some(missing) = Constructor::missing_constructors(ty, &covered_ctors) {
                if missing.is_empty() {
                    // All patterns are covered! Check if we need to recurse for sub-patterns
                    // For each covered constructor, check if wildcard is useful within it
                    for ctor in &covered_ctors {
                        let result = is_constructor_useful(matrix, query, ctor, ty);
                        if result.is_useful {
                            return result;
                        }
                    }
                    return UsefulnessResult::not_useful();
                } else if !missing.contains(&Constructor::NonExhaustive) {
                    // There are specific missing constructors
                    let witness = ctor_to_witness(&missing[0], ty);
                    return UsefulnessResult::useful(witness);
                }
            }

            // Fall back to default matrix approach
            let default = matrix.default_matrix();
            let default_query =
                PatternRow::new(query.rest().to_vec(), query.arm_index, query.has_guard);

            if default.is_unit() && default.is_empty() {
                // No wildcards in matrix and infinite constructors
                // Query is definitely useful
                UsefulnessResult::useful(Witness::any())
            } else {
                is_useful_impl(&default, &default_query)
            }
        },
    }
}

/// Check if a specific constructor is useful.
fn is_constructor_useful(
    matrix: &PatternMatrix,
    query: &PatternRow,
    ctor: &Constructor,
    ty: &Ty,
) -> UsefulnessResult {
    // Get field types for the constructor
    let field_types = get_constructor_field_types(ctor, ty);

    // Specialize matrix and query for this constructor
    let specialized_matrix = matrix.specialize(ctor, &field_types);
    let specialized_query = specialize_query(query, ctor, ty);

    // Recurse
    let result = is_useful_impl(&specialized_matrix, &specialized_query);

    if result.is_useful {
        // Wrap the witness with this constructor
        let inner_witness = result.witness.unwrap_or(Witness::any());
        let wrapped = wrap_witness_with_constructor(inner_witness, ctor, ty);
        UsefulnessResult::useful(wrapped)
    } else {
        UsefulnessResult::not_useful()
    }
}

/// Specialize a query row for a constructor.
fn specialize_query(query: &PatternRow, ctor: &Constructor, ty: &Ty) -> PatternRow {
    let first = query
        .first()
        .expect("Query should have at least one pattern");
    let field_types = get_constructor_field_types(ctor, ty);

    // Extract sub-patterns from the first pattern
    let sub_patterns: Vec<Pattern> = match &first.kind {
        kestrel_semantic_tree::pattern::PatternKind::Wildcard
        | kestrel_semantic_tree::pattern::PatternKind::Local { .. }
        | kestrel_semantic_tree::pattern::PatternKind::Rest => {
            // Generate wildcards for constructor's fields
            field_types
                .iter()
                .map(|field_ty| Pattern::wildcard(field_ty.clone(), first.span.clone()))
                .collect()
        },

        kestrel_semantic_tree::pattern::PatternKind::Tuple {
            prefix,
            has_rest,
            suffix,
        } => {
            // For tuple patterns with rest, expand to full tuple arity
            if *has_rest {
                if let TyKind::Tuple(elem_tys) = first.ty.kind() {
                    let rest_count = elem_tys.len().saturating_sub(prefix.len() + suffix.len());
                    let mut result = prefix.clone();
                    for i in 0..rest_count {
                        let ty_idx = prefix.len() + i;
                        let ty = elem_tys.get(ty_idx).cloned().unwrap_or(first.ty.clone());
                        result.push(Pattern::wildcard(ty, first.span.clone()));
                    }
                    result.extend(suffix.clone());
                    result
                } else {
                    prefix.iter().chain(suffix.iter()).cloned().collect()
                }
            } else {
                prefix.clone()
            }
        },

        kestrel_semantic_tree::pattern::PatternKind::EnumVariant { bindings, .. } => {
            bindings.iter().map(|b| (*b.pattern).clone()).collect()
        },

        kestrel_semantic_tree::pattern::PatternKind::Struct { fields, .. } => {
            // For struct patterns, we need to return patterns for ALL fields
            // in the order they appear in the struct, not just the ones matched
            use kestrel_semantic_tree::behavior::typed::TypedBehavior;
            use semantic_tree::symbol::Symbol;

            if let TyKind::Struct {
                symbol,
                substitutions,
            } = first.ty.kind()
            {
                // Get all field names from the struct in order
                let struct_fields: Vec<_> = symbol
                    .metadata()
                    .children()
                    .into_iter()
                    .filter(|c| c.metadata().kind() == KestrelSymbolKind::Field)
                    .filter_map(|c| c.downcast_arc::<FieldSymbol>().ok())
                    .collect();

                // Build the result by matching pattern fields to struct fields
                let mut result = Vec::with_capacity(struct_fields.len());
                for struct_field in &struct_fields {
                    let field_name = &struct_field.metadata().name().value;
                    // Find the pattern field for this struct field
                    let matched_field = fields.iter().find(|f| &f.field_name == field_name);

                    if let Some(pf) = matched_field {
                        result.push(pf.pattern.clone());
                    } else {
                        // Field not matched in pattern - use a wildcard
                        // Get the field type for the wildcard
                        let raw_field_ty = struct_field
                            .metadata()
                            .get_behavior::<TypedBehavior>()
                            .map(|typed| typed.ty().clone())
                            .unwrap_or_else(|| struct_field.field_type().clone());
                        let field_ty = raw_field_ty.apply_substitutions(substitutions);
                        result.push(Pattern::wildcard(field_ty, first.span.clone()));
                    }
                }
                result
            } else {
                // Fallback: just return the fields from the pattern
                fields.iter().map(|f| f.pattern.clone()).collect()
            }
        },

        kestrel_semantic_tree::pattern::PatternKind::Array {
            prefix,
            rest,
            suffix,
        } => {
            // Array pattern specialization - same logic as extract_pattern_children
            if let Constructor::Array {
                prefix_len: target_prefix,
                suffix_len: target_suffix,
                has_rest: target_has_rest,
            } = ctor
            {
                let target_arity =
                    target_prefix + target_suffix + if *target_has_rest { 1 } else { 0 };

                // Get element type from Array[T] struct
                let elem_ty = match first.ty.kind() {
                    TyKind::Struct { substitutions, .. } => {
                        // Array[T] - get T from substitutions
                        substitutions
                            .iter()
                            .next()
                            .map(|(_, t)| t.clone())
                            .unwrap_or_else(|| first.ty.clone())
                    },
                    _ => first.ty.clone(),
                };

                if rest.is_some() && !target_has_rest {
                    // Pattern has rest, target doesn't - expand rest to wildcards
                    let mut result = Vec::with_capacity(target_arity);

                    for i in 0..*target_prefix {
                        if i < prefix.len() {
                            result.push(prefix[i].clone());
                        } else {
                            result.push(Pattern::wildcard(elem_ty.clone(), first.span.clone()));
                        }
                    }

                    for i in 0..*target_suffix {
                        let suffix_idx = suffix.len().saturating_sub(*target_suffix - i);
                        if suffix_idx < suffix.len() {
                            result.push(suffix[suffix_idx].clone());
                        } else {
                            result.push(Pattern::wildcard(elem_ty.clone(), first.span.clone()));
                        }
                    }

                    result
                } else if rest.is_none() && *target_has_rest {
                    // Pattern doesn't have rest, target does - compress to target arity
                    let mut result = Vec::with_capacity(target_arity);

                    for i in 0..*target_prefix {
                        if i < prefix.len() {
                            result.push(prefix[i].clone());
                        } else {
                            result.push(Pattern::wildcard(elem_ty.clone(), first.span.clone()));
                        }
                    }

                    // Add a wildcard for the rest slot
                    result.push(Pattern::wildcard(first.ty.clone(), first.span.clone()));

                    for i in 0..*target_suffix {
                        let suffix_idx = suffix.len().saturating_sub(*target_suffix - i);
                        if suffix_idx < suffix.len() {
                            result.push(suffix[suffix_idx].clone());
                        } else {
                            result.push(Pattern::wildcard(elem_ty.clone(), first.span.clone()));
                        }
                    }

                    result
                } else if rest.is_some() && *target_has_rest {
                    // Both have rest - map prefix, rest, suffix
                    let mut result = Vec::with_capacity(target_arity);

                    for i in 0..*target_prefix {
                        if i < prefix.len() {
                            result.push(prefix[i].clone());
                        } else {
                            result.push(Pattern::wildcard(elem_ty.clone(), first.span.clone()));
                        }
                    }

                    result.push(Pattern::wildcard(first.ty.clone(), first.span.clone()));

                    for i in 0..*target_suffix {
                        let suffix_idx = suffix.len().saturating_sub(*target_suffix - i);
                        if suffix_idx < suffix.len() {
                            result.push(suffix[suffix_idx].clone());
                        } else {
                            result.push(Pattern::wildcard(elem_ty.clone(), first.span.clone()));
                        }
                    }

                    result
                } else {
                    // Neither has rest - direct mapping
                    let mut result = prefix.clone();
                    result.extend(suffix.clone());
                    result
                }
            } else {
                // Fallback if not an array constructor
                let mut result = prefix.clone();
                if rest.is_some() {
                    result.push(Pattern::wildcard(first.ty.clone(), first.span.clone()));
                }
                result.extend(suffix.clone());
                result
            }
        },

        kestrel_semantic_tree::pattern::PatternKind::At { subpattern, .. } => {
            // Recurse into the subpattern
            let sub_query = PatternRow::new(
                vec![(**subpattern).clone()],
                query.arm_index,
                query.has_guard,
            );
            return specialize_query(&sub_query, ctor, ty);
        },

        kestrel_semantic_tree::pattern::PatternKind::Or { alternatives } => {
            // Use first matching alternative
            // TODO: This is a simplification; proper handling would try all
            if let Some(first_alt) = alternatives.first() {
                let alt_query =
                    PatternRow::new(vec![first_alt.clone()], query.arm_index, query.has_guard);
                return specialize_query(&alt_query, ctor, ty);
            }
            vec![]
        },

        kestrel_semantic_tree::pattern::PatternKind::Literal { .. }
        | kestrel_semantic_tree::pattern::PatternKind::Range { .. } => {
            // No sub-patterns for literals and ranges
            vec![]
        },

        kestrel_semantic_tree::pattern::PatternKind::Error => {
            vec![]
        },
    };

    // Combine sub-patterns with rest of query
    let mut new_patterns = sub_patterns;
    new_patterns.extend(query.rest().iter().cloned());

    PatternRow::new(new_patterns, query.arm_index, query.has_guard)
}

/// Get field types for a constructor.
fn get_constructor_field_types(ctor: &Constructor, ty: &Ty) -> Vec<Ty> {
    use semantic_tree::symbol::Symbol;

    match (ctor, ty.kind()) {
        (Constructor::Tuple { arity }, TyKind::Tuple(elements)) => {
            if elements.len() == *arity {
                elements.clone()
            } else {
                vec![ty.clone(); *arity]
            }
        },

        (
            Constructor::Variant { name, arity },
            TyKind::Enum {
                symbol,
                substitutions,
            },
        ) => {
            if let Some(case) = symbol
                .cases()
                .iter()
                .find(|c| c.metadata().name().value == *name)
                && let Some(cb) = case.callable_behavior()
            {
                // Apply type substitutions to get concrete types
                return cb
                    .parameters()
                    .iter()
                    .map(|p| substitutions.apply(&p.ty))
                    .collect();
            }
            vec![ty.clone(); *arity]
        },

        (
            Constructor::Struct { arity, .. },
            TyKind::Struct {
                symbol,
                substitutions,
            },
        ) => {
            use kestrel_semantic_tree::behavior::typed::TypedBehavior;

            // Get Field children and their types
            let fields: Vec<_> = symbol
                .metadata()
                .children()
                .iter()
                .filter_map(|c| {
                    if c.metadata().kind() == KestrelSymbolKind::Field {
                        c.clone().downcast_arc::<FieldSymbol>().ok()
                    } else {
                        None
                    }
                })
                .collect();

            if fields.len() == *arity {
                // Get the resolved type from TypedBehavior, falling back to field_type
                // Then apply substitutions for generic structs
                fields
                    .iter()
                    .map(|f| {
                        let raw_field_ty = f
                            .metadata()
                            .get_behavior::<TypedBehavior>()
                            .map(|typed| typed.ty().clone())
                            .unwrap_or_else(|| f.field_type().clone());
                        raw_field_ty.apply_substitutions(substitutions)
                    })
                    .collect()
            } else {
                vec![ty.clone(); *arity]
            }
        },

        // Array[T] struct type
        (
            Constructor::Array {
                prefix_len,
                suffix_len,
                has_rest,
            },
            TyKind::Struct { substitutions, .. },
        ) => {
            let elem_ty = substitutions
                .iter()
                .next()
                .map(|(_, t)| t.clone())
                .unwrap_or_else(|| ty.clone());
            let mut types = vec![elem_ty.clone(); *prefix_len];
            if *has_rest {
                types.push(ty.clone()); // Rest is an array/slice
            }
            types.extend(vec![elem_ty; *suffix_len]);
            types
        },

        _ => vec![ty.clone(); ctor.arity()],
    }
}

/// Convert a constructor to a witness.
fn ctor_to_witness(ctor: &Constructor, _ty: &Ty) -> Witness {
    match ctor {
        Constructor::True => Witness::bool(true),
        Constructor::False => Witness::bool(false),
        Constructor::Variant { name, arity } => {
            if *arity == 0 {
                Witness::enum_case(name)
            } else {
                let args = vec![Witness::any(); *arity];
                Witness::enum_case_with_args(name, args)
            }
        },
        Constructor::Tuple { arity } => Witness::tuple(vec![Witness::any(); *arity]),
        Constructor::Struct { name, .. } => Witness::EnumCase {
            name: name.clone(),
            args: vec![],
        },
        Constructor::IntLiteral(n) => Witness::integer(*n),
        Constructor::IntRange { start, end } => {
            // For ranges, pick the start value as witness (or 0 if unbounded)
            match start {
                Some(s) => Witness::integer(*s),
                None => match end {
                    Some(e) => Witness::integer(*e),
                    None => Witness::any(),
                },
            }
        },
        Constructor::CharLiteral(c) => Witness::Literal(format!("'{}'", c)),
        Constructor::CharRange { start, end } => {
            match start {
                Some(s) => Witness::Literal(format!("'{}'", s)),
                None => match end {
                    Some(e) => Witness::Literal(format!("'{}'", e)),
                    None => Witness::any(),
                },
            }
        },
        Constructor::StringLiteral(s) => Witness::string(s),
        Constructor::Unit => Witness::tuple(vec![]),
        Constructor::Wildcard => Witness::any(),
        Constructor::Array {
            prefix_len,
            suffix_len,
            has_rest,
        } => {
            let mut elements = vec![Witness::any(); *prefix_len + *suffix_len];
            if *has_rest {
                elements.push(Witness::any()); // Simplified
            }
            Witness::Array(elements)
        },
        Constructor::NonExhaustive | Constructor::Missing => Witness::any(),
    }
}

/// Wrap a witness with a constructor.
fn wrap_witness_with_constructor(inner: Witness, ctor: &Constructor, _ty: &Ty) -> Witness {
    match ctor {
        Constructor::True => Witness::bool(true),
        Constructor::False => Witness::bool(false),
        Constructor::Variant { name, .. } => {
            // Extract inner witnesses from tuple-like witness
            let args = match inner {
                Witness::Tuple(elems) => elems,
                Witness::Any => vec![Witness::any()],
                other => vec![other],
            };
            if args.is_empty() || (args.len() == 1 && matches!(args[0], Witness::Any)) {
                Witness::enum_case(name)
            } else {
                Witness::enum_case_with_args(name, args)
            }
        },
        Constructor::Tuple { .. } => {
            // Inner should already be the right shape
            match inner {
                Witness::Tuple(_) => inner,
                Witness::Any => Witness::any(),
                other => Witness::tuple(vec![other]),
            }
        },
        Constructor::Struct { name, .. } => Witness::EnumCase {
            name: format!("{} {{ .. }}", name),
            args: vec![],
        },
        _ => inner,
    }
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
        Span::new(0, 0..1)
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

    #[test]
    fn test_tuple_wildcard_after_partial_coverage() {
        let tuple_ty = Ty::tuple(vec![bool_ty(), bool_ty()], test_span());

        // Pattern: (true, _)
        let pat1 = Pattern::tuple(
            vec![
                Pattern::literal(LiteralValue::Bool(true), bool_ty(), test_span()),
                Pattern::wildcard(bool_ty(), test_span()),
            ],
            tuple_ty.clone(),
            test_span(),
        );

        // Wildcard should be useful because (false, _) is uncovered
        let wildcard = Pattern::wildcard(tuple_ty.clone(), test_span());
        let previous = vec![&pat1];
        assert!(is_useful(&wildcard, &previous, &tuple_ty));
    }

    #[test]
    fn test_tuple_fully_covered() {
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

        // Wildcard should NOT be useful - both bool cases covered
        let wildcard = Pattern::wildcard(tuple_ty.clone(), test_span());
        let previous = vec![&pat1, &pat2];
        assert!(!is_useful(&wildcard, &previous, &tuple_ty));
    }
}
