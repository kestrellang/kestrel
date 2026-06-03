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
///
/// Two-pass: types first (structs, enums, protocols), then functions.
/// This ensures all TypeInfo (CopyBehavior, DropBehavior) is available
/// when function bodies are lowered — is_copy_type lookups work regardless
/// of module ordering.
pub fn lower_items(ctx: &mut LowerCtx) {
    let root = ctx.root;
    lower_types(ctx, root);
    lower_functions(ctx, root);
}

// --- Pass 1: types (structs, enums, protocols) ---

fn lower_types(ctx: &mut LowerCtx, parent: Entity) {
    let children: Vec<Entity> = ctx.world.children_of(parent).to_vec();
    for child in children {
        let Some(kind) = ctx.world.get::<NodeKind>(child).cloned() else {
            continue;
        };
        match kind {
            NodeKind::Module => lower_types(ctx, child),
            NodeKind::Struct => struct_lower::lower_struct(ctx, child),
            NodeKind::Enum => enum_lower::lower_enum(ctx, child),
            NodeKind::Protocol => protocol_lower::lower_protocol(ctx, child),
            _ => {},
        }
    }
}

// --- Pass 2: functions, extensions, statics ---

fn lower_functions(ctx: &mut LowerCtx, parent: Entity) {
    let children: Vec<Entity> = ctx.world.children_of(parent).to_vec();
    for child in children {
        let Some(kind) = ctx.world.get::<NodeKind>(child).cloned() else {
            continue;
        };
        match kind {
            NodeKind::Module => lower_functions(ctx, child),
            NodeKind::Struct | NodeKind::Enum => lower_member_functions(ctx, child),
            NodeKind::Extension => lower_member_functions(ctx, child),
            NodeKind::Function | NodeKind::Setter => {
                function_sig::lower_function_sig(ctx, child);
            },
            NodeKind::Field => {
                if ctx.world.get::<Callable>(child).is_some() {
                    function_sig::lower_function_sig(ctx, child);
                } else {
                    static_lower::lower_static(ctx, child);
                }
                lower_functions(ctx, child);
            },
            _ => {},
        }
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
                    lower_functions(ctx, child);
                }
            },
            NodeKind::Field if ctx.world.get::<Callable>(child).is_some() => {
                function_sig::lower_function_sig(ctx, child);
                lower_functions(ctx, child);
            },
            NodeKind::Field if ctx.world.get::<Static>(child).is_some() => {
                static_lower::lower_static(ctx, child);
            },
            _ => {},
        }
    }
}
