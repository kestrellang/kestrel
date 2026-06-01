//! Enum lowering — converts ECS enum entities into MIR EnumDefs.
//!
//! Unlike kestrel-mir (old), kestrel-mir-3 stores payload fields inline on
//! EnumCaseDef rather than referencing a separate payload struct.

use kestrel_ast_builder::{Callable, NodeKind};
use kestrel_hecs::Entity;
use kestrel_mir_3::item::enum_def::{EnumCaseDef, EnumDef};
use kestrel_mir_3::item::struct_def::FieldDef;
use kestrel_mir_3::{DropBehavior, TypeInfo};

use crate::context::LowerCtx;
use crate::items::struct_lower::{collect_type_params, find_user_deinit, lower_copy_behavior};
use crate::ty::resolve_callable_types;

pub fn lower_enum(ctx: &mut LowerCtx, entity: Entity) {
    let name = ctx.register_name(entity);
    let mut def = EnumDef::new(entity, &name);

    def.type_params = collect_type_params(ctx, entity);
    def.type_info = TypeInfo {
        copy: lower_copy_behavior(ctx, entity),
        drop: lower_enum_drop_behavior(ctx, entity),
        layout: None,
    };
    def.conditionally_copyable = ctx
        .query
        .query(kestrel_semantics::ConditionalCopyableParams {
            entity,
            root: ctx.root,
        });

    let children: Vec<Entity> = ctx.world.children_of(entity).to_vec();
    for child in children {
        if ctx.world.get::<NodeKind>(child) != Some(&NodeKind::EnumCase) {
            continue;
        }

        let case_name = ctx
            .world
            .get::<kestrel_ast_builder::Name>(child)
            .map(|n| n.0.clone())
            .unwrap_or_default();

        let mut payload_fields = Vec::new();
        if let Some(callable) = ctx.world.get::<Callable>(child) {
            let resolved_types = resolve_callable_types(ctx, child);
            for (i, param) in callable.params.iter().enumerate() {
                let field_name = if param.name.is_empty() {
                    format!("{}", i)
                } else {
                    param.name.clone()
                };
                let field_ty = resolved_types
                    .get(i)
                    .and_then(|t| *t)
                    .unwrap_or_else(|| ctx.module.ty_arena.error());
                payload_fields.push(FieldDef::new(field_name, field_ty));
            }
        }

        let discriminant = def.cases.len() as u32;
        def.add_case(EnumCaseDef::with_payload(
            case_name,
            discriminant,
            payload_fields,
        ));
    }

    ctx.module.add_enum(def);
}

fn lower_enum_drop_behavior(ctx: &LowerCtx, entity: Entity) -> DropBehavior {
    let deinit = find_user_deinit(ctx, entity);
    if deinit.is_some() {
        DropBehavior::EnumDrop {
            deinit,
            variants: Vec::new(),
        }
    } else {
        DropBehavior::None
    }
}
