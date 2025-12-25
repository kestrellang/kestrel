//! Struct lowering - converts semantic struct symbols to MIR struct definitions.

use kestrel_semantic_tree::symbol::field::FieldSymbol;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
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

    // TODO: Handle type parameters (generics)
    // For now, we skip generic structs or emit a warning
    if struct_symbol.is_generic() {
        // Still create the struct, but type parameters won't be properly handled
        // The type conversion will emit warnings for type parameter usage
    }

    // Add fields
    for child in struct_symbol.metadata().children() {
        if child.metadata().kind() == KestrelSymbolKind::Field {
            if let Ok(field_symbol) = child.downcast_arc::<FieldSymbol>() {
                lower_field(ctx, struct_id, &field_symbol);
            }
        }
    }
}

/// Lower a field to MIR.
fn lower_field(
    ctx: &mut LoweringContext,
    struct_id: kestrel_execution_graph::Id<kestrel_execution_graph::Struct>,
    field_symbol: &Arc<FieldSymbol>,
) {
    let name = field_symbol.metadata().name().value.clone();

    let field_ty = field_symbol.field_type();
    let mir_ty = lower_type(ctx, field_ty);

    ctx.mir.add_field(struct_id, name, mir_ty);
}
