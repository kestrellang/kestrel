//! # Witness Values
//!
//! A witness is an example value demonstrating a gap in pattern coverage.
//! Used to generate helpful error messages:
//!
//! ```text
//! error: non-exhaustive match: missing .None
//!  --> file.ks:10:5
//!   | match opt {
//!   |     .Some(x) => x
//!   | }
//! ```
//!
//! Each `Constructor` can produce a witness via `Constructor::to_witness()`.
//! The exhaustiveness checker collects witnesses for all uncovered constructors.

use std::fmt;

/// An example value that demonstrates why a match is non-exhaustive
/// or which constructor is missing.
#[derive(Debug, Clone, PartialEq)]
pub enum Witness {
    /// Any value of the type (displayed as `_`)
    Any,
    /// A specific enum case (e.g., `.None`, `.Some(_)`)
    EnumCase { name: String, args: Vec<Witness> },
    /// A tuple value (e.g., `(_, 42)`)
    Tuple(Vec<Witness>),
    /// A literal value (e.g., `42`, `"hello"`)
    Literal(String),
    /// A boolean value
    Bool(bool),
    /// An array value (e.g., `[_, 42]`)
    Array(Vec<Witness>),
    /// A struct value (e.g., `Point { x: _, y: 0 }`)
    Struct {
        name: String,
        fields: Vec<(String, Witness)>,
    },
    /// A range value (e.g., `0..=9`)
    Range {
        start: String,
        end: String,
        inclusive: bool,
    },
}

impl Witness {
    pub fn any() -> Self {
        Witness::Any
    }

    pub fn bool(value: bool) -> Self {
        Witness::Bool(value)
    }

    pub fn integer(value: i64) -> Self {
        Witness::Literal(value.to_string())
    }

    pub fn string(value: &str) -> Self {
        Witness::Literal(format!("\"{}\"", value))
    }

    pub fn enum_case(name: &str) -> Self {
        Witness::EnumCase {
            name: name.to_string(),
            args: vec![],
        }
    }

    pub fn enum_case_with_args(name: &str, args: Vec<Witness>) -> Self {
        Witness::EnumCase {
            name: name.to_string(),
            args,
        }
    }

    pub fn tuple(elements: Vec<Witness>) -> Self {
        Witness::Tuple(elements)
    }

    pub fn array(elements: Vec<Witness>) -> Self {
        Witness::Array(elements)
    }

    pub fn struct_witness(name: &str, fields: Vec<(String, Witness)>) -> Self {
        Witness::Struct {
            name: name.to_string(),
            fields,
        }
    }

    pub fn range(start: String, end: String, inclusive: bool) -> Self {
        Witness::Range {
            start,
            end,
            inclusive,
        }
    }
}

impl fmt::Display for Witness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Witness::Any => write!(f, "_"),
            Witness::EnumCase { name, args } => {
                if args.is_empty() {
                    write!(f, ".{}", name)
                } else {
                    let args_str: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                    write!(f, ".{}({})", name, args_str.join(", "))
                }
            },
            Witness::Tuple(elems) => {
                let strs: Vec<String> = elems.iter().map(|e| e.to_string()).collect();
                write!(f, "({})", strs.join(", "))
            },
            Witness::Literal(s) => write!(f, "{}", s),
            Witness::Bool(b) => write!(f, "{}", b),
            Witness::Array(elems) => {
                let strs: Vec<String> = elems.iter().map(|e| e.to_string()).collect();
                write!(f, "[{}]", strs.join(", "))
            },
            Witness::Struct { name, fields } => {
                if fields.is_empty() {
                    write!(f, "{} {{ .. }}", name)
                } else {
                    let strs: Vec<String> = fields
                        .iter()
                        .map(|(n, w)| format!("{}: {}", n, w))
                        .collect();
                    write!(f, "{} {{ {} }}", name, strs.join(", "))
                }
            },
            Witness::Range {
                start,
                end,
                inclusive,
            } => {
                if *inclusive {
                    write!(f, "{}..={}", start, end)
                } else {
                    write!(f, "{}..{}", start, end)
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_any() {
        assert_eq!(Witness::any().to_string(), "_");
    }

    #[test]
    fn display_enum_case() {
        assert_eq!(Witness::enum_case("None").to_string(), ".None");
        assert_eq!(
            Witness::enum_case_with_args("Some", vec![Witness::any()]).to_string(),
            ".Some(_)"
        );
    }

    #[test]
    fn display_tuple() {
        let w = Witness::tuple(vec![Witness::any(), Witness::integer(42)]);
        assert_eq!(w.to_string(), "(_, 42)");
    }

    #[test]
    fn display_bool() {
        assert_eq!(Witness::bool(true).to_string(), "true");
        assert_eq!(Witness::bool(false).to_string(), "false");
    }

    #[test]
    fn display_nested() {
        let inner = Witness::tuple(vec![Witness::any(), Witness::integer(42)]);
        let w = Witness::enum_case_with_args("Some", vec![inner]);
        assert_eq!(w.to_string(), ".Some((_, 42))");
    }
}
