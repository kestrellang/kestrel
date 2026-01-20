//! Kestrel Prelude
//!
//! This crate defines the primitive types and built-in names for the Kestrel language.
//! It serves as the single source of truth for all primitive type names and their mappings.

/// Lang module intrinsic names
pub mod lang {
    /// The "lang" module name
    pub const LANG: &str = "lang";
    /// Pointer type name
    pub const PTR: &str = "ptr";
}

/// Primitive type name constants
pub mod primitives {
    // Integer types
    pub const INT: &str = "Int";
    pub const I8: &str = "I8";
    pub const I16: &str = "I16";
    pub const I32: &str = "I32";
    pub const I64: &str = "I64";

    // Float types
    pub const FLOAT: &str = "Float";
    pub const F32: &str = "F32";
    pub const F64: &str = "F64";

    // Other primitives
    pub const BOOL: &str = "Bool";
    pub const STRING: &str = "String";

    // Special types
    pub const SELF_TYPE: &str = "Self";

    /// All primitive type names for iteration
    pub const ALL: &[&str] = &[INT, I8, I16, I32, I64, FLOAT, F32, F64, BOOL, STRING];

    /// Check if a name is a primitive type
    pub fn is_primitive(name: &str) -> bool {
        ALL.contains(&name)
    }

    /// Check if a name is a special type (like Self)
    pub fn is_special(name: &str) -> bool {
        name == SELF_TYPE
    }

    /// Check if a name is a built-in type (primitive or special)
    pub fn is_builtin(name: &str) -> bool {
        is_primitive(name) || is_special(name)
    }
}

#[cfg(test)]
mod tests {
    use super::primitives::*;

    #[test]
    fn test_is_primitive() {
        assert!(is_primitive("Int"));
        assert!(is_primitive("Bool"));
        assert!(is_primitive("String"));
        assert!(is_primitive("F64"));
        assert!(!is_primitive("MyType"));
        assert!(!is_primitive("Self"));
    }

    #[test]
    fn test_is_special() {
        assert!(is_special("Self"));
        assert!(!is_special("Int"));
    }

    #[test]
    fn test_is_builtin() {
        assert!(is_builtin("Int"));
        assert!(is_builtin("Self"));
        assert!(!is_builtin("MyType"));
    }
}
