//! Struct lowering — converts ECS struct entities into MIR StructDefs.

use kestrel_ast_builder::{Callable, NodeKind, Static, TypeParams};
use kestrel_hecs::Entity;
use kestrel_mir::item::struct_def::{FieldDef, StructDef};
use kestrel_mir::{CopyBehavior, DropBehavior, TypeInfo, TypeParamDef};
use kestrel_semantics::{ConditionalCopyableParams, CopySemantics, NominalCopySemantics};

use crate::context::LowerCtx;
use crate::ty::resolve_type_annotation;

pub fn lower_struct(ctx: &mut LowerCtx, entity: Entity) {
    let name = ctx.register_name(entity);
    let mut def = StructDef::new(entity, name);

    def.type_params = collect_type_params(ctx, entity);
    def.type_info = TypeInfo {
        copy: lower_copy_behavior(ctx, entity),
        drop: lower_drop_behavior(ctx, entity),
        layout: None,
    };
    def.conditionally_copyable = ctx.query.query(ConditionalCopyableParams {
        entity,
        root: ctx.root,
    });

    // Stored fields only — skip computed properties and statics
    for &child in ctx.world.children_of(entity) {
        if ctx.world.get::<NodeKind>(child) != Some(&NodeKind::Field) {
            continue;
        }
        if ctx.world.get::<Callable>(child).is_some() || ctx.world.get::<Static>(child).is_some() {
            continue;
        }

        let field_name = ctx
            .world
            .get::<kestrel_ast_builder::Name>(child)
            .map(|n| n.0.clone())
            .unwrap_or_default();
        let field_ty = resolve_type_annotation(ctx, child);
        def.add_field(FieldDef::new(field_name, field_ty));
    }

    ctx.module.add_struct(def);
}

pub(crate) fn lower_copy_behavior(ctx: &mut LowerCtx, entity: Entity) -> CopyBehavior {
    let info = ctx.query.query(NominalCopySemantics {
        entity,
        root: ctx.root,
    });
    match info.semantics {
        CopySemantics::Copyable => CopyBehavior::Bitwise,
        CopySemantics::Cloneable => CopyBehavior::Clone(entity),
        CopySemantics::NotCopyable => CopyBehavior::None,
    }
}

fn lower_drop_behavior(ctx: &LowerCtx, entity: Entity) -> DropBehavior {
    let deinit = find_user_deinit(ctx, entity);
    if deinit.is_some() {
        DropBehavior::StructDrop {
            deinit,
            fields: Vec::new(),
        }
    } else {
        DropBehavior::None
    }
}

pub(crate) fn find_user_deinit(ctx: &LowerCtx, entity: Entity) -> Option<Entity> {
    ctx.world.children_of(entity).iter().find(|&&child| ctx.world.get::<NodeKind>(child) == Some(&NodeKind::Deinit)).copied()
}

pub(crate) fn collect_type_params(ctx: &mut LowerCtx, entity: Entity) -> Vec<TypeParamDef> {
    let Some(type_params) = ctx.world.get::<TypeParams>(entity) else {
        return Vec::new();
    };
    type_params
        .0
        .iter()
        .map(|&tp_entity| {
            ctx.register_name(tp_entity);
            let tp_name = ctx
                .world
                .get::<kestrel_ast_builder::Name>(tp_entity)
                .map(|n| n.0.clone())
                .unwrap_or_default();
            TypeParamDef::new(tp_entity, tp_name)
        })
        .collect()
}
