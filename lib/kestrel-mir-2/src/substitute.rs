use std::collections::HashMap;

use kestrel_hecs::Entity;

use crate::ty::{MirTy, TyArena, TyId};
use crate::ParamConvention;

#[derive(Debug, Clone, Default)]
pub struct SubstMap {
    pub type_params: HashMap<Entity, TyId>,
    pub self_type: Option<TyId>,
    pub assoc_types: HashMap<(TyId, Entity, Entity), TyId>,
}

impl SubstMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.type_params.is_empty() && self.self_type.is_none() && self.assoc_types.is_empty()
    }
}

pub fn substitute(arena: &mut TyArena, ty: TyId, subst: &SubstMap) -> TyId {
    if subst.is_empty() {
        return ty;
    }

    let mir_ty = arena.get(ty).clone();
    match mir_ty {
        // Primitives and error are unchanged
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
        | MirTy::Error => ty,

        MirTy::TypeParam(entity) => {
            if let Some(&replacement) = subst.type_params.get(&entity) {
                replacement
            } else {
                ty
            }
        }

        MirTy::SelfType => {
            if let Some(replacement) = subst.self_type {
                replacement
            } else {
                ty
            }
        }

        MirTy::AssociatedProjection {
            base,
            protocol,
            assoc_type,
        } => {
            let sub_base = substitute(arena, base, subst);
            if let Some(&concrete) = subst.assoc_types.get(&(sub_base, protocol, assoc_type)) {
                concrete
            } else if sub_base != base {
                arena.intern(MirTy::AssociatedProjection {
                    base: sub_base,
                    protocol,
                    assoc_type,
                })
            } else {
                ty
            }
        }

        MirTy::Pointer(pointee) => {
            let sub_pointee = substitute(arena, pointee, subst);
            if sub_pointee != pointee {
                arena.pointer(sub_pointee)
            } else {
                ty
            }
        }

        MirTy::Tuple(elems) => {
            let sub_elems: Vec<_> = elems.iter().map(|&e| substitute(arena, e, subst)).collect();
            if sub_elems != elems {
                arena.tuple(sub_elems)
            } else {
                ty
            }
        }

        MirTy::Named { entity, type_args } => {
            let sub_args: Vec<_> = type_args
                .iter()
                .map(|&a| substitute(arena, a, subst))
                .collect();
            if sub_args != type_args {
                arena.named(entity, sub_args)
            } else {
                ty
            }
        }

        MirTy::FuncThin { params, ret } => {
            let sub_params: Vec<(TyId, ParamConvention)> = params
                .iter()
                .map(|&(t, conv)| (substitute(arena, t, subst), conv))
                .collect();
            let sub_ret = substitute(arena, ret, subst);
            if sub_params != params || sub_ret != ret {
                arena.intern(MirTy::FuncThin {
                    params: sub_params,
                    ret: sub_ret,
                })
            } else {
                ty
            }
        }

        MirTy::FuncThick { params, ret } => {
            let sub_params: Vec<(TyId, ParamConvention)> = params
                .iter()
                .map(|&(t, conv)| (substitute(arena, t, subst), conv))
                .collect();
            let sub_ret = substitute(arena, ret, subst);
            if sub_params != params || sub_ret != ret {
                arena.intern(MirTy::FuncThick {
                    params: sub_params,
                    ret: sub_ret,
                })
            } else {
                ty
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(n: u32) -> Entity {
        Entity::from_raw(n)
    }

    #[test]
    fn substitute_no_op() {
        let mut arena = TyArena::new();
        let i64_ty = arena.i64();
        let subst = SubstMap::new();
        assert_eq!(substitute(&mut arena, i64_ty, &subst), i64_ty);
    }

    #[test]
    fn substitute_type_param() {
        let mut arena = TyArena::new();
        let t_entity = entity(1);
        let t_ty = arena.intern(MirTy::TypeParam(t_entity));
        let i64_ty = arena.i64();

        let mut subst = SubstMap::new();
        subst.type_params.insert(t_entity, i64_ty);

        let result = substitute(&mut arena, t_ty, &subst);
        assert_eq!(result, i64_ty);
    }

    #[test]
    fn substitute_self_type() {
        let mut arena = TyArena::new();
        let self_ty = arena.intern(MirTy::SelfType);
        let i64_ty = arena.i64();

        let mut subst = SubstMap::new();
        subst.self_type = Some(i64_ty);

        let result = substitute(&mut arena, self_ty, &subst);
        assert_eq!(result, i64_ty);
    }

    #[test]
    fn substitute_pointer() {
        let mut arena = TyArena::new();
        let t_entity = entity(1);
        let t_ty = arena.intern(MirTy::TypeParam(t_entity));
        let ptr_t = arena.pointer(t_ty);
        let i64_ty = arena.i64();

        let mut subst = SubstMap::new();
        subst.type_params.insert(t_entity, i64_ty);

        let result = substitute(&mut arena, ptr_t, &subst);
        let expected = arena.pointer(i64_ty);
        assert_eq!(result, expected);
    }

    #[test]
    fn substitute_named_type_args() {
        let mut arena = TyArena::new();
        let array_entity = entity(10);
        let t_entity = entity(1);
        let t_ty = arena.intern(MirTy::TypeParam(t_entity));
        let array_t = arena.named(array_entity, vec![t_ty]);
        let i64_ty = arena.i64();

        let mut subst = SubstMap::new();
        subst.type_params.insert(t_entity, i64_ty);

        let result = substitute(&mut arena, array_t, &subst);
        let expected = arena.named(array_entity, vec![i64_ty]);
        assert_eq!(result, expected);
    }

    #[test]
    fn substitute_tuple() {
        let mut arena = TyArena::new();
        let t_entity = entity(1);
        let t_ty = arena.intern(MirTy::TypeParam(t_entity));
        let bool_ty = arena.bool();
        let tup = arena.tuple(vec![t_ty, bool_ty]);
        let i64_ty = arena.i64();

        let mut subst = SubstMap::new();
        subst.type_params.insert(t_entity, i64_ty);

        let result = substitute(&mut arena, tup, &subst);
        let expected = arena.tuple(vec![i64_ty, bool_ty]);
        assert_eq!(result, expected);
    }

    #[test]
    fn substitute_preserves_primitives() {
        let mut arena = TyArena::new();
        let i64_ty = arena.i64();
        let bool_ty = arena.bool();

        let mut subst = SubstMap::new();
        subst.type_params.insert(entity(1), arena.i32());

        assert_eq!(substitute(&mut arena, i64_ty, &subst), i64_ty);
        assert_eq!(substitute(&mut arena, bool_ty, &subst), bool_ty);
    }

    #[test]
    fn substitute_unmatched_type_param_unchanged() {
        let mut arena = TyArena::new();
        let t_entity = entity(1);
        let u_entity = entity(2);
        let t_ty = arena.intern(MirTy::TypeParam(t_entity));

        let mut subst = SubstMap::new();
        subst.type_params.insert(u_entity, arena.i64());

        assert_eq!(substitute(&mut arena, t_ty, &subst), t_ty);
    }

    #[test]
    fn substitute_nested() {
        let mut arena = TyArena::new();
        let t_entity = entity(1);
        let t_ty = arena.intern(MirTy::TypeParam(t_entity));
        // Pointer(Pointer(T)) -> Pointer(Pointer(I64))
        let ptr_t = arena.pointer(t_ty);
        let ptr_ptr_t = arena.pointer(ptr_t);
        let i64_ty = arena.i64();

        let mut subst = SubstMap::new();
        subst.type_params.insert(t_entity, i64_ty);

        let result = substitute(&mut arena, ptr_ptr_t, &subst);
        let ptr_i64 = arena.pointer(i64_ty);
        let expected = arena.pointer(ptr_i64);
        assert_eq!(result, expected);
    }

    #[test]
    fn substitute_associated_projection() {
        let mut arena = TyArena::new();
        let t_entity = entity(1);
        let proto = entity(2);
        let assoc = entity(3);
        let t_ty = arena.intern(MirTy::TypeParam(t_entity));
        let proj = arena.intern(MirTy::AssociatedProjection {
            base: t_ty,
            protocol: proto,
            assoc_type: assoc,
        });

        let i64_ty = arena.i64();
        let string_ty = arena.str_ty();

        let mut subst = SubstMap::new();
        subst.type_params.insert(t_entity, i64_ty);
        subst.assoc_types.insert((i64_ty, proto, assoc), string_ty);

        let result = substitute(&mut arena, proj, &subst);
        assert_eq!(result, string_ty);
    }

    #[test]
    fn substitute_func_thin() {
        let mut arena = TyArena::new();
        let t_entity = entity(1);
        let t_ty = arena.intern(MirTy::TypeParam(t_entity));
        let bool_ty = arena.bool();
        let func = arena.intern(MirTy::FuncThin {
            params: vec![(t_ty, ParamConvention::Consuming)],
            ret: bool_ty,
        });
        let i64_ty = arena.i64();

        let mut subst = SubstMap::new();
        subst.type_params.insert(t_entity, i64_ty);

        let result = substitute(&mut arena, func, &subst);
        let expected = arena.intern(MirTy::FuncThin {
            params: vec![(i64_ty, ParamConvention::Consuming)],
            ret: bool_ty,
        });
        assert_eq!(result, expected);
    }
}
