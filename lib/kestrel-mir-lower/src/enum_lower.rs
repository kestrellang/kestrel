//! Enum lowering — converts ECS enum entities into MIR EnumDefs.
//!
//! Each enum case gets a corresponding payload struct in the MIR.

use kestrel_ast_builder::{Callable, NodeKind, TypeParams};
use kestrel_hecs::Entity;
use kestrel_mir::{DeinitBehavior, EnumCaseDef, EnumDef, EnumId, FieldDef, StructDef, TypeParamDef};

use crate::context::LowerCtx;
use crate::ty::resolve_callable_types;

/// Lower an enum entity into a MIR EnumDef with payload structs for each case.
pub fn lower_enum(ctx: &mut LowerCtx, entity: Entity) -> EnumId {
    let name = ctx.register_name(entity);
    let mut def = EnumDef::new(entity, &name);

    // Type parameters
    let type_params = collect_type_params(ctx, entity);
    def.type_params = type_params.clone();

    // CopyBehavior from `NominalCopySemantics` (same query covers structs +
    // enums). DeinitBehavior.user_method comes from the user-defined
    // `deinit { ... }` child if any; field drops are derived structurally
    // at drop-expand time from `CopyBehavior::None` payload field types.
    def.copy_behavior = crate::struct_lower::lower_copy_behavior(ctx, entity);
    def.deinit_behavior = DeinitBehavior {
        user_method: crate::struct_lower::find_user_deinit(ctx, entity),
        field_drops: Vec::new(),
    };

    // Cases: children with NodeKind::EnumCase
    let children: Vec<Entity> = ctx.world.children_of(entity).to_vec();
    for child in children {
        let Some(kind) = ctx.world.get::<NodeKind>(child) else {
            continue;
        };
        if *kind != NodeKind::EnumCase {
            continue;
        }

        let case_name = ctx
            .world
            .get::<kestrel_ast_builder::Name>(child)
            .map(|n| n.0.clone())
            .unwrap_or_default();

        // Create payload struct: EnumName.cases.CaseName
        let struct_name = format!("{}.cases.{}", name, case_name);
        let mut payload = StructDef::new(child, &struct_name);

        // Inherit enum's type parameters
        payload.type_params = type_params.clone();

        // If the case has associated values (Callable), add fields from params
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
                    .and_then(|t| t.clone())
                    .unwrap_or(kestrel_mir::MirTy::Error);
                payload.add_field(FieldDef::new(field_name, field_ty));
            }
        }

        let payload_id = ctx.module.add_struct(payload);
        let discriminant = def.cases.len() as u32;
        def.add_case(EnumCaseDef::new(case_name, discriminant, payload_id));
    }

    ctx.module.add_enum(def)
}

/// Collect type parameters from an entity's TypeParams component.
fn collect_type_params(ctx: &mut LowerCtx, entity: Entity) -> Vec<TypeParamDef> {
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
                .unwrap_or_else(|| format!("{:?}", tp_entity));
            TypeParamDef::new(tp_entity, tp_name)
        })
        .collect()
}
