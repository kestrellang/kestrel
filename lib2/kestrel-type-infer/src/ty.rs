//! Type representation for inference.
//!
//! All types during inference are `TyVar` — indices into a flat `Vec<TySlot>`.
//! This eliminates HashMap-based registries and substitution chains.

use kestrel_hecs::Entity;

/// Index into `InferCtx::types`. Cheap to copy.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct TyVar(pub(crate) u32);

/// State of a type variable.
#[derive(Clone, Debug)]
pub enum TySlot {
    /// Fresh, unconstrained. May have a literal marker.
    Unresolved { literal: Option<LiteralKind> },

    /// Redirects to another TyVar (unification link).
    Redirect(TyVar),

    /// Bound to a concrete type.
    Resolved(TyKind),
}

/// Concrete type representation used by the solver.
#[derive(Clone, Debug)]
pub enum TyKind {
    /// Named type (struct, enum, protocol): entity + type args.
    Named { entity: Entity, args: Vec<TyVar> },

    /// Type parameter from a generic declaration.
    Param { entity: Entity },

    /// Tuple type.
    Tuple(Vec<TyVar>),

    /// Function type: (params) → return.
    Function { params: Vec<TyVar>, ret: TyVar },

    /// Bottom type — diverging control flow (break, return, loop).
    /// Unifies with anything.
    Never,

    /// Error poison — only created after a diagnostic is emitted.
    /// Unifies with anything silently, preventing cascading errors.
    Error,
}

/// Literal kind marker on unresolved TyVars.
/// Controls which `ExpressibleBy*` protocol is required for unification
/// and which default type alias is applied when unconstrained.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LiteralKind {
    Integer,
    Float,
    String,
    Bool,
    Char,
    Null,
    /// Array literal — default: `@builtin(.DefaultArrayLiteralType)[_]`
    Array,
    /// Dictionary literal — default: `@builtin(.DefaultDictionaryLiteralType)[_, _]`
    Dictionary,
}
