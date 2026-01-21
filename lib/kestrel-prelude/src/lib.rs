//! Kestrel Prelude
//!
//! This crate defines the built-in "lang" names for the Kestrel language.
//! It serves as the single source of truth for compiler-known intrinsic names.

/// Lang module intrinsic names
pub mod lang {
    /// The "lang" module name
    pub const LANG: &str = "lang";
    /// Pointer type name
    pub const PTR: &str = "ptr";
    /// Panic unwind intrinsic function name
    pub const PANIC_UNWIND: &str = "panic_unwind";

    // Integer primitives
    /// 8-bit signed integer
    pub const I8: &str = "i8";
    /// 16-bit signed integer/
    pub const I16: &str = "i16";
    /// 32-bit signed integer
    pub const I32: &str = "i32";
    /// 64-bit signed integer
    pub const I64: &str = "i64";

    // Unsigned integer primitives
    /// 8-bit unsigned integer
    pub const U8: &str = "u8";
    /// 16-bit unsigned integer
    pub const U16: &str = "u16";
    /// 32-bit unsigned integer
    pub const U32: &str = "u32";
    /// 64-bit unsigned integer
    pub const U64: &str = "u64";

    // Boolean primitive
    /// 1-bit boolean
    pub const I1: &str = "i1";

    // Float primitives
    /// 16-bit float
    pub const F16: &str = "f16";
    /// 32-bit float
    pub const F32: &str = "f32";
    /// 64-bit float
    pub const F64: &str = "f64";

    // String primitive
    /// String reference
    pub const STR: &str = "str";
}
