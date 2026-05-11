//! MIR type representation.
//!
//! Types are by value — no interning, no `Id<Ty>` indirection.
//! Entity references point into the ECS for struct/enum/protocol/type-param identity.

use crate::MirModule;
use crate::item::CopyBehavior;
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
    /// Canonical unit value — the empty tuple.
    ///
    /// MIR has no `Unit` variant; `()` *is* `Tuple([])`. HIR already uses this
    /// representation (`AstType::Unit → HirTy::Tuple(Vec::new(), …)`), so
    /// keeping MIR in sync removes a class of "which form did this come through
    /// as?" bugs at the HIR→MIR boundary.
    pub fn unit() -> Self {
        MirTy::Tuple(Vec::new())
    }

    /// Check if this is the unit type (empty tuple).
    pub fn is_unit(&self) -> bool {
        matches!(self, MirTy::Tuple(elems) if elems.is_empty())
    }

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
        if self.is_unit() {
            return true;
        }
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
                | MirTy::Never
                | MirTy::Ref(_)
                | MirTy::RefMut(_)
                | MirTy::Pointer(_)
                | MirTy::FuncThin { .. }
        )
    }

    /// Compute this type's [`CopyBehavior`] using its definition in `module`.
    ///
    /// - Primitives, refs (`&T`/`&var T`), raw pointers, thin function pointers,
    ///   `Str`, `Never`, and unit tuples are [`CopyBehavior::Bitwise`].
    /// - [`MirTy::Named`] resolves to the matching `StructDef`/`EnumDef`'s
    ///   `copy_behavior` field. Returns [`CopyBehavior::None`] if no def is
    ///   found (so unresolved references fail safe).
    /// - [`MirTy::Tuple`] composes structurally: any `None` element makes the
    ///   tuple `None`; any non-`Bitwise` element makes the tuple non-`Bitwise`
    ///   (collapsed to a synthetic `Clone` marker — TODO: distinguish
    ///   structural compound clones from method clones).
    /// - [`MirTy::TypeParam`], [`MirTy::SelfType`], [`MirTy::AssociatedProjection`]
    ///   conservatively return `None`; a constraint-aware variant will be added
    ///   in Stage 6.
    /// - [`MirTy::FuncThick`] is composed from its env behavior — for now,
    ///   conservatively `None` until closure-env propagation lands.
    /// - [`MirTy::Error`] returns `None`.
    pub fn copy_behavior(&self, module: &MirModule) -> CopyBehavior {
        match self {
            // Primitives + bitwise types.
            MirTy::I8
            | MirTy::I16
            | MirTy::I32
            | MirTy::I64
            | MirTy::F16
            | MirTy::F32
            | MirTy::F64
            | MirTy::Bool
            | MirTy::Never
            | MirTy::Str
            | MirTy::Ref(_)
            | MirTy::RefMut(_)
            | MirTy::Pointer(_)
            | MirTy::FuncThin { .. } => CopyBehavior::Bitwise,

            // Tuples compose structurally. Unit (empty tuple) is Bitwise.
            MirTy::Tuple(elems) => {
                let mut kind = CopyBehavior::Bitwise;
                for elem in elems {
                    match elem.copy_behavior(module) {
                        CopyBehavior::None => return CopyBehavior::None,
                        CopyBehavior::Clone(e) => {
                            // First non-Bitwise child wins the marker; later
                            // structural-clone work uses the per-element
                            // behavior, not this marker.
                            if matches!(kind, CopyBehavior::Bitwise) {
                                kind = CopyBehavior::Clone(e);
                            }
                        },
                        CopyBehavior::Bitwise => {},
                    }
                }
                kind
            },

            // Named types: look up the matching def.
            MirTy::Named { entity, .. } => {
                if let Some(s) = module.structs.iter().find(|s| s.entity == *entity) {
                    return s.copy_behavior.clone();
                }
                if let Some(e) = module.enums.iter().find(|e| e.entity == *entity) {
                    return e.copy_behavior.clone();
                }
                // Unknown nominal — fail safe. Lowering errors should already
                // have surfaced.
                CopyBehavior::None
            },

            // Conservatively non-copyable until Stage 6's constraint-aware
            // variant. The verifier won't reject these — only `Value::Move`
            // on a non-`None` type — so this leaves us free to lower generic
            // bodies without panicking.
            MirTy::TypeParam(_)
            | MirTy::SelfType
            | MirTy::AssociatedProjection { .. }
            | MirTy::FuncThick { .. }
            | MirTy::Error => CopyBehavior::None,
        }
    }
}
