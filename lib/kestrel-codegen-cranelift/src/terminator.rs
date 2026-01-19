//! Terminator compilation.

use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::monomorphize::Substitution;
use crate::place::compile_place_read;
use crate::rvalue::compile_value;

use kestrel_execution_graph::{
    Block, FunctionDef, Id, Local, MirTy, PlaceKind, Terminator, TerminatorKind, Value,
};

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::types as cl_types;
use cranelift_codegen::ir::{InstBuilder, MemFlags};
use cranelift_frontend::{FunctionBuilder, Variable};

use std::collections::HashMap;

/// Compile a block terminator.
pub fn compile_terminator(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    terminator: &Terminator,
    builder: &mut FunctionBuilder<'_>,
    block_map: &HashMap<Id<Block>, cranelift_codegen::ir::Block>,
    local_map: &HashMap<Id<Local>, Variable>,
    is_main: bool,
) -> Result<(), CodegenError> {
    match &terminator.kind {
        TerminatorKind::Return(value) => {
            let ret_ty = ctx.mir.ty(func_def.ret);
            if matches!(ret_ty, kestrel_execution_graph::MirTy::Unit) {
                if is_main {
                    // main() must return 0 for success exit code
                    let zero = builder.ins().iconst(cl_types::I64, 0);
                    builder.ins().return_(&[zero]);
                } else {
                    builder.ins().return_(&[]);
                }
            } else {
                // Check if we're trying to return unit in a non-unit function
                // This happens in unreachable code paths (e.g., after a loop with only returns)
                let is_unit_value = matches!(
                    value,
                    Value::Immediate(kestrel_execution_graph::Immediate {
                        kind: kestrel_execution_graph::ImmediateKind::Unit,
                        ..
                    })
                );
                if is_unit_value {
                    // This is dead code - emit trap
                    builder
                        .ins()
                        .trap(cranelift_codegen::ir::TrapCode::unwrap_user(1));
                } else {
                    let val = compile_value(ctx, func_def, subst, value, builder, local_map)?;
                    builder.ins().return_(&[val]);
                }
            }
        }

        TerminatorKind::Jump(target) => {
            let cl_block = block_map
                .get(target)
                .ok_or_else(|| CodegenError::Unsupported("unknown jump target".to_string()))?;
            builder.ins().jump(*cl_block, &[]);
        }

        TerminatorKind::Branch {
            condition,
            then_block,
            else_block,
        } => {
            let cond = compile_value(ctx, func_def, subst, condition, builder, local_map)?;
            let then_cl = block_map
                .get(then_block)
                .ok_or_else(|| CodegenError::Unsupported("unknown then block".to_string()))?;
            let else_cl = block_map
                .get(else_block)
                .ok_or_else(|| CodegenError::Unsupported("unknown else block".to_string()))?;
            builder.ins().brif(cond, *then_cl, &[], *else_cl, &[]);
        }

        TerminatorKind::Switch {
            discriminant,
            cases,
        } => {
            // Load the discriminant value from the enum
            // The discriminant is stored at offset 0 as an i32
            let enum_ptr = compile_place_read(ctx, discriminant, builder, local_map, subst)?;
            let discr_val = builder
                .ins()
                .load(cl_types::I32, MemFlags::new(), enum_ptr, 0);

            // Get the enum type to look up case discriminants
            let enum_id = get_enum_id_from_place(ctx, discriminant)?;
            let enum_def = ctx.mir.enum_def(enum_id);

            // Build a chain of brif instructions for each case
            // This is simpler than br_table and works for any number of cases
            if cases.is_empty() {
                // No cases - emit unreachable
                builder
                    .ins()
                    .trap(cranelift_codegen::ir::TrapCode::unwrap_user(1));
            } else {
                // For each case except the last, compare and branch
                for (i, (case_name, target_block)) in cases.iter().enumerate() {
                    let target_cl = block_map.get(target_block).ok_or_else(|| {
                        CodegenError::Unsupported(format!("unknown switch target: {}", case_name))
                    })?;

                    // Handle wildcard case "_" - this is the default/fallback
                    if case_name == "_" {
                        // Wildcard matches everything - just jump
                        builder.ins().jump(*target_cl, &[]);
                        break;
                    }

                    // Look up the discriminant value for this case
                    let case_id = enum_def.case_by_name(case_name).ok_or_else(|| {
                        CodegenError::Unsupported(format!("enum case not found: {}", case_name))
                    })?;
                    let case_def = &ctx.mir.enum_cases[case_id];
                    let expected_discr = case_def.discriminant as i64;

                    if i == cases.len() - 1 {
                        // Last case - just jump unconditionally (exhaustive match)
                        builder.ins().jump(*target_cl, &[]);
                    } else {
                        // Compare discriminant and branch
                        let cmp = builder
                            .ins()
                            .icmp_imm(IntCC::Equal, discr_val, expected_discr);

                        // Create a fallthrough block for the next comparison
                        let next_block = builder.create_block();
                        builder.ins().brif(cmp, *target_cl, &[], next_block, &[]);
                        builder.switch_to_block(next_block);
                        builder.seal_block(next_block);
                    }
                }
            }
        }

        TerminatorKind::Panic(_msg) => {
            // TODO: Call panic handler
            // User trap code 1 = panic
            builder
                .ins()
                .trap(cranelift_codegen::ir::TrapCode::unwrap_user(1));
        }

        TerminatorKind::Unreachable => {
            // User trap code 2 = unreachable
            builder
                .ins()
                .trap(cranelift_codegen::ir::TrapCode::unwrap_user(2));
        }
    }

    Ok(())
}

/// Get the enum ID from a place expression.
fn get_enum_id_from_place(
    ctx: &CodegenContext<'_>,
    place: &kestrel_execution_graph::Place,
) -> Result<kestrel_execution_graph::Id<kestrel_execution_graph::Enum>, CodegenError> {
    // Get the type of the place
    let ty = get_place_type(ctx, place)?;
    let mir_ty = ctx.mir.ty(ty);

    match mir_ty {
        MirTy::Named { name, .. } => {
            let name_data = ctx.mir.name(*name);
            for (id, def) in ctx.mir.enums.iter() {
                let def_name = ctx.mir.name(def.name);
                if def_name == name_data {
                    return Ok(id);
                }
            }
            Err(CodegenError::Unsupported(format!(
                "enum not found for type: {}",
                name_data
            )))
        }
        _ => Err(CodegenError::Unsupported(format!(
            "switch on non-enum type: {:?}",
            mir_ty
        ))),
    }
}

/// Get the type of a place expression.
fn get_place_type(
    ctx: &CodegenContext<'_>,
    place: &kestrel_execution_graph::Place,
) -> Result<kestrel_execution_graph::Id<kestrel_execution_graph::Ty>, CodegenError> {
    match &place.kind {
        PlaceKind::Local(local_id) => {
            let local_def = ctx.mir.local(*local_id);
            Ok(local_def.ty)
        }
        PlaceKind::Field { parent, name } => {
            // Get the parent's type, then look up the field type
            let parent_ty_id = get_place_type(ctx, parent)?;
            let parent_ty = ctx.mir.ty(parent_ty_id);

            // Find the struct and get the field type
            if let MirTy::Named {
                name: type_name, ..
            } = parent_ty
            {
                let type_name_data = ctx.mir.name(*type_name);
                for (_, struct_def) in ctx.mir.structs.iter() {
                    if ctx.mir.name(struct_def.name) == type_name_data {
                        // Found the struct, now find the field
                        for field_id in &struct_def.fields {
                            let field_def = &ctx.mir.fields[*field_id];
                            if field_def.name == *name {
                                return Ok(field_def.ty);
                            }
                        }
                        return Err(CodegenError::Unsupported(format!(
                            "field '{}' not found in struct '{}'",
                            name, type_name_data
                        )));
                    }
                }
                Err(CodegenError::Unsupported(format!(
                    "struct not found for type: {}",
                    type_name_data
                )))
            } else {
                Err(CodegenError::Unsupported(format!(
                    "field access on non-struct type: {:?}",
                    parent_ty
                )))
            }
        }
        PlaceKind::Downcast { parent, .. } => {
            // Downcast preserves the enum type
            get_place_type(ctx, parent)
        }
        PlaceKind::Deref(parent) => {
            // Get the pointer/ref type and extract the pointee type
            let parent_ty_id = get_place_type(ctx, parent)?;
            let parent_ty = ctx.mir.ty(parent_ty_id);
            match parent_ty {
                MirTy::Ref(inner) | MirTy::RefMut(inner) | MirTy::Pointer(inner) => Ok(*inner),
                _ => Err(CodegenError::Unsupported(
                    "deref of non-pointer type".to_string(),
                )),
            }
        }
        PlaceKind::Index { parent, index } => {
            // Get the parent's type, then look up the field type by index
            let parent_ty_id = get_place_type(ctx, parent)?;
            let parent_ty = ctx.mir.ty(parent_ty_id);

            // Check if the parent is a downcast - in that case, find the variant struct
            if let PlaceKind::Downcast {
                parent: grandparent,
                variant,
            } = &parent.kind
            {
                let enum_ty_id = get_place_type(ctx, grandparent)?;
                let enum_ty = ctx.mir.ty(enum_ty_id);

                if let MirTy::Named { name, .. } = enum_ty {
                    let name_data = ctx.mir.name(*name);
                    for (_, enum_def) in ctx.mir.enums.iter() {
                        if ctx.mir.name(enum_def.name) == name_data {
                            let case_id = enum_def.case_by_name(variant).ok_or_else(|| {
                                CodegenError::Unsupported(format!("enum case not found: {}", variant))
                            })?;
                            let case_def = &ctx.mir.enum_cases[case_id];
                            let struct_id = case_def.struct_def.ok_or_else(|| {
                                CodegenError::Unsupported(format!(
                                    "enum case {} has no struct_def",
                                    variant
                                ))
                            })?;
                            let struct_def = ctx.mir.struct_def(struct_id);
                            let fields: Vec<_> = struct_def.fields.clone();
                            if *index >= fields.len() {
                                return Err(CodegenError::Unsupported(format!(
                                    "field index {} out of bounds",
                                    index
                                )));
                            }
                            let field_id = fields[*index];
                            let field_def = &ctx.mir.fields[field_id];
                            return Ok(field_def.ty);
                        }
                    }
                }
            }

            // Regular struct or tuple
            match parent_ty {
                MirTy::Named {
                    name: type_name, ..
                } => {
                    let type_name_data = ctx.mir.name(*type_name);
                    for (_, struct_def) in ctx.mir.structs.iter() {
                        if ctx.mir.name(struct_def.name) == type_name_data {
                            let fields: Vec<_> = struct_def.fields.clone();
                            if *index >= fields.len() {
                                return Err(CodegenError::Unsupported(format!(
                                    "field index {} out of bounds (struct has {} fields)",
                                    index,
                                    fields.len()
                                )));
                            }
                            let field_id = fields[*index];
                            let field_def = &ctx.mir.fields[field_id];
                            return Ok(field_def.ty);
                        }
                    }
                    Err(CodegenError::Unsupported(format!(
                        "struct not found for index access: {}",
                        type_name_data
                    )))
                }
                MirTy::Tuple(elements) => {
                    if *index >= elements.len() {
                        return Err(CodegenError::Unsupported(format!(
                            "tuple index {} out of bounds (len {})",
                            index,
                            elements.len()
                        )));
                    }
                    Ok(elements[*index])
                }
                _ => Err(CodegenError::Unsupported(format!(
                    "index access on unsupported type: {:?}",
                    parent_ty
                ))),
            }
        }
    }
}
