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
///
/// The `Named`-style variants are split by category so that consumers can't
/// silently forget to disambiguate via NodeKind lookups. Abstract associated
/// types carry their receiver explicitly via `AssocProjection`.
#[derive(Clone, Debug)]
pub enum TyKind {
    /// Struct type (includes Optional, Array, Dictionary, Result sugar targets).
    Struct { entity: Entity, args: Vec<TyVar> },

    /// Enum type.
    Enum { entity: Entity, args: Vec<TyVar> },

    /// Protocol type (used as a bound or, eventually, as an existential).
    Protocol { entity: Entity, args: Vec<TyVar> },

    /// Reducible alias reference. The solver equates it to the substituted
    /// definition (and emits bound obligations) via the `Reduce` constraint.
    /// An AliasUse without a concrete `TypeAnnotation` (i.e. an abstract
    /// associated type referenced without a known base) stays as TypeAlias
    /// and is resolved via protocol-bound lookup.
    TypeAlias { entity: Entity, args: Vec<TyVar> },

    /// Tuple type.
    Tuple(Vec<TyVar>),

    /// Function type: (params) → return.
    Function { params: Vec<TyVar>, ret: TyVar },

    /// Type parameter from a generic declaration.
    Param { entity: Entity },

    /// Abstract associated-type projection: `base.assoc`.
    /// Used when the receiver is known at HIR construction time (e.g. `T.Item`,
    /// `Self.Output`, nested `T.Iter.Item`). Member resolution consults the
    /// receiver's bounds via `base`.
    AssocProjection { base: TyVar, assoc: Entity },

    /// Bottom type — diverging control flow (break, return, loop).
    /// Unifies with anything.
    Never,

    /// Error poison — only created after a diagnostic is emitted.
    /// Unifies with anything silently, preventing cascading errors.
    Error,
}

impl TyKind {
    /// Entity associated with nominal types (Struct/Enum/Protocol/TypeAlias/Param).
    /// Returns None for Tuple/Function/AssocProjection/Never/Error.
    pub fn entity(&self) -> Option<Entity> {
        match self {
            TyKind::Struct { entity, .. }
            | TyKind::Enum { entity, .. }
            | TyKind::Protocol { entity, .. }
            | TyKind::TypeAlias { entity, .. } => Some(*entity),
            TyKind::Param { entity } => Some(*entity),
            _ => None,
        }
    }

    /// Type arguments for parameterized nominal types.
    /// Empty slice for variants that don't carry args.
    pub fn args(&self) -> &[TyVar] {
        match self {
            TyKind::Struct { args, .. }
            | TyKind::Enum { args, .. }
            | TyKind::Protocol { args, .. }
            | TyKind::TypeAlias { args, .. } => args,
            _ => &[],
        }
    }

    /// True for Struct/Enum/Protocol (has a direct, statically known member table).
    pub fn is_nominal_concrete(&self) -> bool {
        matches!(
            self,
            TyKind::Struct { .. } | TyKind::Enum { .. } | TyKind::Protocol { .. }
        )
    }

    /// True for TypeAlias variants.
    pub fn is_type_alias(&self) -> bool {
        matches!(self, TyKind::TypeAlias { .. })
    }
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
