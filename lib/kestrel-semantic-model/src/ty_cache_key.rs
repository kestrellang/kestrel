//! Hashable cache key for Ty values.
//!
//! `Ty` does not implement `Hash`/`Eq` (it contains `Arc<dyn Symbol>` and `HashMap`).
//! `TyCacheKey` extracts a hashable representation for use as query cache keys.
//! Substitutions are sorted by SymbolId for deterministic hashing.

use std::hash::{Hash, Hasher};

use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::symbol::{Symbol, SymbolId};

/// A hashable, equality-comparable representation of a `Ty`.
///
/// Used as cache key in memoized queries where `Ty` appears in the input.
/// Query structs store both the original `Ty` (for execution) and a `TyCacheKey`
/// (for Hash/Eq), with custom impls that only use the cache key.
#[derive(Clone, Debug)]
pub enum TyCacheKey {
    /// Struct, Enum, Protocol, or TypeAlias — identified by SymbolId + substitutions
    Nominal {
        symbol_id: SymbolId,
        subs: Vec<(SymbolId, TyCacheKey)>,
    },
    /// Type parameter reference
    TypeParameter(SymbolId),
    /// Associated type (e.g., T.Item)
    AssociatedType {
        symbol_id: SymbolId,
        container: Option<Box<TyCacheKey>>,
    },
    /// Tuple type
    Tuple(Vec<TyCacheKey>),
    /// Raw pointer type
    Pointer(Box<TyCacheKey>),
    /// Function type
    Function {
        params: Vec<TyCacheKey>,
        return_type: Box<TyCacheKey>,
    },
    // Simple types
    Unit,
    Never,
    Bool,
    StringTy,
    Error,
    SelfType,
    Infer,
    Int(IntWidth),
    Float(FloatWidth),
    /// Fallback for unresolved types — uses Display string
    Opaque(String),
}

/// Integer width for cache key (avoids depending on IntBits directly for Hash)
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum IntWidth {
    I8,
    I16,
    I32,
    I64,
}

/// Float width for cache key
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FloatWidth {
    F16,
    F32,
    F64,
}

impl TyCacheKey {
    /// Convert a `Ty` into a hashable cache key.
    pub fn from_ty(ty: &Ty) -> Self {
        use kestrel_semantic_tree::ty::{FloatBits, IntBits};

        match ty.kind() {
            TyKind::Unit => TyCacheKey::Unit,
            TyKind::Never => TyCacheKey::Never,
            TyKind::Bool => TyCacheKey::Bool,
            TyKind::String => TyCacheKey::StringTy,
            TyKind::Error => TyCacheKey::Error,
            TyKind::SelfType => TyCacheKey::SelfType,
            TyKind::Infer => TyCacheKey::Infer,

            TyKind::Int(bits) => TyCacheKey::Int(match bits {
                IntBits::I8 => IntWidth::I8,
                IntBits::I16 => IntWidth::I16,
                IntBits::I32 => IntWidth::I32,
                IntBits::I64 => IntWidth::I64,
            }),

            TyKind::Float(bits) => TyCacheKey::Float(match bits {
                FloatBits::F16 => FloatWidth::F16,
                FloatBits::F32 => FloatWidth::F32,
                FloatBits::F64 => FloatWidth::F64,
            }),

            TyKind::Struct { symbol, substitutions } => TyCacheKey::Nominal {
                symbol_id: symbol.metadata().id(),
                subs: Self::subs_to_vec(substitutions),
            },

            TyKind::Enum { symbol, substitutions } => TyCacheKey::Nominal {
                symbol_id: symbol.metadata().id(),
                subs: Self::subs_to_vec(substitutions),
            },

            TyKind::Protocol { symbol, substitutions } => TyCacheKey::Nominal {
                symbol_id: symbol.metadata().id(),
                subs: Self::subs_to_vec(substitutions),
            },

            TyKind::TypeAlias { symbol, substitutions } => TyCacheKey::Nominal {
                symbol_id: symbol.metadata().id(),
                subs: Self::subs_to_vec(substitutions),
            },

            TyKind::TypeParameter(tp) => TyCacheKey::TypeParameter(tp.metadata().id()),

            TyKind::AssociatedType { symbol, container } => TyCacheKey::AssociatedType {
                symbol_id: symbol.metadata().id(),
                container: container.as_ref().map(|c| Box::new(TyCacheKey::from_ty(c))),
            },

            TyKind::Tuple(elements) => {
                TyCacheKey::Tuple(elements.iter().map(TyCacheKey::from_ty).collect())
            }

            TyKind::Pointer(pointee) => {
                TyCacheKey::Pointer(Box::new(TyCacheKey::from_ty(pointee)))
            }

            TyKind::Function { params, return_type } => TyCacheKey::Function {
                params: params.iter().map(TyCacheKey::from_ty).collect(),
                return_type: Box::new(TyCacheKey::from_ty(return_type)),
            },

            // Unresolved types — use string fallback
            TyKind::UnresolvedFunction { .. } | TyKind::UnresolvedPath { .. } => {
                TyCacheKey::Opaque(ty.to_string())
            }
        }
    }

    /// Convert substitutions to a sorted vec for deterministic hashing.
    fn subs_to_vec(subs: &kestrel_semantic_tree::ty::Substitutions) -> Vec<(SymbolId, TyCacheKey)> {
        let mut v: Vec<_> = subs
            .iter()
            .map(|(id, ty)| (*id, TyCacheKey::from_ty(ty)))
            .collect();
        v.sort_by_key(|(id, _)| id.raw());
        v
    }
}

impl PartialEq for TyCacheKey {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TyCacheKey::Nominal { symbol_id: a, subs: sa }, TyCacheKey::Nominal { symbol_id: b, subs: sb }) => {
                a == b && sa == sb
            }
            (TyCacheKey::TypeParameter(a), TyCacheKey::TypeParameter(b)) => a == b,
            (TyCacheKey::AssociatedType { symbol_id: a, container: ca }, TyCacheKey::AssociatedType { symbol_id: b, container: cb }) => {
                a == b && ca == cb
            }
            (TyCacheKey::Tuple(a), TyCacheKey::Tuple(b)) => a == b,
            (TyCacheKey::Pointer(a), TyCacheKey::Pointer(b)) => a == b,
            (TyCacheKey::Function { params: pa, return_type: ra }, TyCacheKey::Function { params: pb, return_type: rb }) => {
                pa == pb && ra == rb
            }
            (TyCacheKey::Unit, TyCacheKey::Unit) => true,
            (TyCacheKey::Never, TyCacheKey::Never) => true,
            (TyCacheKey::Bool, TyCacheKey::Bool) => true,
            (TyCacheKey::StringTy, TyCacheKey::StringTy) => true,
            (TyCacheKey::Error, TyCacheKey::Error) => true,
            (TyCacheKey::SelfType, TyCacheKey::SelfType) => true,
            (TyCacheKey::Infer, TyCacheKey::Infer) => true,
            (TyCacheKey::Int(a), TyCacheKey::Int(b)) => a == b,
            (TyCacheKey::Float(a), TyCacheKey::Float(b)) => a == b,
            (TyCacheKey::Opaque(a), TyCacheKey::Opaque(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for TyCacheKey {}

impl Hash for TyCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            TyCacheKey::Nominal { symbol_id, subs } => {
                symbol_id.hash(state);
                subs.hash(state);
            }
            TyCacheKey::TypeParameter(id) => id.hash(state),
            TyCacheKey::AssociatedType { symbol_id, container } => {
                symbol_id.hash(state);
                container.hash(state);
            }
            TyCacheKey::Tuple(elements) => elements.hash(state),
            TyCacheKey::Pointer(pointee) => pointee.hash(state),
            TyCacheKey::Function { params, return_type } => {
                params.hash(state);
                return_type.hash(state);
            }
            TyCacheKey::Int(w) => w.hash(state),
            TyCacheKey::Float(w) => w.hash(state),
            TyCacheKey::Opaque(s) => s.hash(state),
            // Simple variants — discriminant is enough
            TyCacheKey::Unit
            | TyCacheKey::Never
            | TyCacheKey::Bool
            | TyCacheKey::StringTy
            | TyCacheKey::Error
            | TyCacheKey::SelfType
            | TyCacheKey::Infer => {}
        }
    }
}
