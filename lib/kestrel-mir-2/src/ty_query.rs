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
                    CopyBehavior::Bitwise => {}
                    other => return other,
                }
            }
            CopyBehavior::Bitwise
        }

        MirTy::Named { entity, .. } => {
            let entity = *entity;
            for s in &module.structs {
                if s.entity == entity {
                    return s.type_info.copy.clone();
                }
            }
            for e in &module.enums {
                if e.entity == entity {
                    return e.type_info.copy.clone();
                }
            }
            CopyBehavior::Bitwise
        }

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
                        }
                        WhereConstraint::NotImplements {
                            type_param,
                            protocol,
                        } if *type_param == entity => {
                            if is_copyable_protocol(module, *protocol) {
                                return CopyBehavior::None;
                            }
                        }
                        _ => {}
                    }
                }
            }
            // Unknown at generic time — treat as non-copyable so the lowering
            // emits Move. After monomorphization the concrete type takes over.
            CopyBehavior::None
        }

        MirTy::FuncThick { .. } => CopyBehavior::None,
        MirTy::AssociatedProjection { .. } => CopyBehavior::Bitwise,
    }
}

/// Does this type need cleanup when it goes out of scope?
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
        }

        MirTy::Named { entity, .. } => {
            let entity = *entity;
            for s in &module.structs {
                if s.entity == entity {
                    return s.type_info.drop != DropBehavior::None;
                }
            }
            for e in &module.enums {
                if e.entity == entity {
                    return e.type_info.drop != DropBehavior::None;
                }
            }
            false
        }

        // Unresolved type param — conservatively needs tracking
        MirTy::TypeParam(_) => true,
        MirTy::AssociatedProjection { .. } => true,

        MirTy::FuncThick { .. } => true,
    }
}

pub fn is_cloneable_protocol(module: &MirModule, entity: Entity) -> bool {
    module
        .protocols
        .iter()
        .any(|p| p.entity == entity && p.name.ends_with("Cloneable"))
}

pub fn is_copyable_protocol(module: &MirModule, entity: Entity) -> bool {
    module
        .protocols
        .iter()
        .any(|p| p.entity == entity && p.name.ends_with("Copyable"))
}

pub fn find_cloneable_protocol(module: &MirModule) -> Option<Entity> {
    module
        .protocols
        .iter()
        .find(|p| p.name.ends_with("Cloneable"))
        .map(|p| p.entity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::ModuleBuilder;
    use crate::item::protocol::ProtocolDef;
    use crate::item::struct_def::{FieldDef, StructDef};
    use crate::item::{CopyBehavior, DropBehavior, TypeInfo};
    use crate::ty::MirTy;
    use crate::FieldIdx;

    // ---- copy_behavior tests ----

    #[test]
    fn copy_behavior_primitives_are_bitwise() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let bool_ty = m.bool();
        let module = m.finish();
        assert_eq!(
            copy_behavior(&module.ty_arena, &module, i64_ty, None),
            CopyBehavior::Bitwise
        );
        assert_eq!(
            copy_behavior(&module.ty_arena, &module, bool_ty, None),
            CopyBehavior::Bitwise
        );
    }

    #[test]
    fn copy_behavior_clone_struct() {
        let mut m = ModuleBuilder::new("test");
        let cloneable = m.fresh_entity();
        m.register_name(cloneable, "std.Cloneable");
        m.add_protocol(ProtocolDef::new(cloneable, "std.Cloneable"));

        let s_entity = m.fresh_entity();
        let s_ty = m.named(s_entity, vec![]);
        let mut def = StructDef::new(s_entity, "MyStr");
        def.type_info.copy = CopyBehavior::Clone(cloneable);
        m.add_struct(def);

        let module = m.finish();
        assert_eq!(
            copy_behavior(&module.ty_arena, &module, s_ty, None),
            CopyBehavior::Clone(cloneable)
        );
    }

    #[test]
    fn copy_behavior_type_param_with_cloneable() {
        let mut m = ModuleBuilder::new("test");
        let cloneable = m.fresh_entity();
        m.register_name(cloneable, "std.Cloneable");
        m.add_protocol(ProtocolDef::new(cloneable, "std.Cloneable"));
        let t_entity = m.fresh_entity();
        let t_ty = m.ty(MirTy::TypeParam(t_entity));
        let module = m.finish();

        let mut wc = WhereClause::new();
        wc.add_constraint(WhereConstraint::implements(t_entity, cloneable));
        assert_eq!(
            copy_behavior(&module.ty_arena, &module, t_ty, Some(&wc)),
            CopyBehavior::Clone(cloneable)
        );
    }

    #[test]
    fn copy_behavior_type_param_not_copyable() {
        let mut m = ModuleBuilder::new("test");
        let copyable = m.fresh_entity();
        m.register_name(copyable, "std.Copyable");
        m.add_protocol(ProtocolDef::new(copyable, "std.Copyable"));
        let t_entity = m.fresh_entity();
        let t_ty = m.ty(MirTy::TypeParam(t_entity));
        let module = m.finish();

        let mut wc = WhereClause::new();
        wc.add_constraint(WhereConstraint::not_implements(t_entity, copyable));
        assert_eq!(
            copy_behavior(&module.ty_arena, &module, t_ty, Some(&wc)),
            CopyBehavior::None
        );
    }

    #[test]
    fn copy_behavior_type_param_default_none() {
        let mut m = ModuleBuilder::new("test");
        let t_entity = m.fresh_entity();
        let t_ty = m.ty(MirTy::TypeParam(t_entity));
        let module = m.finish();
        assert_eq!(
            copy_behavior(&module.ty_arena, &module, t_ty, None),
            CopyBehavior::None
        );
    }

    // ---- needs_drop tests ----

    #[test]
    fn needs_drop_primitives_false() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let bool_ty = m.bool();
        let ptr_ty = m.pointer(i64_ty);
        let module = m.finish();
        assert!(!needs_drop(&module.ty_arena, &module, i64_ty));
        assert!(!needs_drop(&module.ty_arena, &module, bool_ty));
        assert!(!needs_drop(&module.ty_arena, &module, ptr_ty));
    }

    #[test]
    fn needs_drop_struct_with_drop() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let s_entity = m.fresh_entity();
        let s_ty = m.named(s_entity, vec![]);
        let mut def = StructDef::new(s_entity, "String");
        def.add_field(FieldDef::new("data", i64_ty));
        def.type_info = TypeInfo {
            copy: CopyBehavior::None,
            drop: DropBehavior::StructDrop {
                deinit: None,
                fields: vec![FieldIdx::new(0)],
            },
            layout: None,
        };
        m.add_struct(def);
        let module = m.finish();
        assert!(needs_drop(&module.ty_arena, &module, s_ty));
    }

    #[test]
    fn needs_drop_struct_without_drop() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let s_entity = m.fresh_entity();
        let s_ty = m.named(s_entity, vec![]);
        let mut def = StructDef::new(s_entity, "Point");
        def.add_field(FieldDef::new("x", i64_ty));
        def.type_info = TypeInfo::bitwise();
        m.add_struct(def);
        let module = m.finish();
        assert!(!needs_drop(&module.ty_arena, &module, s_ty));
    }

    #[test]
    fn needs_drop_type_param_conservative() {
        let mut m = ModuleBuilder::new("test");
        let t_entity = m.fresh_entity();
        let t_ty = m.ty(MirTy::TypeParam(t_entity));
        let module = m.finish();
        // Unresolved type param → conservatively true
        assert!(needs_drop(&module.ty_arena, &module, t_ty));
    }

    #[test]
    fn needs_drop_tuple_with_droppable_element() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let s_entity = m.fresh_entity();
        let s_ty = m.named(s_entity, vec![]);
        let mut def = StructDef::new(s_entity, "String");
        def.type_info.drop = DropBehavior::StructDrop {
            deinit: None,
            fields: vec![],
        };
        m.add_struct(def);
        let tup_ty = m.ty(MirTy::Tuple(vec![i64_ty, s_ty]));
        let module = m.finish();
        assert!(needs_drop(&module.ty_arena, &module, tup_ty));
    }

    #[test]
    fn needs_drop_tuple_all_bitwise() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let bool_ty = m.bool();
        let tup_ty = m.ty(MirTy::Tuple(vec![i64_ty, bool_ty]));
        let module = m.finish();
        assert!(!needs_drop(&module.ty_arena, &module, tup_ty));
    }

    #[test]
    fn needs_drop_func_thick_true() {
        let mut m = ModuleBuilder::new("test");
        let i64_ty = m.i64();
        let unit_ty = m.unit();
        let thick_ty = m.ty(MirTy::FuncThick {
            params: vec![(i64_ty, crate::ParamConvention::Consuming)],
            ret: unit_ty,
        });
        let module = m.finish();
        assert!(needs_drop(&module.ty_arena, &module, thick_ty));
    }
}
