//! Item dispatch — walk entity tree, route by NodeKind.

pub mod enum_lower;
pub mod function_sig;
pub mod protocol_lower;
pub mod static_lower;
pub mod struct_lower;
pub mod witness_lower;

use kestrel_ast_builder::{Callable, NodeKind, Static};
use kestrel_hecs::Entity;

use crate::context::LowerCtx;

/// Walk all entities under the root and lower declarations to MIR items.
pub fn lower_items(ctx: &mut LowerCtx) {
    let root = ctx.root;
    lower_children(ctx, root);
}

fn lower_children(ctx: &mut LowerCtx, parent: Entity) {
    let children: Vec<Entity> = ctx.world.children_of(parent).to_vec();
    for child in children {
        lower_entity(ctx, child);
    }
}

fn lower_entity(ctx: &mut LowerCtx, entity: Entity) {
    let Some(kind) = ctx.world.get::<NodeKind>(entity).cloned() else {
        return;
    };

    match kind {
        NodeKind::Module => lower_children(ctx, entity),
        NodeKind::Struct => {
            struct_lower::lower_struct(ctx, entity);
            lower_member_functions(ctx, entity);
        }
        NodeKind::Enum => {
            enum_lower::lower_enum(ctx, entity);
            lower_member_functions(ctx, entity);
        }
        NodeKind::Protocol => {
            protocol_lower::lower_protocol(ctx, entity);
        }
        NodeKind::Extension => {
            lower_member_functions(ctx, entity);
        }
        NodeKind::Function | NodeKind::Setter => {
            function_sig::lower_function_sig(ctx, entity);
        }
        NodeKind::Field => {
            if ctx.world.get::<Callable>(entity).is_some() {
                function_sig::lower_function_sig(ctx, entity);
            } else {
                static_lower::lower_static(ctx, entity);
            }
            lower_children(ctx, entity);
        }
        _ => {}
    }
}

fn lower_member_functions(ctx: &mut LowerCtx, parent: Entity) {
    let children: Vec<Entity> = ctx.world.children_of(parent).to_vec();
    for child in children {
        let Some(kind) = ctx.world.get::<NodeKind>(child).cloned() else {
            continue;
        };
        match kind {
            NodeKind::Function
            | NodeKind::Initializer
            | NodeKind::Deinit
            | NodeKind::Subscript
            | NodeKind::Setter => {
                function_sig::lower_function_sig(ctx, child);
                if matches!(kind, NodeKind::Subscript) {
                    lower_children(ctx, child);
                }
            }
            NodeKind::Field if ctx.world.get::<Callable>(child).is_some() => {
                function_sig::lower_function_sig(ctx, child);
                lower_children(ctx, child);
            }
            NodeKind::Field if ctx.world.get::<Static>(child).is_some() => {
                static_lower::lower_static(ctx, child);
            }
            _ => {}
        }
    }
}
