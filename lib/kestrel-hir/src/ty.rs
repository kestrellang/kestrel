//! Resolved type representations for HIR.
//!
//! Type-level sugar is resolved before reaching HIR:
//! `Int?` → `Struct(Optional, [Struct(Int)])`, `[Int]` → `Struct(Array, [Struct(Int)])`, etc.
//!
//! The `Named` variant is split into explicit Struct/Enum/Protocol/AliasUse variants
//! so that consumers can't silently forget to disambiguate. Abstract associated types
//! have their own variant (`AssocProjection`) that carries the receiver explicitly.

use kestrel_hecs::Entity;
use kestrel_span2::Span;

/// A resolved type in HIR. All syntactic sugar has been expanded:
/// Optional, Array, Dictionary, Result are just `Struct` with the
/// appropriate entity and type arguments.
#[derive(Clone, Debug, Hash)]
pub enum HirTy {
    /// Struct type (includes Optional, Array, Dictionary, Result sugar).
    Struct {
        entity: Entity,
        args: Vec<HirTy>,
        span: Span,
    },
    /// Enum type.
    Enum {
        entity: Entity,
        args: Vec<HirTy>,
        span: Span,
    },
    /// Protocol type (used as a bound or, eventually, as an existential).
    Protocol {
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
    /// Use of a regular (non-associated) type alias. Inference reduces this
    /// to the substituted definition and emits any bound obligations.
    AliasUse {
        entity: Entity,
        args: Vec<HirTy>,
        span: Span,
    },
    /// Type parameter resolved to its declaring entity.
    Param(Entity, Span),
    /// `Self` inside a protocol declaration or `extend <protocol>` — the
    /// abstract implementing type. Resolves to a concrete type at
    /// monomorphization time via `MirTy::SelfType`. Only used in contexts
    /// where the enclosing "Self" is a protocol; extensions on concrete
    /// types emit `Struct`/`Enum` directly. Carries the owning protocol
    /// entity so every lowering site can substitute it without relying on
    /// ambient context (e.g. where-clause RHS lowering from an outer body).
    SelfType(Entity, Span),
    /// Abstract associated-type projection: `base.assoc` (e.g. `T.Item`, `Self.Output`).
    /// `base` is the receiver type; `assoc` is the TypeAlias entity on the protocol.
    /// Nested projections chain naturally: `T.Next.Next` is AssocProjection over AssocProjection.
    AssocProjection {
        base: Box<HirTy>,
        assoc: Entity,
        span: Span,
    },
    /// Never type (diverging expressions, e.g. `panic()`)
    Never(Span),
    /// Inferred type (user wrote `_` or omitted)
    Infer(Span),
    /// Error recovery
    Error(Span),
}
