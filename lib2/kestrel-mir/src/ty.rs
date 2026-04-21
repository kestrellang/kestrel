//! MIR type representation.
//!
//! Types are by value — no interning, no `Id<Ty>` indirection.
//! Entity references point into the ECS for struct/enum/protocol/type-param identity.

use kestrel_hecs::Entity;

/// MIR type representation.
///
/// Used by value wherever types appear. `Entity` references resolve to names
/// via `MirModule.entity_names` for display.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MirTy {
    // === Primitives ===
    I8,
    I16,
    I32,
    I64,
    F16,
    F32,
    F64,
    Bool,
    Unit,
    Never,
    Str,

    // === Pointers and references ===
    /// `p[T]` — raw pointer
    Pointer(Box<MirTy>),
    /// `&T` — immutable reference
    Ref(Box<MirTy>),
    /// `&var T` — mutable reference
    RefMut(Box<MirTy>),

    // === Compound ===
    /// `(T1, T2, ...)` — tuple type
    Tuple(Vec<MirTy>),

    /// Named type (struct, enum, protocol) with optional type arguments.
    Named {
        entity: Entity,
        type_args: Vec<MirTy>,
    },

    // === Generics ===
    /// Type parameter reference.
    TypeParam(Entity),

    /// `Self` — the implementing type in a protocol context.
    SelfType,

    /// Associated type projection: `T.Element` where `T: Container`.
    /// Resolved during monomorphization via witness table lookup.
    AssociatedProjection {
        base: Box<MirTy>,
        protocol: Entity,
        name: String,
    },

    // === Function types ===
    /// Thin function pointer (no environment, FFI-safe).
    FuncThin {
        params: Vec<MirTy>,
        ret: Box<MirTy>,
    },

    /// Thick callable (has environment, can escape).
    FuncThick {
        params: Vec<MirTy>,
        ret: Box<MirTy>,
    },

    /// Error/poison type — used when lowering fails.
    Error,
}

impl MirTy {
    /// Check if this is a primitive integer type.
    pub fn is_integer(&self) -> bool {
        matches!(self, MirTy::I8 | MirTy::I16 | MirTy::I32 | MirTy::I64)
    }

    /// Check if this is a primitive float type.
    pub fn is_float(&self) -> bool {
        matches!(self, MirTy::F16 | MirTy::F32 | MirTy::F64)
    }

    /// Check if this is a reference type (immutable or mutable).
    pub fn is_reference(&self) -> bool {
        matches!(self, MirTy::Ref(_) | MirTy::RefMut(_))
    }

    /// Check if this is a pointer type.
    pub fn is_pointer(&self) -> bool {
        matches!(self, MirTy::Pointer(_))
    }

    /// Recursively check whether this type contains `MirTy::Error` anywhere.
    /// Used by MIR-lower's validate pass (hard ICE backstop) and anywhere else
    /// that needs to detect an unresolved-type leak from an upstream phase.
    pub fn contains_error(&self) -> bool {
        match self {
            MirTy::Error => true,
            MirTy::Named { type_args, .. } => type_args.iter().any(MirTy::contains_error),
            MirTy::Tuple(tys) => tys.iter().any(MirTy::contains_error),
            MirTy::Pointer(inner) | MirTy::Ref(inner) | MirTy::RefMut(inner) => {
                inner.contains_error()
            },
            MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
                params.iter().any(MirTy::contains_error) || ret.contains_error()
            },
            MirTy::AssociatedProjection { base, .. } => base.contains_error(),
            MirTy::TypeParam(_)
            | MirTy::SelfType
            | MirTy::Unit
            | MirTy::Never
            | MirTy::Bool
            | MirTy::I8
            | MirTy::I16
            | MirTy::I32
            | MirTy::I64
            | MirTy::F16
            | MirTy::F32
            | MirTy::F64
            | MirTy::Str => false,
        }
    }

    /// Check if this type is trivially copyable (passed by value, no ownership transfer).
    /// Includes primitives, refs, pointers, and thin function pointers.
    pub fn is_trivially_copyable(&self) -> bool {
        matches!(
            self,
            MirTy::I8
                | MirTy::I16
                | MirTy::I32
                | MirTy::I64
                | MirTy::F16
                | MirTy::F32
                | MirTy::F64
                | MirTy::Bool
                | MirTy::Unit
                | MirTy::Never
                | MirTy::Ref(_)
                | MirTy::RefMut(_)
                | MirTy::Pointer(_)
                | MirTy::FuncThin { .. }
        )
    }
}
