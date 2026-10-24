//! Type facts the copy lifetime passes rely on.

use rustc_hash::FxHashMap;

use crate::TyId;
use crate::item::{DropBehavior, TypeInfo};
use crate::mono::types::MonoModule;
use crate::ty::MirTy;

/// Memoized per-type answers. One instance per pass run.
#[derive(Default)]
pub struct TypeFacts {
    memory_only: FxHashMap<TyId, bool>,
    raw_pointer: FxHashMap<TyId, bool>,
}

impl TypeFacts {
    /// Dropping a value of `ty` only releases memory, so dropping it earlier
    /// is unobservable. True when every `deinit` reachable from `ty` belongs to
    /// a type marked `@memoryonlydrop` — and, for a marked type, its type
    /// arguments qualify too, since its deinit may drop values of those types
    /// (`RcBox[File]` frees a `File`). Copyable types never have a deinit.
    pub fn memory_only(&mut self, mono: &MonoModule, ty: TyId) -> bool {
        if let Some(&v) = self.memory_only.get(&ty) {
            return v;
        }
        // Provisional `true` breaks recursion through self-referential storage.
        self.memory_only.insert(ty, true);
        let v = match mono.ty_arena.get(ty).clone() {
            MirTy::Tuple(ts) => ts.iter().all(|t| self.memory_only(mono, *t)),
            MirTy::Named { entity, type_args } => {
                let Some((info, fields)) = nominal(mono, entity, &type_args) else {
                    return self.settle_memory(ty, false);
                };
                let marked = mono.memory_only_drop.contains(&entity);
                let deinit_ok = deinit(info).is_none() || marked;
                let args_ok = !marked || type_args.iter().all(|t| self.memory_only(mono, *t));
                deinit_ok && args_ok && fields.iter().all(|t| self.memory_only(mono, *t))
            },
            ty => is_scalar(&ty),
        };
        self.settle_memory(ty, v)
    }

    /// A value of `ty` holds a raw pointer it does not manage — a view or
    /// pointer into someone else's storage that could dangle if that storage
    /// is released earlier. Pointers inside `@memoryonlydrop` types are the
    /// type's own managed storage and don't count.
    pub fn holds_raw_pointer(&mut self, mono: &MonoModule, ty: TyId) -> bool {
        if let Some(&v) = self.raw_pointer.get(&ty) {
            return v;
        }
        self.raw_pointer.insert(ty, false);
        let v = match mono.ty_arena.get(ty).clone() {
            MirTy::Pointer(_) => true,
            MirTy::Tuple(ts) => ts.iter().any(|t| self.holds_raw_pointer(mono, *t)),
            MirTy::Named { entity, type_args } => {
                if mono.memory_only_drop.contains(&entity) {
                    false
                } else {
                    let fields = nominal(mono, entity, &type_args).map(|(_, f)| f).unwrap_or_default();
                    fields.iter().any(|t| self.holds_raw_pointer(mono, *t))
                }
            },
            // Closures may capture anything.
            MirTy::FuncThick { .. } => true,
            _ => false,
        };
        self.raw_pointer.insert(ty, v);
        v
    }

    fn settle_memory(&mut self, ty: TyId, v: bool) -> bool {
        self.memory_only.insert(ty, v);
        v
    }
}

fn is_scalar(ty: &MirTy) -> bool {
    matches!(
        ty,
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
            | MirTy::Pointer(_)
    )
}

/// Type info and stored field types of a monomorphized struct or enum.
fn nominal<'m>(
    mono: &'m MonoModule,
    entity: kestrel_hecs::Entity,
    type_args: &[TyId],
) -> Option<(&'m TypeInfo, Vec<TyId>)> {
    let key = (entity, type_args.to_vec());
    if let Some(s) = mono.structs.get(&key) {
        return Some((&s.type_info, s.fields.iter().map(|f| f.ty).collect()));
    }
    let e = mono.enums.get(&key)?;
    let tys = e.cases.iter().flat_map(|c| c.payload_fields.iter().map(|f| f.ty)).collect();
    Some((&e.type_info, tys))
}

fn deinit(info: &TypeInfo) -> Option<kestrel_hecs::Entity> {
    match &info.drop {
        DropBehavior::None => None,
        DropBehavior::StructDrop { deinit, .. } | DropBehavior::EnumDrop { deinit, .. } => *deinit,
    }
}
