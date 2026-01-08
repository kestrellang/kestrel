//! Enum lowering - converts semantic enum symbols to MIR enum definitions.

use kestrel_execution_graph::TypeParamOwner;
use kestrel_semantic_tree::symbol::enum_case::EnumCaseSymbol;
use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;
use semantic_tree::symbol::Symbol;
use std::sync::Arc;

use crate::context::LoweringContext;
use crate::name::qualified_name_for_symbol;

/// Lower an enum definition to MIR.
///
/// This creates a MIR enum with all its cases. Each case gets a struct
/// to hold its payload (if any). Methods are lowered separately.
pub fn lower_enum(ctx: &mut LoweringContext, enum_symbol: &Arc<EnumSymbol>) {
    // Generate qualified name for the enum
    let name = qualified_name_for_symbol(ctx, &(enum_symbol.clone() as _));

    // Create the enum in MIR
    let enum_id = ctx.mir.add_enum(name);

    // Register type parameters so they can be referenced when lowering case payloads
    for type_param in enum_symbol.type_parameters() {
        let param_name = type_param.metadata().name().value.clone();
        let mir_type_param = ctx
            .mir
            .type_params
            .alloc(kestrel_execution_graph::TypeParamDef::new(
                param_name,
                TypeParamOwner::Enum(enum_id),
            ));
        ctx.map_type_param(type_param.metadata().id(), mir_type_param);

        // Also add to the enum's type_params list
        ctx.mir.enums[enum_id].type_params.push(mir_type_param);
    }

    // Add each case
    for case_symbol in enum_symbol.cases() {
        lower_enum_case(ctx, enum_id, &case_symbol, enum_symbol);
    }

    // Clear type param mapping when done
    ctx.clear_type_params();
}

/// Lower an enum case to MIR.
fn lower_enum_case(
    ctx: &mut LoweringContext,
    enum_id: kestrel_execution_graph::Id<kestrel_execution_graph::Enum>,
    case_symbol: &Arc<EnumCaseSymbol>,
    enum_symbol: &Arc<EnumSymbol>,
) {
    let case_name = case_symbol.metadata().name().value.clone();

    // Create a struct name for this case's payload
    // Format: Module.EnumName."cases".CaseName
    let enum_name = qualified_name_for_symbol(ctx, &(enum_symbol.clone() as _));
    let enum_name_data = ctx.mir.name(enum_name);
    let mut case_struct_parts: Vec<String> = enum_name_data.segments.clone();
    case_struct_parts.push("cases".to_string());
    case_struct_parts.push(case_name.clone());

    let case_struct_name = ctx
        .mir
        .intern_name(kestrel_execution_graph::QualifiedNameData::new(
            case_struct_parts,
        ));

    // Add the case to the enum
    let case_id = ctx.mir.add_enum_case(enum_id, &case_name, case_struct_name);

    // Get the enum's type parameters to copy to the case struct
    let enum_type_params = ctx.mir.enums[enum_id].type_params.clone();

    // If the case has associated values, create a struct for the payload
    if case_symbol.has_associated_values() {
        if let Some(callable) = case_symbol.callable_behavior() {
            // Create struct for payload
            let struct_id = ctx.mir.add_struct(case_struct_name);

            // Copy the enum's type parameters to the case struct
            ctx.mir.structs[struct_id].type_params = enum_type_params;

            // Add fields from the callable's parameters
            for param in callable.parameters() {
                // Use external label if present, otherwise use internal bind name
                let param_name = param
                    .external_label()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| param.internal_name().to_string());
                let param_ty = crate::ty::lower_type(ctx, &param.ty);
                ctx.mir.add_field(struct_id, param_name, param_ty);
            }

            // Link the case to its struct
            ctx.mir.enum_cases[case_id].struct_def = Some(struct_id);
        }
    } else {
        // Simple case with no payload - still create an empty struct
        let struct_id = ctx.mir.add_struct(case_struct_name);
        // Copy the enum's type parameters to the case struct
        ctx.mir.structs[struct_id].type_params = enum_type_params;
        ctx.mir.enum_cases[case_id].struct_def = Some(struct_id);
    }
}
