//! Place (memory location) compilation.

use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::monomorphize::Substitution;
use crate::types::{translate_type, translate_type_with_subst};

use kestrel_codegen::Layout;

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
    subst: &Substitution,
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
            let struct_ptr = compile_place_read(ctx, parent, builder, local_map, subst)?;

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
            let field_cl_ty = translate_type_with_subst(ctx.mir, field_ty, ctx.target, subst);

            // Apply substitution to get the concrete field type
            // This is important for generic structs where field types might be type parameters
            let concrete_field_ty = subst
                .apply_ty_readonly(ctx.mir, field_ty)
                .unwrap_or(field_ty);

            // Check if the field is an aggregate type - keep it as a pointer
            if is_aggregate_type(ctx, concrete_field_ty) {
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
            // Index access is used for:
            // 1. Tuple field access (tuple.0, tuple.1, etc.)
            // 2. Enum payload field access after downcast (enum.SomeCase.0, etc.)
            //
            // The parent could be a struct/tuple or a downcast result.
            // We need to get the pointer and add the offset for the indexed field.
            let parent_ptr = compile_place_read(ctx, parent, builder, local_map, subst)?;

            // Get the parent type to find the field at this index
            let parent_ty = get_place_type(ctx, parent, local_map)?;

            // Find the field offset for this index
            let (field_offset, field_ty) = get_field_by_index(ctx, parent, parent_ty, *index)?;

            // Compute pointer type
            let ptr_type = if ctx.target.is_64bit() {
                cl_types::I64
            } else {
                cl_types::I32
            };

            // Load the field value from parent_ptr + offset
            let field_cl_ty = translate_type_with_subst(ctx.mir, field_ty, ctx.target, subst);

            // Apply substitution to get the concrete field type
            // This is important for generic tuples/structs where field types might be type parameters
            let concrete_field_ty = subst
                .apply_ty_readonly(ctx.mir, field_ty)
                .unwrap_or(field_ty);

            // Check if the field is an aggregate type - keep it as a pointer
            if is_aggregate_type(ctx, concrete_field_ty) {
                // For compound fields, return a pointer to the nested struct/tuple
                if field_offset == 0 {
                    Ok(parent_ptr)
                } else {
                    Ok(builder.ins().iadd_imm(parent_ptr, field_offset as i64))
                }
            } else {
                // For primitive fields, load the value
                Ok(builder.ins().load(
                    field_cl_ty,
                    MemFlags::new(),
                    parent_ptr,
                    field_offset as i32,
                ))
            }
        }

        PlaceKind::Downcast { parent, variant } => {
            // Downcast is used after a switch to access the variant's payload.
            // The enum layout is: [discriminant: i32][padding][payload...]
            // After downcast, we return a pointer to the payload area.
            let enum_ptr = compile_place_read(ctx, parent, builder, local_map, subst)?;
            let enum_ty = get_place_type(ctx, parent, local_map)?;
            let payload_offset = get_enum_payload_offset(ctx, enum_ty, variant)?;

            // Compute pointer type
            let ptr_type = if ctx.target.is_64bit() {
                cl_types::I64
            } else {
                cl_types::I32
            };

            // Return a pointer to the payload area
            Ok(builder.ins().iadd_imm(enum_ptr, payload_offset as i64))
        }

        PlaceKind::Deref(inner) => {
            // Get the pointer value from the inner place
            let ptr = compile_place_read(ctx, inner, builder, local_map, subst)?;

            // Get the type of the inner (which should be a pointer/ref type)
            let inner_ty = get_place_type(ctx, inner, local_map)?;
            let pointee_ty = get_pointee_type(ctx, inner_ty)?;

            // Apply substitution to get the concrete pointee type
            // This is critical for generic functions where the pointee might be a type parameter
            let concrete_pointee_ty = subst
                .apply_ty_readonly(ctx.mir, pointee_ty)
                .unwrap_or(pointee_ty);

            // Check if the pointee is an aggregate type - keep it as a pointer
            if is_aggregate_type(ctx, concrete_pointee_ty) {
                // For struct types, the "value" is the pointer itself
                // (structs are always passed by pointer)
                Ok(ptr)
            } else {
                // For primitive types, load the value from memory
                let cl_type = translate_type_with_subst(ctx.mir, pointee_ty, ctx.target, subst);
                Ok(builder.ins().load(cl_type, MemFlags::new(), ptr, 0))
            }
        }
    }
}

/// Get the pointee type of a pointer/reference type.
fn get_pointee_type(ctx: &CodegenContext<'_>, ptr_ty: Id<Ty>) -> Result<Id<Ty>, CodegenError> {
    match ctx.mir.ty(ptr_ty) {
        MirTy::Pointer(inner) | MirTy::Ref(inner) | MirTy::RefMut(inner) => Ok(*inner),
        _ => Err(CodegenError::Unsupported(format!(
            "not a pointer/reference type: {:?}",
            ctx.mir.ty(ptr_ty)
        ))),
    }
}

fn get_enum_payload_offset(
    ctx: &mut CodegenContext<'_>,
    enum_ty: Id<Ty>,
    variant: &str,
) -> Result<usize, CodegenError> {
    let mir_ty = ctx.mir.ty(enum_ty);
    let enum_id = match mir_ty {
        MirTy::Named { name, .. } => {
            let name_data = ctx.mir.name(*name);
            let mut found = None;
            for (id, def) in ctx.mir.enums.iter() {
                if ctx.mir.name(def.name) == name_data {
                    found = Some(id);
                    break;
                }
            }
            found.ok_or_else(|| {
                CodegenError::Unsupported(format!("enum not found for type: {}", name_data))
            })?
        }
        _ => {
            return Err(CodegenError::Unsupported(format!(
                "downcast on non-enum type: {:?}",
                mir_ty
            )));
        }
    };

    let enum_def = ctx.mir.enum_def(enum_id);
    let case_id = enum_def
        .case_by_name(variant)
        .ok_or_else(|| CodegenError::Unsupported(format!("enum case not found: {}", variant)))?;
    let case_def = &ctx.mir.enum_cases[case_id];
    let payload_struct_id = case_def.struct_def.ok_or_else(|| {
        CodegenError::Unsupported(format!("enum case {} has no payload", variant))
    })?;
    let payload_layout = ctx.layouts.struct_layout(payload_struct_id);
    let discriminant_layout = Layout::new(4, 4);
    let (payload_offset, _) = discriminant_layout.append(payload_layout.layout);
    Ok(payload_offset)
}

/// Check if a type is a struct type.
pub fn is_struct_type_public(ctx: &CodegenContext<'_>, ty: Id<Ty>) -> bool {
    is_struct_type(ctx, ty)
}

/// Check if a type is represented as an aggregate behind a pointer.
fn is_aggregate_type(ctx: &CodegenContext<'_>, ty: Id<Ty>) -> bool {
    matches!(
        ctx.mir.ty(ty),
        MirTy::Tuple(_) | MirTy::Named { .. } | MirTy::Str | MirTy::FuncThick { .. }
    )
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

        PlaceKind::Index { parent, index } => {
            // Get the type of the indexed field
            let parent_ty = get_place_type(ctx, parent, local_map)?;
            let (_, field_ty) = get_field_by_index(ctx, parent, parent_ty, *index)?;
            Ok(field_ty)
        }

        PlaceKind::Downcast { parent, .. } => {
            // Downcast doesn't change the type for our purposes
            get_place_type(ctx, parent, local_map)
        }

        PlaceKind::Deref(inner) => {
            // Get the type of the inner (which should be a pointer/ref type)
            let inner_ty = get_place_type(ctx, inner, local_map)?;
            // Return the pointee type
            get_pointee_type(ctx, inner_ty)
        }
    }
}

fn copy_aggregate_value(
    ctx: &mut CodegenContext<'_>,
    ty: Id<Ty>,
    dest_ptr: CraneliftValue,
    src_ptr: CraneliftValue,
    builder: &mut FunctionBuilder<'_>,
) {
    let layout = ctx.layouts.layout_of(ty);
    if layout.size == 0 {
        return;
    }

    // Byte-wise copy keeps correctness for small aggregates (e.g. UInt8 structs).
    for offset in 0..layout.size {
        let byte = builder
            .ins()
            .load(cl_types::I8, MemFlags::new(), src_ptr, offset as i32);
        builder
            .ins()
            .store(MemFlags::new(), byte, dest_ptr, offset as i32);
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

/// Get field offset and type by index for a struct or enum payload.
///
/// This handles the case where we have a downcast to an enum variant and need
/// to access the payload fields by index.
fn get_field_by_index(
    ctx: &mut CodegenContext<'_>,
    parent_place: &Place,
    parent_ty: Id<Ty>,
    index: usize,
) -> Result<(usize, Id<Ty>), CodegenError> {
    // Check if the parent is a downcast - in that case, we need to find the variant struct
    if let PlaceKind::Downcast {
        parent: grandparent,
        variant,
    } = &parent_place.kind
    {
        // Get the enum type from the grandparent
        let enum_ty = get_place_type(ctx, grandparent, &HashMap::new())?;
        let mir_ty = ctx.mir.ty(enum_ty);

        if let MirTy::Named { name, .. } = mir_ty {
            let name_data = ctx.mir.name(*name);

            // Find the enum
            for (enum_id, enum_def) in ctx.mir.enums.iter() {
                let def_name = ctx.mir.name(enum_def.name);
                if def_name == name_data {
                    // Find the case
                    let case_id = enum_def.case_by_name(variant).ok_or_else(|| {
                        CodegenError::Unsupported(format!("enum case not found: {}", variant))
                    })?;
                    let case_def = &ctx.mir.enum_cases[case_id];

                    // Get the payload struct
                    let struct_id = case_def.struct_def.ok_or_else(|| {
                        CodegenError::Unsupported(format!(
                            "enum case {} has no struct_def",
                            variant
                        ))
                    })?;

                    return get_struct_field_by_index(ctx, struct_id, index);
                }
            }

            return Err(CodegenError::Unsupported(format!(
                "enum not found for downcast: {}",
                name_data
            )));
        }
    }

    // Otherwise, it's a regular struct or tuple - look up by index
    let mir_ty = ctx.mir.ty(parent_ty);

    match mir_ty {
        MirTy::Named { name, .. } => {
            let name_data = ctx.mir.name(*name);

            // Try to find as struct
            for (struct_id, def) in ctx.mir.structs.iter() {
                if ctx.mir.name(def.name) == name_data {
                    return get_struct_field_by_index(ctx, struct_id, index);
                }
            }

            Err(CodegenError::Unsupported(format!(
                "struct not found for index access: {}",
                name_data
            )))
        }
        MirTy::Tuple(elements) => {
            // For tuples, calculate offset sequentially
            let elements = elements.clone();
            if index >= elements.len() {
                return Err(CodegenError::Unsupported(format!(
                    "tuple index {} out of bounds (len {})",
                    index,
                    elements.len()
                )));
            }

            // Calculate offset by summing sizes of previous elements
            let mut offset = 0usize;
            for (i, elem_ty) in elements.iter().enumerate() {
                let elem_layout = ctx.layouts.layout_of(*elem_ty);
                // Align to this element's alignment
                offset = (offset + elem_layout.align - 1) & !(elem_layout.align - 1);
                if i == index {
                    return Ok((offset, *elem_ty));
                }
                offset += elem_layout.size;
            }

            unreachable!()
        }
        _ => Err(CodegenError::Unsupported(format!(
            "index access on unsupported type: {:?}",
            mir_ty
        ))),
    }
}

/// Get a struct field by index.
fn get_struct_field_by_index(
    ctx: &mut CodegenContext<'_>,
    struct_id: kestrel_execution_graph::Id<kestrel_execution_graph::Struct>,
    index: usize,
) -> Result<(usize, Id<Ty>), CodegenError> {
    let struct_def = ctx.mir.struct_def(struct_id);
    let fields: Vec<_> = struct_def.fields.clone();

    if index >= fields.len() {
        return Err(CodegenError::Unsupported(format!(
            "field index {} out of bounds (struct has {} fields)",
            index,
            fields.len()
        )));
    }

    let field_id = fields[index];
    let field_def = &ctx.mir.fields[field_id];
    let field_name = &field_def.name;
    let field_ty = field_def.ty;

    // Get field offset from layout
    let struct_layout = ctx.layouts.struct_layout(struct_id);
    let offset = *struct_layout.field_offsets.get(field_name).ok_or_else(|| {
        CodegenError::Unsupported(format!("field offset not found: {}", field_name))
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
    subst: &Substitution,
) -> Result<(), CodegenError> {
    match &place.kind {
        PlaceKind::Local(local_id) => {
            let var = local_map
                .get(local_id)
                .ok_or_else(|| CodegenError::Unsupported("unknown local".to_string()))?;

            let local_def = ctx.mir.local(*local_id);
            let concrete_ty = subst
                .apply_ty_readonly(ctx.mir, local_def.ty)
                .unwrap_or(local_def.ty);
            if is_aggregate_type(ctx, concrete_ty) {
                let dest_ptr = builder.use_var(*var);
                copy_aggregate_value(ctx, concrete_ty, dest_ptr, value, builder);
                return Ok(());
            }

            builder.def_var(*var, value);
            Ok(())
        }

        PlaceKind::Field { parent, name } => {
            // Get the struct pointer from the parent place
            let struct_ptr = compile_place_read(ctx, parent, builder, local_map, subst)?;

            // Get the type of the parent to find field offset
            let parent_ty = get_place_type(ctx, parent, local_map)?;
            let (field_offset, field_ty) = get_field_info(ctx, parent_ty, name)?;

            // Store the value at struct_ptr + offset
            let concrete_ty = subst
                .apply_ty_readonly(ctx.mir, field_ty)
                .unwrap_or(field_ty);
            if is_aggregate_type(ctx, concrete_ty) {
                let dest_ptr = if field_offset == 0 {
                    struct_ptr
                } else {
                    builder.ins().iadd_imm(struct_ptr, field_offset as i64)
                };
                copy_aggregate_value(ctx, concrete_ty, dest_ptr, value, builder);
                return Ok(());
            }

            builder
                .ins()
                .store(MemFlags::new(), value, struct_ptr, field_offset as i32);
            Ok(())
        }

        PlaceKind::Index { parent, index } => {
            // Index write is used for:
            // 1. Tuple field assignment (tuple.0 = value)
            // 2. Enum payload field assignment after downcast (enum.SomeCase.0 = value)
            let parent_ptr = compile_place_read(ctx, parent, builder, local_map, subst)?;

            // Get the parent type to find the field at this index
            let parent_ty = get_place_type(ctx, parent, local_map)?;

            // Find the field offset for this index
            let (field_offset, field_ty) = get_field_by_index(ctx, parent, parent_ty, *index)?;

            // Store the value at parent_ptr + offset
            let concrete_ty = subst
                .apply_ty_readonly(ctx.mir, field_ty)
                .unwrap_or(field_ty);
            if is_aggregate_type(ctx, concrete_ty) {
                let dest_ptr = if field_offset == 0 {
                    parent_ptr
                } else {
                    builder.ins().iadd_imm(parent_ptr, field_offset as i64)
                };
                copy_aggregate_value(ctx, concrete_ty, dest_ptr, value, builder);
                return Ok(());
            }

            builder
                .ins()
                .store(MemFlags::new(), value, parent_ptr, field_offset as i32);
            Ok(())
        }

        PlaceKind::Downcast { parent, variant } => {
            // Downcast write is used when assigning to an enum variant's payload area.
            // The enum layout is: [discriminant: i32][padding][payload...]
            // We need to get the pointer to the payload area and store there.
            let enum_ptr = compile_place_read(ctx, parent, builder, local_map, subst)?;
            let enum_ty = get_place_type(ctx, parent, local_map)?;
            let payload_offset = get_enum_payload_offset(ctx, enum_ty, variant)? as i32;

            // Store the value at the payload area
            builder
                .ins()
                .store(MemFlags::new(), value, enum_ptr, payload_offset);
            Ok(())
        }

        PlaceKind::Deref(inner) => {
            // Get the pointer value from the inner place
            let ptr = compile_place_read(ctx, inner, builder, local_map, subst)?;

            // Store the value at the pointer address
            let dest_ty = get_place_type(ctx, place, local_map)?;
            let concrete_ty = subst.apply_ty_readonly(ctx.mir, dest_ty).unwrap_or(dest_ty);
            if is_aggregate_type(ctx, concrete_ty) {
                copy_aggregate_value(ctx, concrete_ty, ptr, value, builder);
                return Ok(());
            }

            builder.ins().store(MemFlags::new(), value, ptr, 0);
            Ok(())
        }
    }
}

/// Get the address of a place (for taking references).
///
/// This is used by Rvalue::Ref and Rvalue::RefMut to get a pointer to a place.
pub fn compile_place_addr(
    ctx: &mut CodegenContext<'_>,
    place: &Place,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    local_slots: &HashMap<Id<Local>, cranelift_codegen::ir::StackSlot>,
    subst: &Substitution,
) -> Result<CraneliftValue, CodegenError> {
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    match &place.kind {
        PlaceKind::Local(local_id) => {
            // Get the stack slot for this local
            let slot = local_slots.get(local_id).ok_or_else(|| {
                CodegenError::Unsupported(format!(
                    "no stack slot for local (cannot take reference of register-only local)"
                ))
            })?;
            Ok(builder.ins().stack_addr(ptr_type, *slot, 0))
        }

        PlaceKind::Field { parent, name } => {
            // Get the parent's address (which is a struct pointer)
            // For a field, the parent is already a pointer to the struct
            let struct_ptr = compile_place_read(ctx, parent, builder, local_map, subst)?;

            // Get the field offset
            let parent_ty = get_place_type(ctx, parent, local_map)?;
            let (field_offset, _field_ty) = get_field_info(ctx, parent_ty, name)?;

            // Compute field address
            if field_offset == 0 {
                Ok(struct_ptr)
            } else {
                Ok(builder.ins().iadd_imm(struct_ptr, field_offset as i64))
            }
        }

        PlaceKind::Index { parent, index } => {
            // Get the parent pointer
            let parent_ptr = compile_place_read(ctx, parent, builder, local_map, subst)?;

            // Get the parent type and field info
            let parent_ty = get_place_type(ctx, parent, local_map)?;
            let (field_offset, _field_ty) = get_field_by_index(ctx, parent, parent_ty, *index)?;

            // Compute field address
            if field_offset == 0 {
                Ok(parent_ptr)
            } else {
                Ok(builder.ins().iadd_imm(parent_ptr, field_offset as i64))
            }
        }

        PlaceKind::Downcast { parent, variant: _ } => {
            // Downcast doesn't change the address - it just changes how we interpret it
            // The payload is at offset 4 (after the discriminant)
            let enum_ptr = compile_place_read(ctx, parent, builder, local_map, subst)?;
            Ok(builder.ins().iadd_imm(enum_ptr, 4))
        }

        PlaceKind::Deref(inner) => {
            // The address of *ptr is just ptr itself
            compile_place_read(ctx, inner, builder, local_map, subst)
        }
    }
}
