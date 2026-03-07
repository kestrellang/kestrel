//! Resolved type representations for HIR.
//!
//! Type-level sugar is resolved before reaching HIR:
//! `Int?` → `Named(Optional, [Named(Int)])`, `[Int]` → `Named(Array, [Named(Int)])`, etc.
//! Only 6 variants instead of AST's 10.

use kestrel_hecs::Entity;
use kestrel_span2::Span;

/// A resolved type in HIR. All syntactic sugar has been expanded:
/// Optional, Array, Dictionary, Result are just `Named` with the
/// appropriate entity and type arguments.
#[derive(Clone, Debug, Hash)]
pub enum HirTy {
    /// Named type resolved to an entity. Covers structs, enums, protocols,
    /// type aliases, Optional, Array, Dictionary, Result — all just Named.
    Named {
        entity: Entity,
        args: Vec<HirTy>,
        span: Span,
    },
    /// Tuple type: `(Int, String)`
    Tuple(Vec<HirTy>, Span),
    /// Function type: `(Int, String) -> Bool`
    Function {
        params: Vec<HirTy>,
        ret: Box<HirTy>,
        span: Span,
    },
    /// Type parameter resolved to its declaring entity
    Param(Entity, Span),
    /// Never type (diverging expressions, e.g. `panic()`)
    Never(Span),
    /// Inferred type (user wrote `_` or omitted)
    Infer(Span),
    /// Error recovery
    Error(Span),
}
