//! AST-level type representation.
//!
//! Pure data types extracted from the CST during build. Stored in
//! `TypeAnnotation` components and embedded in `AstBody` nodes.
//! All types carry a `Span` for error reporting.

use kestrel_span2::Span;

/// A single segment in a qualified type path.
/// Each segment has a name and optional type arguments.
/// e.g. in `Array[Int].Iterator`, `Array[Int]` and `Iterator` are segments.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PathSegment {
    pub name: String,
    pub type_args: Vec<AstType>,
    pub span: Span,
}

/// AST-level type representation extracted from CST.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AstType {
    /// Named type with path segments, each optionally having type arguments.
    /// e.g. `Int64`, `std.collections.Array[Int64]`, `Array[Int].Iterator`
    Named {
        segments: Vec<PathSegment>,
        span: Span,
    },
    /// Tuple type, e.g. `(Int, String)`
    Tuple(Vec<AstType>, Span),
    /// Function type, e.g. `(Int) -> String`
    Function {
        params: Vec<AstType>,
        return_type: Box<AstType>,
        span: Span,
    },
    /// Array type, e.g. `[Int]`
    Array(Box<AstType>, Span),
    /// Dictionary type, e.g. `[String: Int]`
    Dictionary(Box<AstType>, Box<AstType>, Span),
    /// Optional type, e.g. `Int?`
    Optional(Box<AstType>, Span),
    /// Result type, e.g. `Int throws Error`
    Result {
        ok: Box<AstType>,
        err: Box<AstType>,
        span: Span,
    },
    /// Unit type `()`
    Unit(Span),
    /// Never type `Never`
    Never(Span),
    /// Inferred type `_`
    Inferred(Span),
}
