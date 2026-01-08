//! Witness generation for pattern matching errors.
//!
//! A **witness** is an example value that demonstrates why a pattern analysis failed.
//! Witnesses are used to generate helpful error messages:
//!
//! - Non-exhaustive match: "missing pattern: `.None`"
//! - Refutable let binding: "pattern `.Some(x)` doesn't cover `.None`"
//!
//! # Example Witnesses
//!
//! For a non-exhaustive match on `Option[Int]`:
//! ```text
//! match opt {
//!     .Some(x) => x
//! }
//! // Error: non-exhaustive match, missing pattern: `.None`
//! ```
//!
//! For a refutable let binding:
//! ```text
//! let .Some(x) = getValue()
//! // Error: refutable pattern, doesn't cover: `.None`
//! ```

use std::fmt;

/// A witness value demonstrating a gap in pattern coverage.
///
/// Witnesses are constructed during exhaustiveness/usefulness analysis
/// to provide helpful error messages showing uncovered cases.
#[derive(Debug, Clone, PartialEq)]
pub enum Witness {
    /// Wildcard - represents "any value of this type"
    /// Displayed as `_` in error messages
    Any,

    /// A specific enum case (e.g., `.None`, `.Some(_)`)
    EnumCase {
        /// The case name (e.g., "None", "Some")
        name: String,
        /// Sub-witnesses for associated values (empty if no associated values)
        args: Vec<Witness>,
    },

    /// A tuple witness (e.g., `(_, 42)`)
    Tuple(Vec<Witness>),

    /// A specific literal value (e.g., `42`, `"hello"`, `true`)
    Literal(String),

    /// Boolean value
    Bool(bool),

    /// An array witness (e.g., `[_, 42, ..]`)
    Array(Vec<Witness>),

    /// A struct witness (e.g., `Point { x: _, y: 0 }`)
    Struct {
        /// The struct name
        name: String,
        /// Field witnesses (may be partial with `..`)
        fields: Vec<(String, Witness)>,
    },

    /// A range witness (e.g., `0..=9`)
    Range {
        /// Start of range
        start: String,
        /// End of range
        end: String,
        /// Whether inclusive
        inclusive: bool,
    },
}

impl Witness {
    /// Create a wildcard witness (any value)
    pub fn any() -> Self {
        Witness::Any
    }

    /// Create an enum case witness without associated values
    pub fn enum_case(name: impl Into<String>) -> Self {
        Witness::EnumCase {
            name: name.into(),
            args: vec![],
        }
    }

    /// Create an enum case witness with associated values
    pub fn enum_case_with_args(name: impl Into<String>, args: Vec<Witness>) -> Self {
        Witness::EnumCase {
            name: name.into(),
            args,
        }
    }

    /// Create a tuple witness
    pub fn tuple(elements: Vec<Witness>) -> Self {
        Witness::Tuple(elements)
    }

    /// Create a literal witness
    pub fn literal(value: impl Into<String>) -> Self {
        Witness::Literal(value.into())
    }

    /// Create a boolean witness
    pub fn bool(value: bool) -> Self {
        Witness::Bool(value)
    }

    /// Create an integer literal witness
    pub fn integer(value: i64) -> Self {
        Witness::Literal(value.to_string())
    }

    /// Create a string literal witness
    pub fn string(value: impl Into<String>) -> Self {
        let s = value.into();
        Witness::Literal(format!("\"{}\"", s))
    }

    /// Create an array witness
    pub fn array(elements: Vec<Witness>) -> Self {
        Witness::Array(elements)
    }

    /// Create a struct witness
    pub fn struct_witness(name: impl Into<String>, fields: Vec<(String, Witness)>) -> Self {
        Witness::Struct {
            name: name.into(),
            fields,
        }
    }

    /// Create a range witness
    pub fn range(start: impl Into<String>, end: impl Into<String>, inclusive: bool) -> Self {
        Witness::Range {
            start: start.into(),
            end: end.into(),
            inclusive,
        }
    }

    /// Format the witness for display in error messages
    pub fn display(&self) -> String {
        match self {
            Witness::Any => "_".to_string(),

            Witness::EnumCase { name, args } => {
                if args.is_empty() {
                    format!(".{}", name)
                } else {
                    let args_str: Vec<String> = args.iter().map(|a| a.display()).collect();
                    format!(".{}({})", name, args_str.join(", "))
                }
            }

            Witness::Tuple(elements) => {
                let elems_str: Vec<String> = elements.iter().map(|e| e.display()).collect();
                format!("({})", elems_str.join(", "))
            }

            Witness::Literal(s) => s.clone(),

            Witness::Bool(b) => b.to_string(),

            Witness::Array(elements) => {
                let elems_str: Vec<String> = elements.iter().map(|e| e.display()).collect();
                format!("[{}]", elems_str.join(", "))
            }

            Witness::Struct { name, fields } => {
                if fields.is_empty() {
                    format!("{} {{ .. }}", name)
                } else {
                    let fields_str: Vec<String> = fields
                        .iter()
                        .map(|(n, w)| format!("{}: {}", n, w.display()))
                        .collect();
                    format!("{} {{ {} }}", name, fields_str.join(", "))
                }
            }

            Witness::Range {
                start,
                end,
                inclusive,
            } => {
                if *inclusive {
                    format!("{}..={}", start, end)
                } else {
                    format!("{}..{}", start, end)
                }
            }
        }
    }
}

impl fmt::Display for Witness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_any_display() {
        assert_eq!(Witness::any().display(), "_");
    }

    #[test]
    fn test_enum_case_no_args() {
        let w = Witness::enum_case("None");
        assert_eq!(w.display(), ".None");
    }

    #[test]
    fn test_enum_case_with_args() {
        let w = Witness::enum_case_with_args("Some", vec![Witness::any()]);
        assert_eq!(w.display(), ".Some(_)");
    }

    #[test]
    fn test_enum_case_with_multiple_args() {
        let w =
            Witness::enum_case_with_args("Pair", vec![Witness::integer(1), Witness::integer(2)]);
        assert_eq!(w.display(), ".Pair(1, 2)");
    }

    #[test]
    fn test_tuple() {
        let w = Witness::tuple(vec![Witness::any(), Witness::integer(42)]);
        assert_eq!(w.display(), "(_, 42)");
    }

    #[test]
    fn test_empty_tuple() {
        let w = Witness::tuple(vec![]);
        assert_eq!(w.display(), "()");
    }

    #[test]
    fn test_literal() {
        assert_eq!(Witness::integer(42).display(), "42");
        assert_eq!(Witness::integer(-1).display(), "-1");
    }

    #[test]
    fn test_bool() {
        assert_eq!(Witness::bool(true).display(), "true");
        assert_eq!(Witness::bool(false).display(), "false");
    }

    #[test]
    fn test_string_literal() {
        let w = Witness::string("hello");
        assert_eq!(w.display(), "\"hello\"");
    }

    #[test]
    fn test_nested_witness() {
        // .Some((_, 42))
        let inner = Witness::tuple(vec![Witness::any(), Witness::integer(42)]);
        let w = Witness::enum_case_with_args("Some", vec![inner]);
        assert_eq!(w.display(), ".Some((_, 42))");
    }

    #[test]
    fn test_display_trait() {
        let w = Witness::enum_case("None");
        assert_eq!(format!("{}", w), ".None");
    }
}
