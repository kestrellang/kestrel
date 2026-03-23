//! Protocol lowering — converts ECS protocol entities into MIR ProtocolDefs.

use kestrel_ast_builder::{Callable, NodeKind, TypeParams};
use kestrel_hecs::Entity;
use kestrel_mir::{AssociatedTypeDef, ProtocolDef, ProtocolId, ProtocolMethodDef, TypeParamDef};
use kestrel_name_res::conformances::ConformingProtocols;

use crate::context::LowerCtx;
use crate::ty::{resolve_callable_types, resolve_type_annotation};

/// Lower a protocol entity into a MIR ProtocolDef.
pub fn lower_protocol(ctx: &mut LowerCtx, entity: Entity) -> ProtocolId {
    let name = ctx.register_name(entity);
    let mut def = ProtocolDef::new(entity, name);

    // Parent protocols from protocol inheritance / extension-added conformances.
    let parents = ctx.query.query(ConformingProtocols {
        entity,
        root: ctx.root,
    });
    for parent in parents {
        if parent != entity {
            ctx.register_name(parent);
            def.add_parent(parent);
        }
    }

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

    // Children: associated types and methods
    let children: Vec<Entity> = ctx.world.children_of(entity).to_vec();
    for child in children {
        let Some(kind) = ctx.world.get::<NodeKind>(child) else {
            continue;
        };

        match *kind {
            NodeKind::TypeAlias => {
                let assoc_name = ctx
                    .world
                    .get::<kestrel_ast_builder::Name>(child)
                    .map(|n| n.0.clone())
                    .unwrap_or_default();
                let mut assoc_def = AssociatedTypeDef::new(assoc_name);
                let default_ty = resolve_type_annotation(ctx, child);
                if default_ty != kestrel_mir::MirTy::Unit {
                    assoc_def = assoc_def.with_default(default_ty);
                }
                def.add_associated_type(assoc_def);
            },
            NodeKind::Function => {
                let method_name = ctx
                    .world
                    .get::<kestrel_ast_builder::Name>(child)
                    .map(|n| n.0.clone())
                    .unwrap_or_default();

                // Resolve return type
                let ret_ty = resolve_type_annotation(ctx, child);
                let mut method = ProtocolMethodDef::new(method_name, ret_ty);

                // Add type parameters for the method itself
                if let Some(tp) = ctx.world.get::<TypeParams>(child) {
                    for &tp_entity in &tp.0 {
                        ctx.register_name(tp_entity);
                        let tp_name = ctx
                            .world
                            .get::<kestrel_ast_builder::Name>(tp_entity)
                            .map(|n| n.0.clone())
                            .unwrap_or_default();
                        method
                            .type_params
                            .push(TypeParamDef::new(tp_entity, tp_name));
                    }
                }

                // Add parameters from Callable with resolved types
                if let Some(callable) = ctx.world.get::<Callable>(child) {
                    if callable.receiver.is_some() {
                        method.add_param("self", kestrel_mir::MirTy::SelfType);
                    }
                    let resolved_types = resolve_callable_types(ctx, child);
                    for (i, param) in callable.params.iter().enumerate() {
                        let param_ty = resolved_types
                            .get(i)
                            .and_then(|t| t.clone())
                            .unwrap_or(kestrel_mir::MirTy::Error);
                        method.add_param(&param.name, param_ty);
                    }
                }

                def.add_method(method);
            },
            NodeKind::Subscript => {
                // Subscripts are similar to methods — handle in a later phase
            },
            _ => {},
        }
    }

    ctx.module.add_protocol(def)
}
