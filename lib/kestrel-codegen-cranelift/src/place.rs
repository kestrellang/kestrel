//! Place (memory location) compilation.

use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::monomorphize::{Substitution, build_substitution};
use crate::types::translate_type_with_subst;

use kestrel_codegen::Layout;

use kestrel_execution_graph::{Id, Local, MirTy, Place, PlaceKind, Ty};

use cranelift_codegen::ir::types as cl_types;
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value as CraneliftValue};
use cranelift_frontend::{FunctionBuilder, Variable};
use cranelift_module::Module;

use std::collections::{HashMap, HashSet};

/// Read a value from a place.
pub fn compile_place_read(
    ctx: &mut CodegenContext<'_>,
    place: &Place,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    subst: &Substitution,
    stack_locals: &HashSet<Id<Local>>,
) -> Result<CraneliftValue, CodegenError> {
    match &place.kind {
        PlaceKind::Local(local_id) => {
            let var = local_map
                .get(local_id)
                .ok_or_else(|| CodegenError::Unsupported("unknown local".to_string()))?;
            let local_def = ctx.mir.local(*local_id);
            if stack_locals.contains(local_id) {
                let addr = builder.use_var(*var);
                let cl_type = translate_type_with_subst(ctx.mir, local_def.ty, ctx.target, subst);
                Ok(builder.ins().load(cl_type, MemFlags::new(), addr, 0))
            } else {
                // Both aggregate and non-aggregate types use the variable directly
                Ok(builder.use_var(*var))
            }
        },

        PlaceKind::Global(name_id) => {
            // Global/static variable access
            let global_name = ctx.mir.name(*name_id);
            let mangled_name = format!("{}", global_name);

            // Look up the global symbol
            let global_ref = ctx
                .module
                .declare_data(&mangled_name, cranelift_module::Linkage::Import, false, false)
                .map_err(|e| CodegenError::Unsupported(format!("failed to declare global: {}", e)))?;

            // Get the global address
            let global_addr = ctx
                .module
                .declare_data_in_func(global_ref, builder.func);

            // Find the static definition to get its type
            let static_def = ctx
                .mir
                .statics
                .iter()
                .find(|(_, def)| def.name == *name_id)
                .map(|(_, def)| def)
                .ok_or_else(|| {
                    CodegenError::Unsupported(format!("static variable not found: {}", mangled_name))
                })?;

            let static_ty = static_def.ty;
            let cl_type = translate_type_with_subst(ctx.mir, static_ty, ctx.target, subst);

            // Compute pointer type
            let ptr_type = if ctx.target.is_64bit() {
                cl_types::I64
            } else {
                cl_types::I32
            };

            // Load the value from the global
            let ptr = builder.ins().global_value(ptr_type, global_addr);
            Ok(builder.ins().load(cl_type, MemFlags::new(), ptr, 0))
        },

        PlaceKind::Field { parent, name } => {
            // Get the struct pointer from the parent place
            let struct_ptr =
                compile_place_read(ctx, parent, builder, local_map, subst, stack_locals)?;

            // Get the type of the parent to find field offset
            let parent_ty = get_place_type(ctx, parent, local_map, subst)?;
            let (field_offset, field_ty) = get_field_info(ctx, parent_ty, name, subst)?;

            // Compute pointer type
            let _ptr_type = if ctx.target.is_64bit() {
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
        },

        PlaceKind::Index { parent, index } => {
            // Index access is used for:
            // 1. Tuple field access (tuple.0, tuple.1, etc.)
            // 2. Enum payload field access after downcast (enum.SomeCase.0, etc.)
            //
            // The parent could be a struct/tuple or a downcast result.
            // We need to get the pointer and add the offset for the indexed field.
            let parent_ptr =
                compile_place_read(ctx, parent, builder, local_map, subst, stack_locals)?;

            // Get the parent type to find the field at this index
            let parent_ty = get_place_type(ctx, parent, local_map, subst)?;

            // Find the field offset for this index
            let (field_offset, field_ty) =
                get_field_by_index(ctx, parent, parent_ty, *index, subst)?;

            // Compute pointer type
            let _ptr_type = if ctx.target.is_64bit() {
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
        },

        PlaceKind::Downcast { parent, variant } => {
            // Downcast is used after a switch to access the variant's payload.
            // The enum layout is: [discriminant: i32][padding][payload...]
            // After downcast, we return a pointer to the payload area.
            let enum_ptr =
                compile_place_read(ctx, parent, builder, local_map, subst, stack_locals)?;
            let enum_ty = get_place_type(ctx, parent, local_map, subst)?;
            let payload_offset = get_enum_payload_offset(ctx, enum_ty, variant, subst)?;

            // Compute pointer type
            let _ptr_type = if ctx.target.is_64bit() {
                cl_types::I64
            } else {
                cl_types::I32
            };

            // Return a pointer to the payload area
            Ok(builder.ins().iadd_imm(enum_ptr, payload_offset as i64))
        },

        PlaceKind::Deref(inner) => {
            // Get the pointer value from the inner place
            let ptr = compile_place_read(ctx, inner, builder, local_map, subst, stack_locals)?;

            // Get the type of the inner (which should be a pointer/ref type)
            let inner_ty = get_place_type(ctx, inner, local_map, subst)?;
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
        },
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

pub(crate) fn get_enum_payload_offset(
    ctx: &mut CodegenContext<'_>,
    enum_ty: Id<Ty>,
    _variant: &str,
    subst: &Substitution,
) -> Result<usize, CodegenError> {
    let mir_ty = ctx.mir.ty(enum_ty);
    let (enum_id, type_args) = match mir_ty {
        MirTy::Named { name, type_args } => {
            let name_data = ctx.mir.name(*name);
            // Apply substitution to type_args to replace any type parameters with concrete types
            let type_args: Vec<_> = type_args
                .iter()
                .map(|&ty| subst.apply_ty_readonly(ctx.mir, ty).unwrap_or(ty))
                .collect();
            let mut found = None;
            for (id, def) in ctx.mir.enums.iter() {
                if ctx.mir.name(def.name) == name_data {
                    found = Some(id);
                    break;
                }
            }
            let enum_id = found.ok_or_else(|| {
                CodegenError::Unsupported(format!("enum not found for type: {}", name_data))
            })?;
            (enum_id, type_args)
        },
        _ => {
            return Err(CodegenError::Unsupported(format!(
                "downcast on non-enum type: {:?}",
                mir_ty
            )));
        },
    };

    let enum_def = ctx.mir.enum_def(enum_id);

    // We need to compute the payload offset based on the maximum payload alignment
    // across ALL cases, not just the specific case we're accessing. This ensures
    // all cases have a consistent payload offset.
    let case_ids: Vec<_> = enum_def.cases.clone();
    let mut max_payload_layout = Layout::zero(1);

    for case_id in &case_ids {
        let case_def = &ctx.mir.enum_cases[*case_id];
        if let Some(struct_id) = case_def.struct_def {
            let payload_layout = ctx.layouts.struct_layout(struct_id, &type_args);
            // Track the maximum alignment and size
            if payload_layout.layout.align > max_payload_layout.align
                || (payload_layout.layout.align == max_payload_layout.align
                    && payload_layout.layout.size > max_payload_layout.size)
            {
                max_payload_layout = payload_layout.layout;
            }
        }
    }

    let discriminant_layout = Layout::new(4, 4);
    let (payload_offset, _) = discriminant_layout.append(max_payload_layout);
    Ok(payload_offset)
}

/// Check if a type is a struct type.
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(clippy::only_used_in_recursion)]
fn get_place_type(
    ctx: &mut CodegenContext<'_>,
    place: &Place,
    local_map: &HashMap<Id<Local>, Variable>,
    subst: &Substitution,
) -> Result<Id<Ty>, CodegenError> {
    match &place.kind {
        PlaceKind::Local(local_id) => {
            let local_def = ctx.mir.local(*local_id);
            Ok(local_def.ty)
        },

        PlaceKind::Global(name_id) => {
            // Find the static definition to get its type
            let static_def = ctx
                .mir
                .statics
                .iter()
                .find(|(_, def)| def.name == *name_id)
                .map(|(_, def)| def)
                .ok_or_else(|| {
                    let global_name = ctx.mir.name(*name_id);
                    CodegenError::Unsupported(format!("static variable not found: {}", global_name))
                })?;
            Ok(static_def.ty)
        },

        PlaceKind::Field { parent, name } => {
            let parent_ty = get_place_type(ctx, parent, local_map, subst)?;
            let (_, field_ty) = get_field_info(ctx, parent_ty, name, subst)?;
            Ok(field_ty)
        },

        PlaceKind::Index { parent, index } => {
            // Get the type of the indexed field
            let parent_ty = get_place_type(ctx, parent, local_map, subst)?;
            let (_, field_ty) = get_field_by_index(ctx, parent, parent_ty, *index, subst)?;
            Ok(field_ty)
        },

        PlaceKind::Downcast { parent, .. } => {
            // Downcast doesn't change the type for our purposes
            get_place_type(ctx, parent, local_map, subst)
        },

        PlaceKind::Deref(inner) => {
            // Get the type of the inner (which should be a pointer/ref type)
            let inner_ty = get_place_type(ctx, inner, local_map, subst)?;
            // Return the pointee type
            get_pointee_type(ctx, inner_ty)
        },
    }
}

fn copy_aggregate_value(
    ctx: &mut CodegenContext<'_>,
    ty: Id<Ty>,
    dest_ptr: CraneliftValue,
    src_ptr: CraneliftValue,
    builder: &mut FunctionBuilder<'_>,
) {
    // Unit types have zero size - nothing to copy
    if matches!(ctx.mir.ty(ty), kestrel_execution_graph::MirTy::Unit) {
        return;
    }

    let layout = ctx.layouts.layout_of(ty);
    if layout.size == 0 {
        return;
    }

    // Skip copy if src_ptr is a constant 0 (null pointer from Unit value).
    // This can happen when if-else expressions have aggregate types but
    // the branch values are from discarded statement results.
    if let cranelift_codegen::ir::ValueDef::Result(inst, _) = builder.func.dfg.value_def(src_ptr)
        && let cranelift_codegen::ir::InstructionData::UnaryImm { imm, .. } =
            builder.func.dfg.insts[inst]
        && imm.bits() == 0
    {
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
    subst: &Substitution,
) -> Result<(usize, Id<Ty>), CodegenError> {
    let mir_ty = ctx.mir.ty(parent_ty);

    let (struct_id, type_args) = match mir_ty {
        MirTy::Named { name, type_args } => {
            let name_data = ctx.mir.name(*name);
            // Apply substitution to type_args to replace any type parameters with concrete types
            let type_args: Vec<_> = type_args
                .iter()
                .map(|&ty| subst.apply_ty_readonly(ctx.mir, ty).unwrap_or(ty))
                .collect();
            let mut found = None;
            for (id, def) in ctx.mir.structs.iter() {
                if ctx.mir.name(def.name) == name_data {
                    found = Some(id);
                    break;
                }
            }
            let struct_id = found.ok_or_else(|| {
                CodegenError::Unsupported(format!("struct not found: {}", name_data))
            })?;
            (struct_id, type_args)
        },
        _ => {
            return Err(CodegenError::Unsupported(format!(
                "field access on non-struct type: {:?}",
                mir_ty
            )));
        },
    };

    // Get field offset from layout (pass type_args for generic structs)
    let struct_layout = ctx.layouts.struct_layout(struct_id, &type_args);
    let offset = *struct_layout
        .field_offsets
        .get(field_name)
        .ok_or_else(|| CodegenError::Unsupported(format!("unknown field: {}", field_name)))?;

    // Get field type
    let struct_def = ctx.mir.struct_def(struct_id);
    let type_params: Vec<_> = struct_def.type_params.clone();
    let mut field_ty = None;
    for field_id in &struct_def.fields {
        let field_def = &ctx.mir.fields[*field_id];
        if field_def.name == field_name {
            field_ty = Some(field_def.ty);
            break;
        }
    }

    let mut field_ty = field_ty.ok_or_else(|| {
        CodegenError::Unsupported(format!("field type not found: {}", field_name))
    })?;

    // Substitute the field type if the struct is generic
    if !type_params.is_empty() && type_params.len() == type_args.len() {
        let field_subst = build_substitution(ctx.mir, &type_params, &type_args);
        if let Ok(substituted_ty) = field_subst.apply_ty_readonly(ctx.mir, field_ty) {
            field_ty = substituted_ty;
        }
    }

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
    subst: &Substitution,
) -> Result<(usize, Id<Ty>), CodegenError> {
    // Check if the parent is a downcast - in that case, we need to find the variant struct
    if let PlaceKind::Downcast {
        parent: grandparent,
        variant,
    } = &parent_place.kind
    {
        // Get the enum type from the grandparent
        let enum_ty = get_place_type(ctx, grandparent, &HashMap::new(), subst)?;
        let mir_ty = ctx.mir.ty(enum_ty);

        if let MirTy::Named { name, type_args } = mir_ty {
            let name_data = ctx.mir.name(*name);
            let type_args = type_args.clone();

            // Find the enum
            for (_enum_id, enum_def) in ctx.mir.enums.iter() {
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

                    // Pass the enum's type_args since payload struct uses the same type parameters
                    return get_struct_field_by_index(ctx, struct_id, &type_args, index, subst);
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
        MirTy::Named { name, type_args } => {
            let name_data = ctx.mir.name(*name);
            let type_args = type_args.clone();

            // Try to find as struct
            for (struct_id, def) in ctx.mir.structs.iter() {
                if ctx.mir.name(def.name) == name_data {
                    return get_struct_field_by_index(ctx, struct_id, &type_args, index, subst);
                }
            }

            Err(CodegenError::Unsupported(format!(
                "struct not found for index access: {}",
                name_data
            )))
        },
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
            // Apply substitution to element types to get correct layouts
            let mut offset = 0usize;
            for (i, elem_ty) in elements.iter().enumerate() {
                let concrete_ty = subst
                    .apply_ty_readonly(ctx.mir, *elem_ty)
                    .unwrap_or(*elem_ty);
                let elem_layout = ctx.layouts.layout_of(concrete_ty);
                // Align to this element's alignment
                offset = (offset + elem_layout.align - 1) & !(elem_layout.align - 1);
                if i == index {
                    return Ok((offset, *elem_ty));
                }
                offset += elem_layout.size;
            }

            unreachable!()
        },
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
    type_args: &[Id<Ty>],
    index: usize,
    subst: &Substitution,
) -> Result<(usize, Id<Ty>), CodegenError> {
    let struct_def = ctx.mir.struct_def(struct_id);
    let fields: Vec<_> = struct_def.fields.clone();
    let type_params: Vec<_> = struct_def.type_params.clone();

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
    let mut field_ty = field_def.ty;

    // Apply substitution to type_args to get concrete types
    let concrete_type_args: Vec<_> = type_args
        .iter()
        .map(|&ty| subst.apply_ty_readonly(ctx.mir, ty).unwrap_or(ty))
        .collect();

    // Get field offset from layout (pass substituted type_args for generic structs)
    let struct_layout = ctx.layouts.struct_layout(struct_id, &concrete_type_args);
    let offset = *struct_layout.field_offsets.get(field_name).ok_or_else(|| {
        CodegenError::Unsupported(format!("field offset not found: {}", field_name))
    })?;

    // Substitute the field type if the struct is generic
    // The field may have type T (a type parameter) which needs to be substituted
    // with the concrete type argument
    if !type_params.is_empty() && type_params.len() == concrete_type_args.len() {
        let field_subst = build_substitution(ctx.mir, &type_params, &concrete_type_args);
        if let Ok(substituted_ty) = field_subst.apply_ty_readonly(ctx.mir, field_ty) {
            field_ty = substituted_ty;
        }
    }

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
    stack_locals: &HashSet<Id<Local>>,
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

            // Unit types are zero-sized - nothing to write
            if matches!(
                ctx.mir.ty(concrete_ty),
                kestrel_execution_graph::MirTy::Unit
            ) {
                return Ok(());
            }

            if stack_locals.contains(local_id) {
                let addr = builder.use_var(*var);
                builder.ins().store(MemFlags::new(), value, addr, 0);
                return Ok(());
            }
            if is_aggregate_type(ctx, concrete_ty) {
                let dest_ptr = builder.use_var(*var);
                copy_aggregate_value(ctx, concrete_ty, dest_ptr, value, builder);
                return Ok(());
            }

            builder.def_var(*var, value);
            Ok(())
        },

        PlaceKind::Global(name_id) => {
            // Global/static variable write
            let global_name = ctx.mir.name(*name_id);
            let mangled_name = format!("{}", global_name);

            // Look up the global symbol
            let global_ref = ctx
                .module
                .declare_data(&mangled_name, cranelift_module::Linkage::Import, false, false)
                .map_err(|e| CodegenError::Unsupported(format!("failed to declare global: {}", e)))?;

            // Get the global address
            let global_addr = ctx
                .module
                .declare_data_in_func(global_ref, builder.func);

            // Compute pointer type
            let ptr_type = if ctx.target.is_64bit() {
                cl_types::I64
            } else {
                cl_types::I32
            };

            // Get the pointer to the global
            let ptr = builder.ins().global_value(ptr_type, global_addr);

            // Store the value to the global
            builder.ins().store(MemFlags::new(), value, ptr, 0);
            Ok(())
        },

        PlaceKind::Field { parent, name } => {
            // Get the struct pointer from the parent place
            let struct_ptr =
                compile_place_read(ctx, parent, builder, local_map, subst, stack_locals)?;

            // Get the type of the parent to find field offset
            let parent_ty = get_place_type(ctx, parent, local_map, subst)?;
            let (field_offset, field_ty) = get_field_info(ctx, parent_ty, name, subst)?;

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
        },

        PlaceKind::Index { parent, index } => {
            // Index write is used for:
            // 1. Tuple field assignment (tuple.0 = value)
            // 2. Enum payload field assignment after downcast (enum.SomeCase.0 = value)
            let parent_ptr =
                compile_place_read(ctx, parent, builder, local_map, subst, stack_locals)?;

            // Get the parent type to find the field at this index
            let parent_ty = get_place_type(ctx, parent, local_map, subst)?;

            // Find the field offset for this index
            let (field_offset, field_ty) =
                get_field_by_index(ctx, parent, parent_ty, *index, subst)?;

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
        },

        PlaceKind::Downcast { parent, variant } => {
            // Downcast write is used when assigning to an enum variant's payload area.
            // The enum layout is: [discriminant: i32][padding][payload...]
            // We need to get the pointer to the payload area and store there.
            let enum_ptr =
                compile_place_read(ctx, parent, builder, local_map, subst, stack_locals)?;
            let enum_ty = get_place_type(ctx, parent, local_map, subst)?;
            let payload_offset = get_enum_payload_offset(ctx, enum_ty, variant, subst)? as i32;

            // Store the value at the payload area
            builder
                .ins()
                .store(MemFlags::new(), value, enum_ptr, payload_offset);
            Ok(())
        },

        PlaceKind::Deref(inner) => {
            // Get the pointer value from the inner place
            let ptr = compile_place_read(ctx, inner, builder, local_map, subst, stack_locals)?;

            // Store the value at the pointer address
            let dest_ty = get_place_type(ctx, place, local_map, subst)?;
            let concrete_ty = subst.apply_ty_readonly(ctx.mir, dest_ty).unwrap_or(dest_ty);
            if is_aggregate_type(ctx, concrete_ty) {
                copy_aggregate_value(ctx, concrete_ty, ptr, value, builder);
                return Ok(());
            }

            builder.ins().store(MemFlags::new(), value, ptr, 0);
            Ok(())
        },
    }
}

/// Get the address of a place (for taking references).
///
/// This is used by Rvalue::Ref and Rvalue::RefMut to get a pointer to a place.
#[allow(dead_code)]
pub fn compile_place_addr(
    ctx: &mut CodegenContext<'_>,
    place: &Place,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    local_slots: &HashMap<Id<Local>, cranelift_codegen::ir::StackSlot>,
    subst: &Substitution,
    stack_locals: &HashSet<Id<Local>>,
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
                CodegenError::Unsupported(
                    "no stack slot for local (cannot take reference of register-only local)"
                        .to_string(),
                )
            })?;
            Ok(builder.ins().stack_addr(ptr_type, *slot, 0))
        },

        PlaceKind::Global(name_id) => {
            // Global/static variable - return the address of the global
            let global_name = ctx.mir.name(*name_id);
            let mangled_name = format!("{}", global_name);

            // Look up the global symbol
            let global_ref = ctx
                .module
                .declare_data(&mangled_name, cranelift_module::Linkage::Import, false, false)
                .map_err(|e| CodegenError::Unsupported(format!("failed to declare global: {}", e)))?;

            // Get the global address
            let global_addr = ctx
                .module
                .declare_data_in_func(global_ref, builder.func);

            // Return the address of the global
            Ok(builder.ins().global_value(ptr_type, global_addr))
        },

        PlaceKind::Field { parent, name } => {
            // Get the parent's address (which is a struct pointer)
            // For a field, the parent is already a pointer to the struct
            let struct_ptr =
                compile_place_read(ctx, parent, builder, local_map, subst, stack_locals)?;

            // Get the field offset
            let parent_ty = get_place_type(ctx, parent, local_map, subst)?;
            let (field_offset, _field_ty) = get_field_info(ctx, parent_ty, name, subst)?;

            // Compute field address
            if field_offset == 0 {
                Ok(struct_ptr)
            } else {
                Ok(builder.ins().iadd_imm(struct_ptr, field_offset as i64))
            }
        },

        PlaceKind::Index { parent, index } => {
            // Get the parent pointer
            let parent_ptr =
                compile_place_read(ctx, parent, builder, local_map, subst, stack_locals)?;

            // Get the parent type and field info
            let parent_ty = get_place_type(ctx, parent, local_map, subst)?;
            let (field_offset, _field_ty) =
                get_field_by_index(ctx, parent, parent_ty, *index, subst)?;

            // Compute field address
            if field_offset == 0 {
                Ok(parent_ptr)
            } else {
                Ok(builder.ins().iadd_imm(parent_ptr, field_offset as i64))
            }
        },

        PlaceKind::Downcast { parent, variant } => {
            // Downcast doesn't change the address - it just changes how we interpret it.
            let enum_ptr =
                compile_place_read(ctx, parent, builder, local_map, subst, stack_locals)?;
            let enum_ty = get_place_type(ctx, parent, local_map, subst)?;
            let payload_offset = get_enum_payload_offset(ctx, enum_ty, variant, subst)?;
            Ok(builder.ins().iadd_imm(enum_ptr, payload_offset as i64))
        },

        PlaceKind::Deref(inner) => {
            // The address of *ptr is just ptr itself
            compile_place_read(ctx, inner, builder, local_map, subst, stack_locals)
        },
    }
}
