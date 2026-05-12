//! Move paths — interned per-function symbols for the places that participate
//! in ownership analysis.
//!
//! ## Stage 4 (current)
//!
//! Move paths are *root locals only*. Each non-parameter local with a
//! non-`Copy` type gets a `MovePathId`; the `MovePathSet` is the bidirectional
//! mapping. Fields and projections are folded into their root via
//! [`MovePathSet::lookup_place`].
//!
//! ## Stage 7 (planned)
//!
//! Move paths will become tree-structured (each `Field` / `Index` /
//! `Downcast` projection is a child of its parent path) to support
//! field-level partial moves. The dataflow API is designed so that change
//! is internal — `lookup_place` becomes structurally-aware, but the
//! consumer-facing types stay the same.

use std::collections::HashMap;

use kestrel_mir::{CopyBehavior, LocalId, MirBody, MirModule, MirTy, Place};

/// Per-function identifier for a move path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MovePathId(pub u32);

impl MovePathId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// One move path. Stage 4 stores only root-local info; Stage 7 adds
/// `parent`, `children`, and projection info.
#[derive(Debug, Clone)]
pub struct MovePath {
    /// The root local this path is rooted at.
    pub local: LocalId,
    /// The local's MIR type, captured at construction time so dataflow doesn't
    /// have to chase indices.
    pub ty: MirTy,
}

/// Set of move paths for one function body. Built once per body via
/// [`MovePathSet::build`].
#[derive(Debug, Clone, Default)]
pub struct MovePathSet {
    paths: Vec<MovePath>,
    by_local: HashMap<LocalId, MovePathId>,
}

/// Returns true if `ty` contains a type whose copy semantics the MIR layer
/// can't decide locally:
///
/// - `MirTy::TypeParam`, `MirTy::SelfType`, `MirTy::AssociatedProjection` —
///   syntactic placeholders for types that vary per instantiation.
/// - `MirTy::Error` — upstream inference failure; running move-check on
///   the broken structure produces noise.
///
/// Used by [`MovePathSet::build`] to skip tracking these as move paths,
/// matching the legacy HIR tracker's "treat unresolved as copyable" rule.
fn has_unresolved_ty(ty: &MirTy, module: &MirModule) -> bool {
    match ty {
        MirTy::TypeParam(_)
        | MirTy::SelfType
        | MirTy::AssociatedProjection { .. }
        | MirTy::Error => true,
        MirTy::Tuple(elems) => elems.iter().any(|t| has_unresolved_ty(t, module)),
        MirTy::Pointer(inner) | MirTy::Ref(inner) | MirTy::RefMut(inner) => {
            has_unresolved_ty(inner, module)
        },
        MirTy::Named { type_args, .. } => {
            type_args.iter().any(|t| has_unresolved_ty(t, module))
        },
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            params.iter().any(|t| has_unresolved_ty(t, module))
                || has_unresolved_ty(ret, module)
        },
        _ => false,
    }
}

impl MovePathSet {
    /// Build the move-path set for a function body.
    ///
    /// Includes every local whose type is *not* trivially copyable
    /// (`CopyBehavior::None`). Parameters are included too — they start
    /// `DefinitelyInit` at function entry. Copyable locals are skipped
    /// because moving them is illegal and they have no ownership-transfer
    /// semantics worth tracking.
    pub fn build(body: &MirBody, module: &MirModule) -> Self {
        let mut paths = Vec::new();
        let mut by_local = HashMap::new();
        for (i, local) in body.locals.iter().enumerate() {
            if local.ty.copy_behavior(module) != CopyBehavior::None {
                // Trivially copyable (or Cloneable, etc.) — moving is
                // illegal, no ownership transfer to track.
                continue;
            }
            // Types whose copy semantics depend on facts the MIR layer
            // can't resolve yet (associated-type projections, abstract
            // `Self`, raw type parameters, `MirTy::Error` from upstream
            // failures, or `MirTy::Named` referring to an associated-type
            // entity rather than a `StructDef`/`EnumDef`) are treated as
            // copyable for ownership purposes. The legacy HIR tracker
            // took the same permissive stance — doing otherwise produces
            // a flurry of false-positive E500s on perfectly fine generic
            // code (e.g. `iter::MapIterator.next` where `item:
            // Iterator.Item`).
            if has_unresolved_ty(&local.ty, module) {
                continue;
            }
            let id = MovePathId(paths.len() as u32);
            let local_id = LocalId::new(i);
            paths.push(MovePath {
                local: local_id,
                ty: local.ty.clone(),
            });
            by_local.insert(local_id, id);
        }
        MovePathSet { paths, by_local }
    }

    /// Total number of interned paths.
    pub fn len(&self) -> usize {
        self.paths.len()
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    pub fn paths(&self) -> &[MovePath] {
        &self.paths
    }

    pub fn get(&self, id: MovePathId) -> &MovePath {
        &self.paths[id.index()]
    }

    /// Look up the path for a `Place`. Stage 4 folds projections to the
    /// root local — `s.f.0` and `s` both resolve to `s`'s path. Stage 7
    /// will return distinct paths for distinct projections.
    pub fn lookup_place(&self, place: &Place) -> Option<MovePathId> {
        place.root_local().and_then(|l| self.by_local.get(&l).copied())
    }

    /// Look up by local ID directly.
    pub fn lookup_local(&self, local: LocalId) -> Option<MovePathId> {
        self.by_local.get(&local).copied()
    }
}
