//! Item dispatch — walk entity tree, route by NodeKind.

use kestrel_ast_builder::{Callable, NodeKind, Static};
use kestrel_hecs::Entity;

use crate::context::LowerCtx;
use crate::enum_lower::lower_enum;
use crate::function_lower::lower_function_sig;
use crate::protocol_lower::lower_protocol;
use crate::static_lower::lower_static;
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
        NodeKind::Function | NodeKind::Setter => {
            // Top-level function (not inside a type) or a setter accessor
            // that reached lower_entity via its parent Field's children.
            lower_function_sig(ctx, entity);
        },
        NodeKind::Field => {
            // Module-level field: a global variable or constant.
            // Computed globals (with `Callable`) lower as functions —
            // `field.rs` gives them no receiver when the parent is a Module.
            // Stored globals lower as statics.
            if ctx.world.get::<Callable>(entity).is_some() {
                lower_function_sig(ctx, entity);
            } else {
                lower_static(ctx, entity);
            }
            // A computed global may own a Setter child — recurse so it gets
            // lowered as its own function.
            lower_children(ctx, entity);
        },
        // Enum cases, type params, etc. are handled by their parent's lowering
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
            NodeKind::Function
            | NodeKind::Initializer
            | NodeKind::Deinit
            | NodeKind::Subscript
            | NodeKind::Setter => {
                lower_function_sig(ctx, child);
                // Subscripts may own a Setter child — recurse to lower it.
                if matches!(kind, NodeKind::Subscript) {
                    lower_children(ctx, child);
                }
            },
            // Computed properties (fields with a getter body) are lowered as methods.
            // They may also own a Setter child — recurse to lower it.
            NodeKind::Field if ctx.world.get::<Callable>(child).is_some() => {
                lower_function_sig(ctx, child);
                lower_children(ctx, child);
            },
            // Static stored fields (e.g. `static var _s: Int64 = 5`) become
            // globals. Instance stored fields are handled by struct_lower.
            NodeKind::Field if ctx.world.get::<Static>(child).is_some() => {
                lower_static(ctx, child);
            },
            _ => {},
        }
    }
}
