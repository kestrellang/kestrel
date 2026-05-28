use kestrel_hecs::Entity;

use crate::item::function::{WhereClause, WhereConstraint};
use crate::ty::{MirTy, TyArena};
use crate::{CopyBehavior, DropBehavior, MirModule, TyId};

pub fn copy_behavior(
    arena: &TyArena,
    module: &MirModule,
    ty: TyId,
    where_clause: Option<&WhereClause>,
) -> CopyBehavior {
    match arena.get(ty) {
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
        | MirTy::FuncThin { .. }
        | MirTy::Error => CopyBehavior::Bitwise,

        MirTy::Tuple(elems) => {
            let elems = elems.clone();
            for &elem in &elems {
                match copy_behavior(arena, module, elem, where_clause) {
                    CopyBehavior::Bitwise => {},
                    other => return other,
                }
            }
            CopyBehavior::Bitwise
        },

        MirTy::Named { entity, .. } => {
            let entity = *entity;
            if let Some(s) = module.structs.get(&entity) {
                return s.type_info.copy.clone();
            }
            if let Some(e) = module.enums.get(&entity) {
                return e.type_info.copy.clone();
            }
            CopyBehavior::Bitwise
        },

        MirTy::TypeParam(entity) => {
            let entity = *entity;
            if let Some(wc) = where_clause {
                for constraint in &wc.constraints {
                    match constraint {
                        WhereConstraint::Implements {
                            type_param,
                            protocol,
                            ..
                        } if *type_param == entity => {
                            if is_cloneable_protocol(module, *protocol) {
                                return CopyBehavior::Clone(*protocol);
                            }
                            if is_copyable_protocol(module, *protocol) {
                                return CopyBehavior::Bitwise;
                            }
                        },
                        WhereConstraint::NotImplements {
                            type_param,
                            protocol,
                        } if *type_param == entity => {
                            if is_copyable_protocol(module, *protocol) {
                                return CopyBehavior::None;
                            }
                        },
                        _ => {},
                    }
                }
            }
            CopyBehavior::Bitwise
        },

        MirTy::FuncThick { .. } => CopyBehavior::None,
        MirTy::AssociatedProjection { .. } => CopyBehavior::Bitwise,
    }
}

pub fn needs_drop(arena: &TyArena, module: &MirModule, ty: TyId) -> bool {
    match arena.get(ty) {
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
        | MirTy::FuncThin { .. }
        | MirTy::Error => false,

        MirTy::Tuple(elems) => {
            let elems = elems.clone();
            elems.iter().any(|&e| needs_drop(arena, module, e))
        },

        MirTy::Named { entity, .. } => {
            let entity = *entity;
            if let Some(s) = module.structs.get(&entity) {
                return s.type_info.drop != DropBehavior::None;
            }
            if let Some(e) = module.enums.get(&entity) {
                return e.type_info.drop != DropBehavior::None;
            }
            false
        },

        MirTy::TypeParam(_) => true,
        MirTy::AssociatedProjection { .. } => true,
        MirTy::FuncThick { .. } => true,
    }
}

pub fn is_cloneable_protocol(module: &MirModule, entity: Entity) -> bool {
    module
        .protocols
        .get(&entity)
        .is_some_and(|p| p.name.ends_with("Cloneable"))
}

pub fn is_copyable_protocol(module: &MirModule, entity: Entity) -> bool {
    module
        .protocols
        .get(&entity)
        .is_some_and(|p| p.name.ends_with("Copyable"))
}

pub fn find_cloneable_protocol(module: &MirModule) -> Option<Entity> {
    module
        .protocols
        .values()
        .find(|p| p.name.ends_with("Cloneable"))
        .map(|p| p.entity)
}
