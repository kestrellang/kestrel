use std::collections::HashMap;

use kestrel_hecs::Entity;

pub use crate::id::TyId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamConvention {
    Borrow,
    MutBorrow,
    Consuming,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MirTy {
    // Primitives
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

    // Pointers
    Pointer(TyId),

    // Compound
    Tuple(Vec<TyId>),
    Named {
        entity: Entity,
        type_args: Vec<TyId>,
    },

    // Generics (resolved by monomorphization)
    TypeParam(Entity),
    SelfType,
    AssociatedProjection {
        base: TyId,
        protocol: Entity,
        assoc_type: Entity,
    },

    // Function types
    FuncThin {
        params: Vec<(TyId, ParamConvention)>,
        ret: TyId,
    },
    FuncThick {
        params: Vec<(TyId, ParamConvention)>,
        ret: TyId,
    },

    // Poison
    Error,
}

#[derive(Debug)]
pub struct TyArena {
    types: Vec<MirTy>,
    intern_map: HashMap<MirTy, TyId>,
}

impl TyArena {
    pub fn new() -> Self {
        Self {
            types: Vec::new(),
            intern_map: HashMap::new(),
        }
    }

    pub fn intern(&mut self, ty: MirTy) -> TyId {
        if let Some(&id) = self.intern_map.get(&ty) {
            return id;
        }
        let id = TyId::new(self.types.len());
        self.types.push(ty.clone());
        self.intern_map.insert(ty, id);
        id
    }

    pub fn get(&self, id: TyId) -> &MirTy {
        &self.types[id.index()]
    }

    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    // Convenience constructors for common types

    pub fn i8(&mut self) -> TyId {
        self.intern(MirTy::I8)
    }

    pub fn i16(&mut self) -> TyId {
        self.intern(MirTy::I16)
    }

    pub fn i32(&mut self) -> TyId {
        self.intern(MirTy::I32)
    }

    pub fn i64(&mut self) -> TyId {
        self.intern(MirTy::I64)
    }

    pub fn f16(&mut self) -> TyId {
        self.intern(MirTy::F16)
    }

    pub fn f32(&mut self) -> TyId {
        self.intern(MirTy::F32)
    }

    pub fn f64(&mut self) -> TyId {
        self.intern(MirTy::F64)
    }

    pub fn bool(&mut self) -> TyId {
        self.intern(MirTy::Bool)
    }

    pub fn never(&mut self) -> TyId {
        self.intern(MirTy::Never)
    }

    pub fn str_ty(&mut self) -> TyId {
        self.intern(MirTy::Str)
    }

    pub fn unit(&mut self) -> TyId {
        self.intern(MirTy::Tuple(vec![]))
    }

    pub fn pointer(&mut self, pointee: TyId) -> TyId {
        self.intern(MirTy::Pointer(pointee))
    }

    pub fn tuple(&mut self, elems: Vec<TyId>) -> TyId {
        self.intern(MirTy::Tuple(elems))
    }

    pub fn named(&mut self, entity: Entity, type_args: Vec<TyId>) -> TyId {
        self.intern(MirTy::Named { entity, type_args })
    }

    pub fn error(&mut self) -> TyId {
        self.intern(MirTy::Error)
    }
}

impl Default for TyArena {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_starts_empty() {
        let arena = TyArena::new();
        assert!(arena.is_empty());
        assert_eq!(arena.len(), 0);
    }

    #[test]
    fn intern_primitive_returns_id_zero() {
        let mut arena = TyArena::new();
        let id = arena.intern(MirTy::I64);
        assert_eq!(id, TyId::new(0));
    }

    #[test]
    fn intern_dedup() {
        let mut arena = TyArena::new();
        let a = arena.intern(MirTy::I64);
        let b = arena.intern(MirTy::I64);
        assert_eq!(a, b);
        assert_eq!(arena.len(), 1);
    }

    #[test]
    fn different_types_get_different_ids() {
        let mut arena = TyArena::new();
        let a = arena.intern(MirTy::I64);
        let b = arena.intern(MirTy::Bool);
        assert_ne!(a, b);
        assert_eq!(arena.len(), 2);
    }

    #[test]
    fn get_round_trip() {
        let mut arena = TyArena::new();
        let id = arena.intern(MirTy::I64);
        assert_eq!(arena.get(id), &MirTy::I64);
    }

    #[test]
    fn convenience_i64_dedup() {
        let mut arena = TyArena::new();
        let a = arena.i64();
        let b = arena.i64();
        assert_eq!(a, b);
        assert_eq!(arena.len(), 1);
    }

    #[test]
    fn unit_is_empty_tuple() {
        let mut arena = TyArena::new();
        let id = arena.unit();
        assert_eq!(arena.get(id), &MirTy::Tuple(vec![]));
    }

    #[test]
    fn pointer_round_trip() {
        let mut arena = TyArena::new();
        let i64_ty = arena.i64();
        let ptr_ty = arena.pointer(i64_ty);
        assert_eq!(arena.get(ptr_ty), &MirTy::Pointer(i64_ty));
    }

    #[test]
    fn named_type_with_args() {
        let mut arena = TyArena::new();
        let entity = Entity::from_raw(1);
        let i64_ty = arena.i64();
        let named = arena.named(entity, vec![i64_ty]);
        match arena.get(named) {
            MirTy::Named {
                entity: e,
                type_args,
            } => {
                assert_eq!(*e, entity);
                assert_eq!(type_args, &[i64_ty]);
            }
            other => panic!("expected Named, got {other:?}"),
        }
    }

    #[test]
    fn named_type_dedup() {
        let mut arena = TyArena::new();
        let entity = Entity::from_raw(1);
        let i64_ty = arena.i64();
        let a = arena.named(entity, vec![i64_ty]);
        let b = arena.named(entity, vec![i64_ty]);
        assert_eq!(a, b);
    }

    #[test]
    fn named_different_args_differ() {
        let mut arena = TyArena::new();
        let entity = Entity::from_raw(1);
        let i64_ty = arena.i64();
        let bool_ty = arena.bool();
        let a = arena.named(entity, vec![i64_ty]);
        let b = arena.named(entity, vec![bool_ty]);
        assert_ne!(a, b);
    }

    #[test]
    fn all_convenience_constructors() {
        let mut arena = TyArena::new();
        let _ = arena.i8();
        let _ = arena.i16();
        let _ = arena.i32();
        let _ = arena.i64();
        let _ = arena.f16();
        let _ = arena.f32();
        let _ = arena.f64();
        let _ = arena.bool();
        let _ = arena.never();
        let _ = arena.str_ty();
        let _ = arena.unit();
        let _ = arena.error();
        // 10 distinct primitives + unit (Tuple([])) + error = 12
        assert_eq!(arena.len(), 12);
    }

    #[test]
    fn tuple_type() {
        let mut arena = TyArena::new();
        let i64_ty = arena.i64();
        let bool_ty = arena.bool();
        let tup = arena.tuple(vec![i64_ty, bool_ty]);
        match arena.get(tup) {
            MirTy::Tuple(elems) => assert_eq!(elems, &[i64_ty, bool_ty]),
            other => panic!("expected Tuple, got {other:?}"),
        }
    }

    #[test]
    fn type_param() {
        let mut arena = TyArena::new();
        let entity = Entity::from_raw(42);
        let id = arena.intern(MirTy::TypeParam(entity));
        assert_eq!(arena.get(id), &MirTy::TypeParam(entity));
    }

    #[test]
    fn self_type() {
        let mut arena = TyArena::new();
        let id = arena.intern(MirTy::SelfType);
        assert_eq!(arena.get(id), &MirTy::SelfType);
    }

    #[test]
    fn associated_projection() {
        let mut arena = TyArena::new();
        let base = arena.intern(MirTy::TypeParam(Entity::from_raw(1)));
        let proto = Entity::from_raw(2);
        let assoc = Entity::from_raw(3);
        let id = arena.intern(MirTy::AssociatedProjection {
            base,
            protocol: proto,
            assoc_type: assoc,
        });
        match arena.get(id) {
            MirTy::AssociatedProjection {
                base: b,
                protocol: p,
                assoc_type: a,
            } => {
                assert_eq!(*b, base);
                assert_eq!(*p, proto);
                assert_eq!(*a, assoc);
            }
            other => panic!("expected AssociatedProjection, got {other:?}"),
        }
    }

    #[test]
    fn func_thin() {
        let mut arena = TyArena::new();
        let i64_ty = arena.i64();
        let bool_ty = arena.bool();
        let id = arena.intern(MirTy::FuncThin {
            params: vec![(i64_ty, ParamConvention::Consuming)],
            ret: bool_ty,
        });
        match arena.get(id) {
            MirTy::FuncThin { params, ret } => {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0], (i64_ty, ParamConvention::Consuming));
                assert_eq!(*ret, bool_ty);
            }
            other => panic!("expected FuncThin, got {other:?}"),
        }
    }

    #[test]
    fn param_convention_copy() {
        let a = ParamConvention::Borrow;
        let b = a;
        assert_eq!(a, b);
    }
}
