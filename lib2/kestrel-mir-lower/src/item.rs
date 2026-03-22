//! Item dispatch — walk entity tree, route by NodeKind.

use kestrel_ast_builder::{Callable, NodeKind};
use kestrel_hecs::Entity;

use crate::context::LowerCtx;
use crate::enum_lower::lower_enum;
use crate::function_lower::lower_function_sig;
use crate::protocol_lower::lower_protocol;
use crate::struct_lower::lower_struct;

/// Walk all entities under the root and lower declarations to MIR items.
pub fn lower_items(ctx: &mut LowerCtx) {
    let root = ctx.root;
    lower_children(ctx, root);
}

/// Recursively lower all children of an entity.
fn lower_children(ctx: &mut LowerCtx, parent: Entity) {
    // Collect children to avoid borrowing issues
    let children: Vec<Entity> = ctx.world.children_of(parent).to_vec();

    for child in children {
        lower_entity(ctx, child);
    }
}

/// Lower a single entity based on its NodeKind.
fn lower_entity(ctx: &mut LowerCtx, entity: Entity) {
    let Some(kind) = ctx.world.get::<NodeKind>(entity).cloned() else {
        return;
    };

    match kind {
        NodeKind::Module => {
            // Recurse into module children
            lower_children(ctx, entity);
        },
        NodeKind::Struct => {
            lower_struct(ctx, entity);
            // Also lower methods/inits/deinits inside the struct
            lower_member_functions(ctx, entity);
        },
        NodeKind::Enum => {
            lower_enum(ctx, entity);
            // Also lower methods inside the enum
            lower_member_functions(ctx, entity);
        },
        NodeKind::Protocol => {
            lower_protocol(ctx, entity);
            // Protocol methods are part of the ProtocolDef, not separate functions
        },
        NodeKind::Extension => {
            // Lower methods defined in extensions
            lower_member_functions(ctx, entity);
        },
        NodeKind::Function => {
            // Top-level function (not inside a type)
            lower_function_sig(ctx, entity);
        },
        // Fields, enum cases, type params, etc. are handled by their parent's lowering
        _ => {},
    }
}

/// Lower function/initializer/deinit children of a type entity.
fn lower_member_functions(ctx: &mut LowerCtx, parent: Entity) {
    let children: Vec<Entity> = ctx.world.children_of(parent).to_vec();

    for child in children {
        let Some(kind) = ctx.world.get::<NodeKind>(child).cloned() else {
            continue;
        };
        match kind {
            NodeKind::Function | NodeKind::Initializer | NodeKind::Deinit | NodeKind::Subscript => {
                lower_function_sig(ctx, child);
            },
            // Computed properties (fields with a getter body) are lowered as methods
            NodeKind::Field if ctx.world.get::<Callable>(child).is_some() => {
                lower_function_sig(ctx, child);
            },
            _ => {},
        }
    }
}
