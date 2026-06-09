use std::collections::HashMap;

use kestrel_hecs::Entity;

use crate::ParamConvention;
use crate::ty::{MirTy, TyArena, TyId};

#[derive(Debug, Clone, Default)]
pub struct SubstMap {
    pub type_params: HashMap<Entity, TyId>,
    pub assoc_types: HashMap<(TyId, Entity, Entity), TyId>,
}

impl SubstMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.type_params.is_empty() && self.assoc_types.is_empty()
    }
}

pub fn substitute(arena: &mut TyArena, ty: TyId, subst: &SubstMap) -> TyId {
    if subst.is_empty() {
        return ty;
    }

    let mir_ty = arena.get(ty).clone();
    match mir_ty {
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
        },

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
        },

        MirTy::Pointer(pointee) => {
            let sub_pointee = substitute(arena, pointee, subst);
            if sub_pointee != pointee {
                arena.pointer(sub_pointee)
            } else {
                ty
            }
        },

        MirTy::Ref { pointee, mutating } => {
            let sub_pointee = substitute(arena, pointee, subst);
            if sub_pointee != pointee {
                arena.ref_ty(sub_pointee, mutating)
            } else {
                ty
            }
        },

        MirTy::Tuple(elems) => {
            let sub_elems: Vec<_> = elems.iter().map(|&e| substitute(arena, e, subst)).collect();
            if sub_elems != elems {
                arena.tuple(sub_elems)
            } else {
                ty
            }
        },

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
        },

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
        },

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
        },
    }
}
