//! Constructor representation for pattern matching.
//!
//! A "constructor" in the pattern matching sense is a way to build a value
//! of a type. For exhaustiveness checking, we need to know all constructors
//! of a type to determine if patterns cover all cases.
//!
//! # Constructors for Different Types
//!
//! - **Bool**: Two constructors: `True`, `False`
//! - **Enum**: One constructor per case
//! - **Tuple**: Single constructor with N fields
//! - **Struct**: Single constructor with named fields
//! - **Int/String/Float**: Infinite constructors (literals), or `NonExhaustive` marker
//! - **Unit**: Single constructor (no fields)
//! - **Never**: Zero constructors (uninhabited)

use kestrel_semantic_tree::expr::LiteralValue;
use kestrel_semantic_tree::pattern::{Pattern, PatternKind, RangeBound};
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::symbol::Symbol;
use std::collections::HashSet;

/// A constructor in the pattern matching sense.
///
/// Constructors are the "heads" of patterns. Pattern matching decomposes
/// patterns by constructor to check coverage.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constructor {
    /// Boolean true
    True,
    /// Boolean false
    False,

    /// Enum case with name and arity (number of associated values)
    Variant { name: String, arity: usize },

    /// Tuple with given arity
    Tuple { arity: usize },

    /// Struct with given number of fields
    Struct { name: String, arity: usize },

    /// Integer literal
    IntLiteral(i64),

    /// Integer range (inclusive on both ends after normalization)
    IntRange { start: i64, end: i64 },

    /// Character literal
    CharLiteral(char),

    /// Character range (inclusive on both ends after normalization)
    CharRange { start: char, end: char },

    /// String literal
    StringLiteral(String),

    /// Unit value ()
    Unit,

    /// Wildcard - matches anything, used as placeholder
    Wildcard,

    /// Array with specific prefix/suffix lengths and optional rest
    Array {
        /// Number of fixed elements at start
        prefix_len: usize,
        /// Number of fixed elements at end
        suffix_len: usize,
        /// Whether there's a rest pattern
        has_rest: bool,
    },

    /// Marker for types with infinite constructors where we didn't see all
    /// Used to indicate that even if we've covered some literals, there are always more
    NonExhaustive,

    /// Missing constructor - used in witness generation to represent
    /// a constructor that wasn't covered
    Missing,
}

impl Constructor {
    /// Get the arity (number of sub-patterns) of this constructor.
    pub fn arity(&self) -> usize {
        match self {
            Constructor::True | Constructor::False => 0,
            Constructor::Variant { arity, .. } => *arity,
            Constructor::Tuple { arity } => *arity,
            Constructor::Struct { arity, .. } => *arity,
            Constructor::IntLiteral(_) | Constructor::IntRange { .. } => 0,
            Constructor::CharLiteral(_) | Constructor::CharRange { .. } => 0,
            Constructor::StringLiteral(_) => 0,
            Constructor::Unit => 0,
            Constructor::Wildcard => 0,
            Constructor::Array {
                prefix_len,
                suffix_len,
                has_rest,
            } => {
                // Array constructor arity is prefix + suffix (rest is handled separately)
                prefix_len + suffix_len + if *has_rest { 1 } else { 0 }
            },
            Constructor::NonExhaustive => 0,
            Constructor::Missing => 0,
        }
    }

    /// Check if this constructor is a wildcard.
    pub fn is_wildcard(&self) -> bool {
        matches!(self, Constructor::Wildcard)
    }

    /// Create a constructor from a pattern.
    ///
    /// Returns `Wildcard` for patterns that match anything (wildcards, bindings).
    pub fn from_pattern(pattern: &Pattern) -> Self {
        match &pattern.kind {
            PatternKind::Wildcard | PatternKind::Local { .. } | PatternKind::Rest => {
                Constructor::Wildcard
            },

            PatternKind::Literal { value } => match value {
                LiteralValue::Bool(true) => Constructor::True,
                LiteralValue::Bool(false) => Constructor::False,
                LiteralValue::Integer(n) => Constructor::IntLiteral(*n),
                LiteralValue::Char(c) => {
                    Constructor::CharLiteral(char::from_u32(*c).unwrap_or('\0'))
                },
                LiteralValue::String(s) => Constructor::StringLiteral(s.clone()),
                LiteralValue::Float(_) => Constructor::NonExhaustive, // Floats can't be exhaustively matched
                LiteralValue::Unit => Constructor::Unit,
                LiteralValue::Null => Constructor::NonExhaustive, // Null literals handled via Optional pattern matching
            },

            PatternKind::EnumVariant {
                case_name,
                bindings,
                ..
            } => Constructor::Variant {
                name: case_name.clone(),
                arity: bindings.len(),
            },

            PatternKind::Tuple {
                prefix,
                has_rest,
                suffix,
            } => {
                // For tuple patterns with rest, get arity from the type
                let arity = if *has_rest {
                    match pattern.ty.kind() {
                        TyKind::Tuple(elems) => elems.len(),
                        _ => prefix.len() + suffix.len(), // Fallback
                    }
                } else {
                    prefix.len() // No rest, suffix should be empty
                };
                Constructor::Tuple { arity }
            },

            PatternKind::Struct {
                struct_name,
                fields,
                has_rest: _,
                ..
            } => {
                // For struct patterns, we need to get the actual field count from the type
                // The pattern may only match some fields (with `..` for rest)
                // We use the pattern's type to get the full field count for proper matching
                let full_arity = match pattern.ty.kind() {
                    TyKind::Struct { symbol, .. } => symbol
                        .metadata()
                        .children()
                        .iter()
                        .filter(|c| c.metadata().kind() == KestrelSymbolKind::Field)
                        .count(),
                    _ => fields.len(), // Fallback if type isn't resolved
                };
                Constructor::Struct {
                    name: struct_name.clone(),
                    arity: full_arity,
                }
            },

            PatternKind::Range {
                start,
                end,
                inclusive,
            } => match (start, end) {
                (RangeBound::Integer(s), RangeBound::Integer(e)) => {
                    let end_val = if *inclusive { *e } else { e - 1 };
                    Constructor::IntRange {
                        start: *s,
                        end: end_val,
                    }
                },
                (RangeBound::Char(s), RangeBound::Char(e)) => {
                    let end_val = if *inclusive {
                        *e
                    } else {
                        char::from_u32(*e as u32 - 1).unwrap_or(*e)
                    };
                    Constructor::CharRange {
                        start: *s,
                        end: end_val,
                    }
                },
                // Mismatched range bounds - treat as non-exhaustive
                _ => Constructor::NonExhaustive,
            },

            PatternKind::Array {
                prefix,
                rest,
                suffix,
            } => Constructor::Array {
                prefix_len: prefix.len(),
                suffix_len: suffix.len(),
                has_rest: rest.is_some(),
            },

            PatternKind::Or { alternatives } => {
                // For or-patterns, we need to handle this specially in the algorithm
                // For now, return wildcard as a conservative choice
                // The actual handling happens in pattern expansion
                if let Some(first) = alternatives.first() {
                    Constructor::from_pattern(first)
                } else {
                    Constructor::Wildcard
                }
            },

            PatternKind::At { subpattern, .. } => {
                // @-pattern: the constructor is determined by the subpattern
                Constructor::from_pattern(subpattern)
            },

            PatternKind::Error => Constructor::Wildcard,
        }
    }

    /// Get all constructors for a type.
    ///
    /// Returns `None` if the type has infinitely many constructors
    /// (Int, String, Float, arrays with variable length).
    pub fn all_constructors(ty: &Ty) -> Option<Vec<Constructor>> {
        // Treat type aliases as transparent for pattern matching.
        let ty = ty.expand_aliases();

        match ty.kind() {
            TyKind::Bool => Some(vec![Constructor::True, Constructor::False]),

            TyKind::Unit => Some(vec![Constructor::Unit]),

            TyKind::Never => Some(vec![]), // No constructors - always exhaustive!

            TyKind::Enum { symbol, .. } => {
                let cases = symbol.cases();
                Some(
                    cases
                        .iter()
                        .map(|case| {
                            let arity = case
                                .callable_behavior()
                                .map(|cb| cb.parameters().len())
                                .unwrap_or(0);
                            Constructor::Variant {
                                name: case.metadata().name().value.clone(),
                                arity,
                            }
                        })
                        .collect(),
                )
            },

            TyKind::Tuple(elements) => Some(vec![Constructor::Tuple {
                arity: elements.len(),
            }]),

            TyKind::Struct { symbol, .. } => {
                // Check for Array[T] struct - arrays have variable length
                if symbol.metadata().name().value == "Array"
                    && symbol.type_parameters().len() == 1
                {
                    return None;
                }

                // Check if this struct conforms to ExpressibleByBoolLiteral
                // If so, use True/False constructors for exhaustiveness checking
                use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
                if let Some(conformances) = symbol.metadata().get_behavior::<ConformancesBehavior>()
                {
                    for conf in conformances.conformances() {
                        if let TyKind::Protocol { symbol: proto, .. } = conf.kind()
                            && proto.metadata().name().value == "ExpressibleByBoolLiteral"
                        {
                            return Some(vec![Constructor::True, Constructor::False]);
                        }
                    }
                }

                // Count Field children
                let field_count = symbol
                    .metadata()
                    .children()
                    .iter()
                    .filter(|c| c.metadata().kind() == KestrelSymbolKind::Field)
                    .count();
                Some(vec![Constructor::Struct {
                    name: symbol.metadata().name().value.clone(),
                    arity: field_count,
                }])
            },

            // Infinite constructor spaces
            TyKind::Int(_) | TyKind::Float(_) | TyKind::String => None,

            // Unknown/error types - be conservative
            _ => None,
        }
    }

    /// Check if this constructor covers all values of the type.
    ///
    /// Returns true for wildcards and for single-constructor types.
    pub fn is_exhaustive_alone(&self, ty: &Ty) -> bool {
        match self {
            Constructor::Wildcard => true,
            _ => {
                if let Some(all) = Self::all_constructors(ty) {
                    all.len() == 1 && all.contains(self)
                } else {
                    false
                }
            },
        }
    }

    /// Get missing constructors given a set of covered constructors.
    ///
    /// Returns `None` if the type has infinite constructors and no wildcard
    /// covers them all.
    pub fn missing_constructors(
        ty: &Ty,
        covered: &HashSet<Constructor>,
    ) -> Option<Vec<Constructor>> {
        // If there's a wildcard in covered, everything is covered
        if covered.contains(&Constructor::Wildcard) {
            return Some(vec![]);
        }

        match Self::all_constructors(ty) {
            Some(all) => {
                let missing: Vec<_> = all.into_iter().filter(|c| !covered.contains(c)).collect();
                Some(missing)
            },
            None => {
                // Infinite constructors case
                // For arrays (both TyKind::Array and Array[T] struct), check if patterns cover all possible lengths
                let is_array = match ty.kind() {
                    TyKind::Struct { symbol, .. } => {
                        symbol.metadata().name().value == "Array"
                            && symbol.type_parameters().len() == 1
                    },
                    _ => false,
                };

                if is_array {
                    return Self::missing_array_constructors(covered);
                }

                // Other infinite constructor types need a wildcard to be exhaustive
                // Return NonExhaustive marker to indicate uncovered values exist
                Some(vec![Constructor::NonExhaustive])
            },
        }
    }

    /// Check if array patterns cover all possible lengths.
    ///
    /// Array patterns can have:
    /// - Fixed length: `[a, b]` matches only length 2
    /// - Rest at end: `[a, ..]` matches length >= 1
    /// - Rest at beginning: `[.., a]` matches length >= 1
    /// - Rest in middle: `[a, .., b]` matches length >= 2
    ///
    /// To be exhaustive, we need to cover all lengths from 0 to infinity.
    fn missing_array_constructors(covered: &HashSet<Constructor>) -> Option<Vec<Constructor>> {
        // Collect information about covered lengths
        let mut has_rest_pattern = false;
        let mut min_len_for_rest = usize::MAX;
        let mut fixed_lengths: HashSet<usize> = HashSet::new();

        for ctor in covered {
            if let Constructor::Array {
                prefix_len,
                suffix_len,
                has_rest,
            } = ctor
            {
                let min_len = prefix_len + suffix_len;
                if *has_rest {
                    has_rest_pattern = true;
                    min_len_for_rest = min_len_for_rest.min(min_len);
                } else {
                    fixed_lengths.insert(min_len);
                }
            }
        }

        // If we have a rest pattern with min_len N, it covers all lengths >= N
        // We need fixed patterns to cover lengths 0, 1, ..., N-1
        if has_rest_pattern {
            let mut missing = Vec::new();
            for len in 0..min_len_for_rest {
                if !fixed_lengths.contains(&len) {
                    // Missing a pattern for this length
                    // Return a representative constructor
                    missing.push(Constructor::Array {
                        prefix_len: len,
                        suffix_len: 0,
                        has_rest: false,
                    });
                }
            }
            Some(missing)
        } else {
            // No rest pattern - we can't cover infinite lengths
            Some(vec![Constructor::NonExhaustive])
        }
    }

    /// Get display name for error messages.
    pub fn display_name(&self) -> String {
        match self {
            Constructor::True => "true".to_string(),
            Constructor::False => "false".to_string(),
            Constructor::Variant { name, arity } => {
                if *arity == 0 {
                    format!(".{}", name)
                } else {
                    format!(".{}(_)", name)
                }
            },
            Constructor::Tuple { arity } => {
                let wildcards = vec!["_"; *arity].join(", ");
                format!("({})", wildcards)
            },
            Constructor::Struct { name, .. } => format!("{} {{ .. }}", name),
            Constructor::IntLiteral(n) => n.to_string(),
            Constructor::IntRange { start, end } => format!("{}..={}", start, end),
            Constructor::CharLiteral(c) => format!("'{}'", c),
            Constructor::CharRange { start, end } => format!("'{}'..='{}'", start, end),
            Constructor::StringLiteral(s) => format!("\"{}\"", s),
            Constructor::Unit => "()".to_string(),
            Constructor::Wildcard => "_".to_string(),
            Constructor::Array {
                prefix_len,
                suffix_len,
                has_rest,
            } => {
                let mut parts = vec!["_"; *prefix_len];
                if *has_rest {
                    parts.push("..");
                }
                parts.extend(vec!["_"; *suffix_len]);
                format!("[{}]", parts.join(", "))
            },
            Constructor::NonExhaustive => "_".to_string(),
            Constructor::Missing => "_".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_semantic_tree::ty::IntBits;
    use kestrel_span::Span;

    fn test_span() -> Span {
        Span::new(0, 0..1)
    }

    #[test]
    fn test_bool_constructors() {
        let bool_ty = Ty::bool(test_span());
        let all = Constructor::all_constructors(&bool_ty).unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&Constructor::True));
        assert!(all.contains(&Constructor::False));
    }

    #[test]
    fn test_unit_constructors() {
        let unit_ty = Ty::unit(test_span());
        let all = Constructor::all_constructors(&unit_ty).unwrap();
        assert_eq!(all.len(), 1);
        assert!(all.contains(&Constructor::Unit));
    }

    #[test]
    fn test_int_infinite_constructors() {
        let int_ty = Ty::int(IntBits::I64, test_span());
        assert!(Constructor::all_constructors(&int_ty).is_none());
    }

    #[test]
    fn test_tuple_constructor() {
        let tuple_ty = Ty::tuple(
            vec![Ty::int(IntBits::I64, test_span()), Ty::bool(test_span())],
            test_span(),
        );
        let all = Constructor::all_constructors(&tuple_ty).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0], Constructor::Tuple { arity: 2 });
    }

    #[test]
    fn test_constructor_from_literal_pattern() {
        let pattern = Pattern::literal(
            LiteralValue::Integer(42),
            Ty::int(IntBits::I64, test_span()),
            test_span(),
        );
        assert_eq!(
            Constructor::from_pattern(&pattern),
            Constructor::IntLiteral(42)
        );
    }

    #[test]
    fn test_constructor_from_wildcard_pattern() {
        let pattern = Pattern::wildcard(Ty::int(IntBits::I64, test_span()), test_span());
        assert_eq!(Constructor::from_pattern(&pattern), Constructor::Wildcard);
    }

    #[test]
    fn test_missing_constructors_bool() {
        let bool_ty = Ty::bool(test_span());
        let mut covered = HashSet::new();
        covered.insert(Constructor::True);

        let missing = Constructor::missing_constructors(&bool_ty, &covered).unwrap();
        assert_eq!(missing.len(), 1);
        assert!(missing.contains(&Constructor::False));
    }

    #[test]
    fn test_missing_constructors_with_wildcard() {
        let bool_ty = Ty::bool(test_span());
        let mut covered = HashSet::new();
        covered.insert(Constructor::Wildcard);

        let missing = Constructor::missing_constructors(&bool_ty, &covered).unwrap();
        assert!(missing.is_empty());
    }
}
