//! Place (memory location) compilation.

use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::types::translate_type;

use kestrel_execution_graph::{Id, Local, MirTy, Place, PlaceKind, Ty};

use cranelift_codegen::ir::types as cl_types;
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value as CraneliftValue};
use cranelift_frontend::{FunctionBuilder, Variable};

use std::collections::HashMap;

/// Read a value from a place.
pub fn compile_place_read(
    ctx: &mut CodegenContext<'_>,
    place: &Place,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<CraneliftValue, CodegenError> {
    match &place.kind {
        PlaceKind::Local(local_id) => {
            let var = local_map
                .get(local_id)
                .ok_or_else(|| CodegenError::Unsupported("unknown local".to_string()))?;
            Ok(builder.use_var(*var))
        }

        PlaceKind::Field { parent, name } => {
            // Get the struct pointer from the parent place
            let struct_ptr = compile_place_read(ctx, parent, builder, local_map)?;

            // Get the type of the parent to find field offset
            let parent_ty = get_place_type(ctx, parent, local_map)?;
            let (field_offset, field_ty) = get_field_info(ctx, parent_ty, name)?;

            // Compute pointer type
            let ptr_type = if ctx.target.is_64bit() {
                cl_types::I64
            } else {
                cl_types::I32
            };

            // Load the field value from struct_ptr + offset
            let field_cl_ty = translate_type(ctx.mir, field_ty, ctx.target);

            // Check if the field is itself a struct (compound type)
            let field_mir_ty = ctx.mir.ty(field_ty);
            let is_struct_field =
                matches!(field_mir_ty, MirTy::Named { .. }) && is_struct_type(ctx, field_ty);

            if is_struct_field {
                // For struct fields, return a pointer to the nested struct
                if field_offset == 0 {
                    Ok(struct_ptr)
                } else {
                    Ok(builder.ins().iadd_imm(struct_ptr, field_offset as i64))
                }
            } else {
                // For primitive fields, load the value
                Ok(builder.ins().load(
                    field_cl_ty,
                    MemFlags::new(),
                    struct_ptr,
                    field_offset as i32,
                ))
            }
        }

        PlaceKind::Index { parent, index } => {
            // TODO: Implement tuple/array indexing
            Err(CodegenError::Unsupported("index access".to_string()))
        }

        PlaceKind::Downcast { parent, variant } => {
            // TODO: Implement enum downcast
            Err(CodegenError::Unsupported("enum downcast".to_string()))
        }

        PlaceKind::Deref(inner) => {
            // TODO: Implement dereference
            Err(CodegenError::Unsupported("dereference".to_string()))
        }
    }
}

/// Check if a type is a struct type.
fn is_struct_type(ctx: &CodegenContext<'_>, ty: Id<Ty>) -> bool {
    let mir_ty = ctx.mir.ty(ty);
    if let MirTy::Named { name, .. } = mir_ty {
        let name_data = ctx.mir.name(*name);
        for (_, def) in ctx.mir.structs.iter() {
            if ctx.mir.name(def.name) == name_data {
                return true;
            }
        }
    }
    false
}

/// Get the type of a place expression.
fn get_place_type(
    ctx: &mut CodegenContext<'_>,
    place: &Place,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<Id<Ty>, CodegenError> {
    match &place.kind {
        PlaceKind::Local(local_id) => {
            let local_def = ctx.mir.local(*local_id);
            Ok(local_def.ty)
        }

        PlaceKind::Field { parent, name } => {
            let parent_ty = get_place_type(ctx, parent, local_map)?;
            let (_, field_ty) = get_field_info(ctx, parent_ty, name)?;
            Ok(field_ty)
        }

        PlaceKind::Index { parent, index: _ } => {
            // TODO: Handle tuple indexing
            Err(CodegenError::Unsupported("index type".to_string()))
        }

        PlaceKind::Downcast { parent, .. } => {
            // Downcast doesn't change the type for our purposes
            get_place_type(ctx, parent, local_map)
        }

        PlaceKind::Deref(_inner) => {
            // TODO: Handle pointer dereference
            Err(CodegenError::Unsupported("deref type".to_string()))
        }
    }
}

/// Get field offset and type for a named field in a struct type.
fn get_field_info(
    ctx: &mut CodegenContext<'_>,
    parent_ty: Id<Ty>,
    field_name: &str,
) -> Result<(usize, Id<Ty>), CodegenError> {
    let mir_ty = ctx.mir.ty(parent_ty);

    let struct_id = match mir_ty {
        MirTy::Named { name, .. } => {
            let name_data = ctx.mir.name(*name);
            let mut found = None;
            for (id, def) in ctx.mir.structs.iter() {
                if ctx.mir.name(def.name) == name_data {
                    found = Some(id);
                    break;
                }
            }
            found.ok_or_else(|| {
                CodegenError::Unsupported(format!("struct not found: {}", name_data))
            })?
        }
        _ => {
            return Err(CodegenError::Unsupported(format!(
                "field access on non-struct type: {:?}",
                mir_ty
            )));
        }
    };

    // Get field offset from layout
    let struct_layout = ctx.layouts.struct_layout(struct_id);
    let offset = *struct_layout
        .field_offsets
        .get(field_name)
        .ok_or_else(|| CodegenError::Unsupported(format!("unknown field: {}", field_name)))?;

    // Get field type
    let struct_def = ctx.mir.struct_def(struct_id);
    let mut field_ty = None;
    for field_id in &struct_def.fields {
        let field_def = &ctx.mir.fields[*field_id];
        if field_def.name == field_name {
            field_ty = Some(field_def.ty);
            break;
        }
    }

    let field_ty = field_ty.ok_or_else(|| {
        CodegenError::Unsupported(format!("field type not found: {}", field_name))
    })?;

    Ok((offset, field_ty))
}

/// Write a value to a place.
pub fn compile_place_write(
    ctx: &mut CodegenContext<'_>,
    place: &Place,
    value: CraneliftValue,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<(), CodegenError> {
    match &place.kind {
        PlaceKind::Local(local_id) => {
            let var = local_map
                .get(local_id)
                .ok_or_else(|| CodegenError::Unsupported("unknown local".to_string()))?;
            builder.def_var(*var, value);
            Ok(())
        }

        PlaceKind::Field { parent, name } => {
            // Get the struct pointer from the parent place
            let struct_ptr = compile_place_read(ctx, parent, builder, local_map)?;

            // Get the type of the parent to find field offset
            let parent_ty = get_place_type(ctx, parent, local_map)?;
            let (field_offset, _field_ty) = get_field_info(ctx, parent_ty, name)?;

            // Store the value at struct_ptr + offset
            builder
                .ins()
                .store(MemFlags::new(), value, struct_ptr, field_offset as i32);
            Ok(())
        }

        PlaceKind::Index { .. } => Err(CodegenError::Unsupported("index write".to_string())),

        PlaceKind::Downcast { .. } => Err(CodegenError::Unsupported("downcast write".to_string())),

        PlaceKind::Deref(_) => Err(CodegenError::Unsupported("deref write".to_string())),
    }
}
