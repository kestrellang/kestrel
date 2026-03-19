//! Struct lowering — converts ECS struct entities into MIR StructDefs.

use kestrel_ast_builder::{NodeKind, TypeParams};
use kestrel_hecs::Entity;
use kestrel_mir::{FieldDef, StructDef, StructId, TypeParamDef};

use crate::context::LowerCtx;
use crate::ty::resolve_type_annotation;

/// Lower a struct entity into a MIR StructDef.
pub fn lower_struct(ctx: &mut LowerCtx, entity: Entity) -> StructId {
    let name = ctx.register_name(entity);
    let mut def = StructDef::new(entity, name);

    // Type parameters
    if let Some(type_params) = ctx.world.get::<TypeParams>(entity) {
        for &tp_entity in &type_params.0 {
            ctx.register_name(tp_entity);
            let tp_name = ctx
                .world
                .get::<kestrel_ast_builder::Name>(tp_entity)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| format!("{:?}", tp_entity));
            def.type_params.push(TypeParamDef::new(tp_entity, tp_name));
        }
    }

    // Fields: children with NodeKind::Field
    for &child in ctx.world.children_of(entity) {
        let Some(kind) = ctx.world.get::<NodeKind>(child) else {
            continue;
        };
        if *kind != NodeKind::Field {
            continue;
        }

        let field_name = ctx
            .world
            .get::<kestrel_ast_builder::Name>(child)
            .map(|n| n.0.clone())
            .unwrap_or_default();

        // Resolve field type: TypeAnnotation(AstType) → HirTy → MirTy
        let field_ty = resolve_type_annotation(ctx, child);

        def.add_field(FieldDef::new(field_name, field_ty));
    }

    ctx.module.add_struct(def)
}
