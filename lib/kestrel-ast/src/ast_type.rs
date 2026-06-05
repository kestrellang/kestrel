//! AST-level type representation.
//!
//! Pure data types extracted from the CST during build. Stored in
//! `TypeAnnotation` components and embedded in `AstBody` nodes.
//! All types carry a `Span` for error reporting.

use kestrel_span::Span;

/// Parameter passing convention carried on function types.
///
/// Defined here (the lowest language-fact crate that AST/HIR/type-infer all
/// depend on) so a `mutating` closure/function parameter's convention can ride
/// the type from parse through inference. Mirrors `kestrel_mir::ParamConvention`;
/// converted at the mir-lower boundary.
///
/// `Consuming` is the default for an un-annotated function-type parameter — this
/// preserves pre-#106 lowering (mir-lower previously hardcoded `Consuming` for
/// every function-type param). `MutBorrow` is introduced only by an explicit
/// `mutating` annotation or an inference upgrade from an expected type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ParamConvention {
    /// Read-only borrow.
    Borrow,
    /// Mutable (by-reference) borrow — the `mutating` convention.
    MutBorrow,
    /// Takes ownership. Default for un-annotated function-type params.
    #[default]
    Consuming,
}

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
    /// Function type, e.g. `(Int) -> String` or `(mutating Grid) -> Unit`.
    /// `param_conventions` is parallel to `params` (same length); a
    /// `mutating` prefix on a param yields `MutBorrow`, otherwise `Consuming`.
    Function {
        params: Vec<AstType>,
        param_conventions: Vec<ParamConvention>,
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
    /// Opaque type, e.g. `some P`, `some P and Q`
    Some { bounds: Vec<AstType>, span: Span },
}
