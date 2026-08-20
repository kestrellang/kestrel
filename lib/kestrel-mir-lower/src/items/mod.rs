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
/// Three phases: types, `fix_drop_behaviors`, then functions. Both splits are
/// load-bearing, and the two halves of `TypeInfo` are NOT symmetric:
///
/// - **`CopyBehavior` is final after `lower_types`.** `lower_copy_behavior`
///   asks `NominalCopySemantics`, which already folds the whole type; no later
///   pass fixes it up. Pass 1 alone is enough for every `is_copy_type` /
///   `copy_behavior` lookup in a body, regardless of declaration order.
/// - **`DropBehavior` is NOT.** `lower_drop_behavior` only reports a *user*
///   `deinit`; a struct with no `deinit` but a droppable field comes out of
///   pass 1 as `DropBehavior::None` and becomes droppable only when
///   [`fix_drop_behaviors`](kestrel_mir::passes::drop_fix::fix_drop_behaviors)
///   walks the fields to a fixed point. Body lowering reads that flag through
///   `needs_drop` (`body/mod.rs` `setup_init_field_flags`), so the pass MUST
///   run between the two halves — running it only from the `Stage::DropFix`
///   slot in `passes::run_pipeline_until` put the writer strictly after the
///   reader and silently leaked every transitively-droppable init field
///   (fragility audit G1).
///
/// The pass is deliberately run TWICE (here and in `run_pipeline_until`), not
/// duplicated by accident — see the comment at the other call site. It is
/// monotone and additive (`None → StructDrop`, or push a field index that
/// isn't already present) and its own outer `loop` already re-runs it to a
/// no-change fixed point, so a second run over an unchanged module is a no-op.
pub fn lower_items(ctx: &mut LowerCtx) {
    let root = ctx.root;
    lower_types(ctx, root);
    // Decide DropBehavior BEFORE any body is lowered: bodies query it.
    kestrel_mir::passes::drop_fix::fix_drop_behaviors(&mut ctx.module);
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
            NodeKind::Function | NodeKind::Setter | NodeKind::RefAccessor => {
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
            | NodeKind::Setter
            | NodeKind::RefAccessor => {
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
