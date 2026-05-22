//! Protocol lowering — converts ECS protocol entities into MIR ProtocolDefs.

use kestrel_ast_builder::{Callable, NodeKind, TypeParams};
use kestrel_hecs::Entity;
use kestrel_mir_2::item::protocol::{AssociatedTypeDef, ProtocolDef, ProtocolMethodDef};
use kestrel_mir_2::{MirTy, TypeParamDef};
use kestrel_name_res::conformances::ConformingProtocols;

use crate::context::LowerCtx;
use crate::items::struct_lower::collect_type_params;
use crate::ty::{resolve_callable_return_type, resolve_callable_types, resolve_type_annotation};

pub fn lower_protocol(ctx: &mut LowerCtx, entity: Entity) {
    let name = ctx.register_name(entity);
    let mut def = ProtocolDef::new(entity, name);

    // Parent protocols
    let parents = ctx.query.query(ConformingProtocols {
        entity,
        root: ctx.root,
    });
    for parent in parents {
        if parent != entity {
            ctx.register_name(parent);
            def.parent_protocols.push(parent);
        }
    }

    def.type_params = collect_type_params(ctx, entity);

    // Associated types and methods
    let children: Vec<Entity> = ctx.world.children_of(entity).to_vec();
    for child in children {
        let Some(kind) = ctx.world.get::<NodeKind>(child) else {
            continue;
        };
        match *kind {
            NodeKind::TypeAlias => {
                ctx.register_name(child);
                let assoc_name = ctx
                    .world
                    .get::<kestrel_ast_builder::Name>(child)
                    .map(|n| n.0.clone())
                    .unwrap_or_default();
                let mut assoc_def = AssociatedTypeDef::new(child, assoc_name);
                let default_ty = resolve_type_annotation(ctx, child);
                let is_unit = ctx.module.ty_arena.get(default_ty) == &MirTy::Tuple(vec![]);
                if !is_unit {
                    assoc_def.default = Some(default_ty);
                }
                def.associated_types.push(assoc_def);
            }
            NodeKind::Function => {
                let method_name = ctx
                    .world
                    .get::<kestrel_ast_builder::Name>(child)
                    .map(|n| n.0.clone())
                    .unwrap_or_default();
                let ret_ty = resolve_callable_return_type(ctx, child);

                let mut params = Vec::new();
                let mut method_type_params = Vec::new();

                if let Some(tp) = ctx.world.get::<TypeParams>(child) {
                    for &tp_entity in &tp.0 {
                        ctx.register_name(tp_entity);
                        let tp_name = ctx
                            .world
                            .get::<kestrel_ast_builder::Name>(tp_entity)
                            .map(|n| n.0.clone())
                            .unwrap_or_default();
                        method_type_params.push(TypeParamDef::new(tp_entity, tp_name));
                    }
                }

                if let Some(callable) = ctx.world.get::<Callable>(child) {
                    if callable.receiver.is_some() {
                        let self_ty = crate::ty::build_self_type(ctx, entity);
                        params.push(("self".to_string(), self_ty));
                    }
                    let resolved_types = resolve_callable_types(ctx, child);
                    for (i, param) in callable.params.iter().enumerate() {
                        let param_ty = resolved_types
                            .get(i)
                            .and_then(|t| *t)
                            .unwrap_or_else(|| ctx.module.ty_arena.error());
                        params.push((param.name.clone(), param_ty));
                    }
                }

                let mut method_def = ProtocolMethodDef::new(method_name, params, ret_ty);
                method_def.type_params = method_type_params;
                def.methods.push(method_def);
            }
            NodeKind::Subscript => {
                // Subscripts handled in a later phase
            }
            _ => {}
        }
    }

    ctx.module.add_protocol(def);
}
