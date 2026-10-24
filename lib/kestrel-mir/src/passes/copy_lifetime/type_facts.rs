//! Type facts the copy lifetime passes rely on.

use rustc_hash::FxHashMap;

use crate::TyId;
use crate::item::{DropBehavior, TypeInfo};
use crate::mono::types::MonoModule;
use crate::ty::MirTy;

/// Memoized per-type answers. One instance per pass run.
#[derive(Default)]
pub struct TypeFacts {
    observable: FxHashMap<TyId, bool>,
    raw_pointer: FxHashMap<TyId, bool>,
}

impl TypeFacts {
    /// Dropping a value of `ty` may have an observable effect, so a copy of it
    /// must keep its scope-end drop. True when `ty` is marked
    /// `@observabledrop`, or when anything it can drop is: stored fields, enum
    /// payloads, type arguments, and pointer pointees (`RcBox[T]` stores only
    /// a `Pointer`, but dropping the last one drops the `T`).
    pub fn observable(&mut self, mono: &MonoModule, ty: TyId) -> bool {
        if let Some(&v) = self.observable.get(&ty) {
            return v;
        }
        // Provisional `false` breaks recursion through self-referential storage.
        self.observable.insert(ty, false);
        let v = match mono.ty_arena.get(ty).clone() {
            MirTy::Pointer(pointee) => self.observable(mono, pointee),
            MirTy::Tuple(ts) => ts.iter().any(|t| self.observable(mono, *t)),
            MirTy::Named { entity, type_args } => {
                mono.observable_drop.contains(&entity)
                    || type_args.iter().any(|t| self.observable(mono, *t))
                    || nominal(mono, entity, &type_args)
                        .is_some_and(|(_, fields)| fields.iter().any(|t| self.observable(mono, *t)))
            },
            // Closures may capture anything.
            MirTy::FuncThick { .. } => true,
            _ => false,
        };
        self.observable.insert(ty, v);
        v
    }

    /// A value of `ty` holds a raw pointer it does not manage — a view or
    /// pointer into someone else's storage that could dangle if that storage
    /// is released earlier. A pointer stored in a type with a `deinit` is that
    /// type's own managed storage (`RcBox`, array storage) and doesn't count;
    /// a view has no `deinit` because it frees nothing.
    pub fn holds_raw_pointer(&mut self, mono: &MonoModule, ty: TyId) -> bool {
        if let Some(&v) = self.raw_pointer.get(&ty) {
            return v;
        }
        self.raw_pointer.insert(ty, false);
        let v = match mono.ty_arena.get(ty).clone() {
            MirTy::Pointer(_) => true,
            MirTy::Tuple(ts) => ts.iter().any(|t| self.holds_raw_pointer(mono, *t)),
            MirTy::Named { entity, type_args } => match nominal(mono, entity, &type_args) {
                Some((info, _)) if has_deinit(info) => false,
                Some((_, fields)) => fields.iter().any(|t| self.holds_raw_pointer(mono, *t)),
                None => false,
            },
            // Closures may capture anything.
            MirTy::FuncThick { .. } => true,
            _ => false,
        };
        self.raw_pointer.insert(ty, v);
        v
    }
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
    let tys = e
        .cases
        .iter()
        .flat_map(|c| c.payload_fields.iter().map(|f| f.ty))
        .collect();
    Some((&e.type_info, tys))
}

fn has_deinit(info: &TypeInfo) -> bool {
    match &info.drop {
        DropBehavior::None => false,
        DropBehavior::StructDrop { deinit, .. } | DropBehavior::EnumDrop { deinit, .. } => {
            deinit.is_some()
        },
    }
}
