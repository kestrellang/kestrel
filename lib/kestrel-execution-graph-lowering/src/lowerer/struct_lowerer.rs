//! Struct lowering - converts semantic struct symbols to MIR struct definitions.

use kestrel_execution_graph::TypeParamOwner;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::symbol::field::FieldSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use semantic_tree::symbol::Symbol;
use std::sync::Arc;

use crate::context::LoweringContext;
use crate::name::qualified_name_for_symbol;
use crate::ty::lower_type;

/// Lower a struct definition to MIR.
///
/// This creates a MIR struct with all its fields. Methods and initializers
/// are lowered separately as top-level functions.
pub fn lower_struct(ctx: &mut LoweringContext, struct_symbol: &Arc<StructSymbol>) {
    // Generate qualified name
    let name = qualified_name_for_symbol(ctx, &(struct_symbol.clone() as _));

    // Create the struct
    let struct_id = ctx.mir.add_struct(name);

    // Register type parameters BEFORE lowering field types.
    // This ensures that type parameters like A, B are in scope when lowering fields.
    for tp in struct_symbol.type_parameters() {
        let tp_name = tp.metadata().name().value.clone();
        let tp_def =
            kestrel_execution_graph::TypeParamDef::new(tp_name, TypeParamOwner::Struct(struct_id));
        let tp_id = ctx.mir.type_params.alloc(tp_def);
        ctx.mir.structs[struct_id].type_params.push(tp_id);
        ctx.map_type_param(tp.metadata().id(), tp_id);
    }

    // Add fields (now type parameters are in scope)
    // Only include stored instance fields, not static or computed properties
    for child in struct_symbol.metadata().children() {
        if child.metadata().kind() == KestrelSymbolKind::Field
            && let Ok(field_symbol) = child.downcast_arc::<FieldSymbol>()
        {
            // Skip static fields - they're not part of the instance layout
            if field_symbol.is_static() {
                continue;
            }
            // Skip computed properties - they have getters, not storage
            if field_symbol.is_computed() {
                continue;
            }
            lower_field(ctx, struct_id, &field_symbol);
        }
    }

    // Clear type params after lowering struct fields.
    // Methods will register their own type params (including parent's) when lowered.
    ctx.clear_type_params();
}

/// Lower a field to MIR.
fn lower_field(
    ctx: &mut LoweringContext,
    struct_id: kestrel_execution_graph::Id<kestrel_execution_graph::Struct>,
    field_symbol: &Arc<FieldSymbol>,
) {
    let name = field_symbol.metadata().name().value.clone();

    // Get the resolved type from TypedBehavior (set during binding),
    // falling back to field_type() if not available
    let field_ty = field_symbol
        .metadata()
        .get_behavior::<TypedBehavior>()
        .map(|typed| typed.ty().clone())
        .unwrap_or_else(|| field_symbol.field_type().clone());

    let mir_ty = lower_type(ctx, &field_ty);

    ctx.mir.add_field(struct_id, name, mir_ty);
}
