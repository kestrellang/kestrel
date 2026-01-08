//! Rvalue compilation.

use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::monomorphize::{resolve_witness, Substitution};
use crate::place::compile_place_read;
use crate::types::translate_type;

use kestrel_codegen::mangle_name;
use kestrel_execution_graph::{
    BinOp, CallArg, Callee, CastKind, FloatBits, Function, FunctionDef, Id, Immediate,
    ImmediateKind, IntBits, Local, MirTy, Origin, Place, PlaceKind, QualifiedName, Rvalue, Struct,
    Ty, UnOp, Value,
};

use cranelift_codegen::ir::types as cl_types;
use cranelift_codegen::ir::{
    AbiParam, InstBuilder, MemFlags, Signature, StackSlotData, StackSlotKind,
    Value as CraneliftValue,
};
use cranelift_frontend::{FunctionBuilder, Variable};
use cranelift_module::Module;

use std::collections::HashMap;

/// Compile an rvalue to a Cranelift value.
pub fn compile_rvalue(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    rvalue: &Rvalue,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<CraneliftValue, CodegenError> {
    match rvalue {
        Rvalue::Use(imm) => compile_immediate(ctx, subst, imm, builder),

        Rvalue::Copy(place) | Rvalue::Move(place) => {
            compile_place_read(ctx, place, builder, local_map, subst)
        }

        Rvalue::BinaryOp { op, lhs, rhs } => {
            let lhs_val = compile_value(ctx, func_def, subst, lhs, builder, local_map)?;
            let rhs_val = compile_value(ctx, func_def, subst, rhs, builder, local_map)?;
            compile_binop(ctx, *op, lhs_val, rhs_val, builder)
        }

        Rvalue::UnaryOp { op, operand } => {
            let operand_val = compile_value(ctx, func_def, subst, operand, builder, local_map)?;
            compile_unop(ctx, *op, operand_val, builder)
        }

        Rvalue::Call { callee, args } => {
            compile_call(ctx, func_def, subst, callee, args, builder, local_map)
        }

        Rvalue::Construct { ty, fields } => {
            compile_construct(ctx, func_def, subst, *ty, fields, builder, local_map)
        }

        Rvalue::EnumVariant {
            enum_ty,
            variant,
            payload,
        } => compile_enum_variant(
            ctx, func_def, subst, *enum_ty, variant, payload, builder, local_map,
        ),

        Rvalue::Ref(place) | Rvalue::RefMut(place) => {
            compile_ref(ctx, place, builder, local_map, subst)
        }

        // Pointer/reference conversions - these are no-ops at runtime
        Rvalue::PtrToRef(value) | Rvalue::PtrToRefMut(value) | Rvalue::RefToPtr(value) => {
            // All three are semantically different but have the same runtime representation
            compile_value(ctx, func_def, subst, value, builder, local_map)
        }

        Rvalue::PtrOffset { ptr, offset } => {
            compile_ptr_offset(ctx, func_def, subst, ptr, offset, builder, local_map)
        }

        Rvalue::Cast {
            kind,
            operand,
            target,
        } => compile_cast(
            ctx, func_def, subst, *kind, operand, *target, builder, local_map,
        ),

        // String intrinsics
        Rvalue::StrPtr(value) => compile_str_ptr(ctx, func_def, subst, value, builder, local_map),
        Rvalue::StrLen(value) => compile_str_len(ctx, func_def, subst, value, builder, local_map),
        Rvalue::StrFromParts { ptr, len } => {
            compile_str_from_parts(ctx, func_def, subst, ptr, len, builder, local_map)
        }

        Rvalue::Tuple(values) => compile_tuple(ctx, func_def, subst, values, builder, local_map),

        Rvalue::Array { .. } => Err(CodegenError::Unsupported(
            "arrays not yet supported - use std arrays".into(),
        )),

        Rvalue::ApplyPartial { func, captures } => {
            compile_apply_partial(ctx, func_def, subst, *func, captures, builder, local_map)
        }

        // TODO: Implement remaining rvalues
        _ => Err(CodegenError::Unsupported(format!("rvalue: {:?}", rvalue))),
    }
}

/// Compile a struct construction.
///
/// Allocates stack space for the struct, stores each field value at its offset,
/// and returns a pointer to the stack slot.
fn compile_construct(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    ty: Id<Ty>,
    fields: &[(String, Value)],
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<CraneliftValue, CodegenError> {
    // Get the struct layout to determine size and field offsets
    let mir_ty = ctx.mir.ty(ty);

    // Find the struct ID from the type
    let struct_id = match mir_ty {
        MirTy::Named { name, .. } => {
            // Look up struct by name
            let name_data = ctx.mir.name(*name);
            let mut found_struct = None;
            for (id, def) in ctx.mir.structs.iter() {
                let def_name = ctx.mir.name(def.name);
                if def_name == name_data {
                    found_struct = Some(id);
                    break;
                }
            }
            found_struct.ok_or_else(|| {
                CodegenError::Unsupported(format!("struct not found: {}", name_data))
            })?
        }
        _ => {
            return Err(CodegenError::Unsupported(format!(
                "construct non-struct type: {:?}",
                mir_ty
            )));
        }
    };

    // Get struct layout with field offsets
    let struct_layout = ctx.layouts.struct_layout(struct_id);
    let layout = struct_layout.layout;
    let field_offsets = struct_layout.field_offsets.clone();

    // Allocate stack slot for the struct
    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        layout.size as u32,
        layout.align as u8,
    ));

    // Get pointer type for the target
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    // Get pointer to the stack slot
    let ptr = builder.ins().stack_addr(ptr_type, slot, 0);

    // Store each field at its offset
    let struct_def = ctx.mir.struct_def(struct_id);
    for (field_name, field_value) in fields {
        let offset = field_offsets
            .get(field_name)
            .ok_or_else(|| CodegenError::Unsupported(format!("unknown field: {}", field_name)))?;

        // Find the field type
        let mut field_ty = None;
        for field_id in &struct_def.fields {
            let field_def = &ctx.mir.fields[*field_id];
            if &field_def.name == field_name {
                field_ty = Some(field_def.ty);
                break;
            }
        }
        let field_ty = field_ty.ok_or_else(|| {
            CodegenError::Unsupported(format!("field type not found: {}", field_name))
        })?;

        // Compile the field value
        let value = compile_value(ctx, func_def, subst, field_value, builder, local_map)?;

        // Apply substitution to get the concrete field type
        // This is important for generic structs where field types might be type parameters
        let concrete_field_ty = subst
            .apply_ty_readonly(ctx.mir, field_ty)
            .unwrap_or(field_ty);

        // Check if this is a nested struct - if so, copy the struct data
        let field_mir_ty = ctx.mir.ty(concrete_field_ty);
        let is_nested_struct =
            matches!(field_mir_ty, MirTy::Named { .. }) && is_struct_type(ctx, concrete_field_ty);

        if is_nested_struct {
            // Value is a pointer to the nested struct - copy its contents
            let nested_layout = ctx.layouts.layout_of(concrete_field_ty);
            let dest_ptr = if *offset == 0 {
                ptr
            } else {
                builder.ins().iadd_imm(ptr, *offset as i64)
            };
            // Copy the struct data byte by byte (simple approach)
            // For larger structs, we could use memcpy, but for now just copy word by word
            let words = (nested_layout.size + 7) / 8;
            for i in 0..words {
                let word_offset = (i * 8) as i32;
                let word = builder
                    .ins()
                    .load(cl_types::I64, MemFlags::new(), value, word_offset);
                builder
                    .ins()
                    .store(MemFlags::new(), word, dest_ptr, word_offset);
            }
        } else {
            // Store primitive value directly
            builder
                .ins()
                .store(MemFlags::new(), value, ptr, *offset as i32);
        }
    }

    // Return the pointer to the struct
    Ok(ptr)
}

/// Compile a tuple construction.
///
/// Allocates stack space for the tuple, stores each element at its offset,
/// and returns a pointer to the stack slot.
fn compile_tuple(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    values: &[Value],
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<CraneliftValue, CodegenError> {
    // Calculate tuple layout by laying out elements sequentially
    let mut offsets = Vec::with_capacity(values.len());
    let mut element_layouts = Vec::with_capacity(values.len());
    let mut element_types = Vec::with_capacity(values.len());
    let mut current_offset = 0usize;
    let mut max_align = 1usize;

    // First pass: compute element layouts and offsets
    for value in values {
        let (elem_layout, elem_ty) = get_value_layout(ctx, value, local_map)?;

        // Align to element's alignment
        current_offset = (current_offset + elem_layout.align - 1) & !(elem_layout.align - 1);
        offsets.push(current_offset);
        element_layouts.push(elem_layout);
        element_types.push(elem_ty);

        current_offset += elem_layout.size;
        max_align = max_align.max(elem_layout.align);
    }

    // Pad to overall alignment
    let total_size = (current_offset + max_align - 1) & !(max_align - 1);
    // Ensure minimum size of 1 byte for empty tuples
    let total_size = if total_size == 0 { 1 } else { total_size };
    let max_align = if max_align == 0 { 1 } else { max_align };

    // Allocate stack slot for the tuple
    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        total_size as u32,
        max_align as u8,
    ));

    // Get pointer type for the target
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    // Get pointer to the stack slot
    let ptr = builder.ins().stack_addr(ptr_type, slot, 0);

    // Store each element at its offset
    for (i, value) in values.iter().enumerate() {
        let offset = offsets[i];
        let elem_layout = element_layouts[i];
        let elem_ty = element_types[i];

        // Compile the element value
        let val = compile_value(ctx, func_def, subst, value, builder, local_map)?;

        // Check if this is a nested compound type - if so, copy the data
        let is_compound = if let Some(ty) = elem_ty {
            // Apply substitution to get concrete type for generic tuples
            let concrete_ty = subst.apply_ty_readonly(ctx.mir, ty).unwrap_or(ty);
            let elem_mir_ty = ctx.mir.ty(concrete_ty);
            matches!(elem_mir_ty, MirTy::Named { .. } | MirTy::Tuple(_))
                && (is_struct_type(ctx, concrete_ty) || matches!(elem_mir_ty, MirTy::Tuple(_)))
        } else {
            false
        };

        if is_compound {
            // Value is a pointer to the nested compound type - copy its contents
            let dest_ptr = if offset == 0 {
                ptr
            } else {
                builder.ins().iadd_imm(ptr, offset as i64)
            };
            // Copy the data word by word
            let words = (elem_layout.size + 7) / 8;
            for w in 0..words {
                let word_offset = (w * 8) as i32;
                let word = builder
                    .ins()
                    .load(cl_types::I64, MemFlags::new(), val, word_offset);
                builder
                    .ins()
                    .store(MemFlags::new(), word, dest_ptr, word_offset);
            }
        } else {
            // Store primitive value directly
            builder
                .ins()
                .store(MemFlags::new(), val, ptr, offset as i32);
        }
    }

    // Return the pointer to the tuple
    Ok(ptr)
}

/// Get the layout of a value and optionally its type ID.
/// Returns (Layout, Option<type_id>).
fn get_value_layout(
    ctx: &mut CodegenContext<'_>,
    value: &Value,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<(kestrel_codegen::Layout, Option<Id<Ty>>), CodegenError> {
    match value {
        Value::Place(place) => {
            let ty = get_place_type(ctx, place, local_map)?;
            let layout = ctx.layouts.layout_of(ty);
            Ok((layout, Some(ty)))
        }
        Value::Immediate(imm) => {
            let layout = get_immediate_layout(ctx, imm)?;
            Ok((layout, None))
        }
    }
}

/// Get the layout of an immediate value.
fn get_immediate_layout(
    ctx: &mut CodegenContext<'_>,
    imm: &Immediate,
) -> Result<kestrel_codegen::Layout, CodegenError> {
    use kestrel_codegen::Layout;

    match &imm.kind {
        ImmediateKind::IntLiteral { bits, .. } => {
            let layout = match bits {
                IntBits::I8 => Layout::new(1, 1),
                IntBits::I16 => Layout::new(2, 2),
                IntBits::I32 => Layout::new(4, 4),
                IntBits::I64 => Layout::new(8, 8),
            };
            Ok(layout)
        }
        ImmediateKind::FloatLiteral { bits, .. } => {
            let layout = match bits {
                FloatBits::F16 => Layout::new(2, 2),
                FloatBits::F32 => Layout::new(4, 4),
                FloatBits::F64 => Layout::new(8, 8),
            };
            Ok(layout)
        }
        ImmediateKind::BoolLiteral(_) => Ok(Layout::new(1, 1)),
        ImmediateKind::Unit => Ok(Layout::new(0, 1)),
        ImmediateKind::StringLiteral(_) => {
            // String is a fat pointer: { ptr, len }
            let ptr_size = ctx.target.pointer_size();
            Ok(Layout::new(ptr_size * 2, ptr_size))
        }
        ImmediateKind::NullPtr(ty) => {
            let layout = ctx.layouts.layout_of(*ty);
            Ok(layout)
        }
        ImmediateKind::FunctionRef { .. } => {
            // Function references are pointer-sized
            let ptr_size = ctx.target.pointer_size();
            Ok(Layout::new(ptr_size, ptr_size))
        }
        ImmediateKind::WitnessMethod { .. } => {
            Err(CodegenError::Unsupported("witness method layout".into()))
        }
        ImmediateKind::Error => Err(CodegenError::Unsupported("error immediate".into())),
    }
}

/// Compile an enum variant construction.
///
/// Allocates stack space for the enum (discriminant + max payload size),
/// stores the discriminant, then stores the payload fields.
fn compile_enum_variant(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    enum_ty: Id<Ty>,
    variant: &str,
    payload: &[Value],
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<CraneliftValue, CodegenError> {
    let mir_ty = ctx.mir.ty(enum_ty);

    // Find the enum ID from the type
    let enum_id = match mir_ty {
        MirTy::Named { name, .. } => {
            let name_data = ctx.mir.name(*name);
            let mut found_enum = None;
            for (id, def) in ctx.mir.enums.iter() {
                let def_name = ctx.mir.name(def.name);
                if def_name == name_data {
                    found_enum = Some(id);
                    break;
                }
            }
            found_enum.ok_or_else(|| {
                CodegenError::Unsupported(format!("enum not found: {}", name_data))
            })?
        }
        _ => {
            return Err(CodegenError::Unsupported(format!(
                "enum variant on non-named type: {:?}",
                mir_ty
            )));
        }
    };

    // Get the enum layout
    let enum_layout = ctx.layouts.layout_of(enum_ty);

    // Allocate stack slot for the enum
    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        enum_layout.size as u32,
        enum_layout.align as u8,
    ));

    // Get pointer type for the target
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    // Get pointer to the stack slot
    let ptr = builder.ins().stack_addr(ptr_type, slot, 0);

    // Find the case and its discriminant
    let enum_def = ctx.mir.enum_def(enum_id);
    let case_id = enum_def
        .case_by_name(variant)
        .ok_or_else(|| CodegenError::Unsupported(format!("enum case not found: {}", variant)))?;
    let case_def = &ctx.mir.enum_cases[case_id];
    let discriminant = case_def.discriminant;

    // Store the discriminant at offset 0 (i32)
    let discr_val = builder.ins().iconst(cl_types::I32, discriminant as i64);
    builder.ins().store(MemFlags::new(), discr_val, ptr, 0);

    // If there's a payload, store the fields after the discriminant
    if !payload.is_empty() {
        // Get the payload struct layout
        let payload_struct_id = case_def.struct_def.ok_or_else(|| {
            CodegenError::Unsupported(format!("enum case {} has no struct_def", variant))
        })?;
        let payload_layout = ctx.layouts.struct_layout(payload_struct_id);
        let field_offsets = payload_layout.field_offsets.clone();

        // Discriminant is 4 bytes, payload starts at offset 4 (or aligned)
        // The payload offset is after discriminant, aligned to payload's alignment
        let payload_base_offset = 4i32; // discriminant is i32 = 4 bytes

        // Get the struct definition to find field names in order
        let payload_struct = ctx.mir.struct_def(payload_struct_id);
        let field_ids: Vec<_> = payload_struct.fields.clone();

        for (i, value) in payload.iter().enumerate() {
            if i >= field_ids.len() {
                break;
            }
            let field_id = field_ids[i];
            let field_def = &ctx.mir.fields[field_id];
            let field_name = &field_def.name;

            let field_offset = field_offsets.get(field_name).copied().unwrap_or(0);
            let total_offset = payload_base_offset + field_offset as i32;

            // Compile the payload value
            let val = compile_value(ctx, func_def, subst, value, builder, local_map)?;

            // Check if this is a nested struct
            let field_ty = field_def.ty;
            // Apply substitution to get concrete type for generic enums
            let concrete_field_ty = subst
                .apply_ty_readonly(ctx.mir, field_ty)
                .unwrap_or(field_ty);
            let field_mir_ty = ctx.mir.ty(concrete_field_ty);
            let is_nested_struct = matches!(field_mir_ty, MirTy::Named { .. })
                && is_struct_type(ctx, concrete_field_ty);

            if is_nested_struct {
                // Copy nested struct data
                let nested_layout = ctx.layouts.layout_of(concrete_field_ty);
                let dest_ptr = if total_offset == 0 {
                    ptr
                } else {
                    builder.ins().iadd_imm(ptr, total_offset as i64)
                };
                let words = (nested_layout.size + 7) / 8;
                for w in 0..words {
                    let word_offset = (w * 8) as i32;
                    let word = builder
                        .ins()
                        .load(cl_types::I64, MemFlags::new(), val, word_offset);
                    builder
                        .ins()
                        .store(MemFlags::new(), word, dest_ptr, word_offset);
                }
            } else {
                // Store primitive value directly
                builder.ins().store(MemFlags::new(), val, ptr, total_offset);
            }
        }
    }

    Ok(ptr)
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

/// Compile a reference operation (Ref or RefMut).
///
/// Taking a reference means getting the address of a place.
/// For primitives stored in Variables, we need to spill them to a stack slot first.
fn compile_ref(
    ctx: &mut CodegenContext<'_>,
    place: &Place,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
    subst: &Substitution,
) -> Result<CraneliftValue, CodegenError> {
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    match &place.kind {
        PlaceKind::Local(local_id) => {
            // For a local variable, we need to get its address.
            // The local might be stored in a Variable (SSA register).
            // We need to spill it to a stack slot to get an address.
            let local_def = ctx.mir.local(*local_id);
            let local_ty = local_def.ty;

            // Apply substitution to get concrete type for generic locals
            let concrete_local_ty = subst
                .apply_ty_readonly(ctx.mir, local_ty)
                .unwrap_or(local_ty);
            let mir_ty = ctx.mir.ty(concrete_local_ty);

            // Check if this is a struct type (already a pointer)
            let is_struct =
                matches!(mir_ty, MirTy::Named { .. }) && is_struct_type(ctx, concrete_local_ty);

            if is_struct {
                // Structs are already represented as pointers, just return the value
                let var = local_map
                    .get(local_id)
                    .ok_or_else(|| CodegenError::Unsupported("unknown local".to_string()))?;
                Ok(builder.use_var(*var))
            } else {
                // Primitive - need to spill to stack and return the address
                let var = local_map
                    .get(local_id)
                    .ok_or_else(|| CodegenError::Unsupported("unknown local".to_string()))?;
                let value = builder.use_var(*var);

                // Get the size of the type
                let layout = ctx.layouts.layout_of(local_ty);

                // Create a stack slot
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    layout.size as u32,
                    layout.align as u8,
                ));

                // Get address of the slot
                let addr = builder.ins().stack_addr(ptr_type, slot, 0);

                // Store the value
                builder.ins().store(MemFlags::new(), value, addr, 0);

                Ok(addr)
            }
        }

        PlaceKind::Field { parent, name } => {
            // Taking a reference to a field means getting the field's address
            // The parent is a struct pointer, so we compute: parent_ptr + field_offset
            let struct_ptr = compile_place_read(ctx, parent, builder, local_map, subst)?;

            // Get the field offset
            let parent_ty = get_place_type(ctx, parent, local_map)?;
            let (field_offset, _field_ty) = get_field_info(ctx, parent_ty, name)?;

            if field_offset == 0 {
                Ok(struct_ptr)
            } else {
                Ok(builder.ins().iadd_imm(struct_ptr, field_offset as i64))
            }
        }

        PlaceKind::Index { parent, index } => {
            // Taking a reference to an indexed element
            let parent_ptr = compile_place_read(ctx, parent, builder, local_map, subst)?;

            // Get the field offset
            let parent_ty = get_place_type(ctx, parent, local_map)?;
            let (field_offset, _field_ty) = get_field_by_index(ctx, parent, parent_ty, *index)?;

            if field_offset == 0 {
                Ok(parent_ptr)
            } else {
                Ok(builder.ins().iadd_imm(parent_ptr, field_offset as i64))
            }
        }

        PlaceKind::Downcast { parent, variant: _ } => {
            // Taking a reference to a downcast - return pointer to payload
            let enum_ptr = compile_place_read(ctx, parent, builder, local_map, subst)?;
            Ok(builder.ins().iadd_imm(enum_ptr, 4))
        }

        PlaceKind::Deref(inner) => {
            // Taking a reference to a dereference: &*ptr is just ptr
            compile_place_read(ctx, inner, builder, local_map, subst)
        }
    }
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
            let parent_ty = get_place_type(ctx, parent, local_map)?;
            let (_, field_ty) = get_field_by_index(ctx, parent, parent_ty, *index)?;
            Ok(field_ty)
        }

        PlaceKind::Downcast { parent, .. } => get_place_type(ctx, parent, local_map),

        PlaceKind::Deref(inner) => {
            let inner_ty = get_place_type(ctx, inner, local_map)?;
            get_pointee_type(ctx, inner_ty)
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

/// Get field offset and type by index for a struct or tuple.
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

/// Compile a pointer offset operation.
fn compile_ptr_offset(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    ptr: &Value,
    offset: &Value,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<CraneliftValue, CodegenError> {
    let ptr_val = compile_value(ctx, func_def, subst, ptr, builder, local_map)?;
    let offset_val = compile_value(ctx, func_def, subst, offset, builder, local_map)?;

    // For now, assume offset is in bytes
    // TODO: If we need ptr + n * sizeof(pointee), we'd need the pointee type
    Ok(builder.ins().iadd(ptr_val, offset_val))
}

/// Compile a value (place or immediate).
pub fn compile_value(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    value: &Value,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<CraneliftValue, CodegenError> {
    match value {
        Value::Place(place) => compile_place_read(ctx, place, builder, local_map, subst),
        Value::Immediate(imm) => compile_immediate(ctx, subst, imm, builder),
    }
}

/// Compile an immediate value.
fn compile_immediate(
    ctx: &mut CodegenContext<'_>,
    subst: &Substitution,
    imm: &Immediate,
    builder: &mut FunctionBuilder<'_>,
) -> Result<CraneliftValue, CodegenError> {
    match &imm.kind {
        ImmediateKind::IntLiteral { bits, value } => {
            let cl_type = match bits {
                IntBits::I8 => cl_types::I8,
                IntBits::I16 => cl_types::I16,
                IntBits::I32 => cl_types::I32,
                IntBits::I64 => cl_types::I64,
            };
            Ok(builder.ins().iconst(cl_type, *value as i64))
        }

        ImmediateKind::FloatLiteral { bits, value } => {
            match bits {
                FloatBits::F32 => Ok(builder.ins().f32const(*value as f32)),
                FloatBits::F64 => Ok(builder.ins().f64const(*value)),
                FloatBits::F16 => {
                    // F16 needs special handling
                    Err(CodegenError::Unsupported("f16 literals".to_string()))
                }
            }
        }

        ImmediateKind::BoolLiteral(b) => {
            Ok(builder.ins().iconst(cl_types::I8, if *b { 1 } else { 0 }))
        }

        ImmediateKind::Unit => {
            // Unit is zero-sized, return dummy value
            Ok(builder.ins().iconst(cl_types::I8, 0))
        }

        ImmediateKind::StringLiteral(s) => compile_string_literal(ctx, s, builder),

        ImmediateKind::FunctionRef { name, type_args } => {
            // Get the function address as a pointer value
            let ptr_type = if ctx.target.is_64bit() {
                cl_types::I64
            } else {
                cl_types::I32
            };

            // Apply substitution to type args
            let concrete_args: Vec<_> = type_args
                .iter()
                .map(|ty| subst.apply_ty_readonly(ctx.mir, *ty).unwrap_or(*ty))
                .collect();

            // Look up the function by its mangled name
            let mangled_name = mangle_name(ctx.mir, *name, &concrete_args);
            let cl_func_id = ctx.func_ids_by_name.get(&mangled_name).ok_or_else(|| {
                CodegenError::Unsupported(format!(
                    "function not found for reference: {} (mangled: {})",
                    ctx.mir.name(*name),
                    mangled_name
                ))
            })?;

            // Get the function reference for use in this function
            let func_ref = ctx.module.declare_func_in_func(*cl_func_id, builder.func);
            // Get the address of the function
            let func_ptr = builder.ins().func_addr(ptr_type, func_ref);
            Ok(func_ptr)
        }

        ImmediateKind::WitnessMethod {
            protocol,
            method,
            for_type,
        } => {
            // Apply substitution to for_type
            let concrete_for_type = subst
                .apply_ty_readonly(ctx.mir, *for_type)
                .unwrap_or(*for_type);

            // Resolve the witness to get the concrete function
            let (impl_name, impl_type_args) =
                resolve_witness(ctx.mir, *protocol, method, concrete_for_type)?;

            // Get the function address
            let ptr_type = if ctx.target.is_64bit() {
                cl_types::I64
            } else {
                cl_types::I32
            };

            let mangled_name = mangle_name(ctx.mir, impl_name, &impl_type_args);
            let cl_func_id = ctx.func_ids_by_name.get(&mangled_name).ok_or_else(|| {
                CodegenError::Unsupported(format!(
                    "witness method function not found: {} (mangled: {})",
                    ctx.mir.name(impl_name),
                    mangled_name
                ))
            })?;

            let func_ref = ctx.module.declare_func_in_func(*cl_func_id, builder.func);
            let func_ptr = builder.ins().func_addr(ptr_type, func_ref);
            Ok(func_ptr)
        }

        ImmediateKind::NullPtr(_) => Ok(builder.ins().iconst(cl_types::I64, 0)),

        ImmediateKind::Error => Err(CodegenError::Unsupported("error immediate".to_string())),
    }
}

/// Compile a string literal.
///
/// String literals are compiled as fat pointers: { ptr_to_data, length }.
/// The string content is stored in the binary's data section.
fn compile_string_literal(
    ctx: &mut CodegenContext<'_>,
    s: &str,
    builder: &mut FunctionBuilder<'_>,
) -> Result<CraneliftValue, CodegenError> {
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    // Add string to data section
    let data_id = ctx.add_string_data(s)?;

    // Get reference to the string data in this function
    let data_ref = ctx.module.declare_data_in_func(data_id, builder.func);
    let str_ptr = builder.ins().global_value(ptr_type, data_ref);

    // Create length constant
    let str_len = builder.ins().iconst(ptr_type, s.len() as i64);

    // Allocate stack slot for the fat pointer struct (ptr + len = 16 bytes on 64-bit)
    let ptr_size = if ctx.target.is_64bit() { 8 } else { 4 };
    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        (ptr_size * 2) as u32,
        ptr_size as u8,
    ));
    let struct_ptr = builder.ins().stack_addr(ptr_type, slot, 0);

    // Store ptr at offset 0
    builder.ins().store(MemFlags::new(), str_ptr, struct_ptr, 0);
    // Store len at offset ptr_size
    builder
        .ins()
        .store(MemFlags::new(), str_len, struct_ptr, ptr_size as i32);

    Ok(struct_ptr)
}

/// Compile a binary operation.
fn compile_binop(
    ctx: &CodegenContext<'_>,
    op: BinOp,
    lhs: CraneliftValue,
    rhs: CraneliftValue,
    builder: &mut FunctionBuilder<'_>,
) -> Result<CraneliftValue, CodegenError> {
    let result = match op {
        // Signed integer arithmetic
        BinOp::AddSigned => builder.ins().iadd(lhs, rhs),
        BinOp::SubSigned => builder.ins().isub(lhs, rhs),
        BinOp::MulSigned => builder.ins().imul(lhs, rhs),
        BinOp::DivSigned => builder.ins().sdiv(lhs, rhs),
        BinOp::RemSigned => builder.ins().srem(lhs, rhs),

        // Unsigned integer arithmetic
        BinOp::AddUnsigned => builder.ins().iadd(lhs, rhs),
        BinOp::SubUnsigned => builder.ins().isub(lhs, rhs),
        BinOp::MulUnsigned => builder.ins().imul(lhs, rhs),
        BinOp::DivUnsigned => builder.ins().udiv(lhs, rhs),
        BinOp::RemUnsigned => builder.ins().urem(lhs, rhs),

        // Float arithmetic
        BinOp::FAdd => builder.ins().fadd(lhs, rhs),
        BinOp::FSub => builder.ins().fsub(lhs, rhs),
        BinOp::FMul => builder.ins().fmul(lhs, rhs),
        BinOp::FDiv => builder.ins().fdiv(lhs, rhs),

        // Bitwise operations
        BinOp::And => builder.ins().band(lhs, rhs),
        BinOp::Or => builder.ins().bor(lhs, rhs),
        BinOp::Xor => builder.ins().bxor(lhs, rhs),
        BinOp::Shl => builder.ins().ishl(lhs, rhs),
        BinOp::ShrSigned => builder.ins().sshr(lhs, rhs),
        BinOp::ShrUnsigned => builder.ins().ushr(lhs, rhs),

        // Integer comparisons
        // Note: icmp returns I8 on most platforms, no need to extend
        BinOp::Eq => builder
            .ins()
            .icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, lhs, rhs),
        BinOp::Ne => {
            builder
                .ins()
                .icmp(cranelift_codegen::ir::condcodes::IntCC::NotEqual, lhs, rhs)
        }
        BinOp::LtSigned => builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedLessThan,
            lhs,
            rhs,
        ),
        BinOp::LeSigned => builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedLessThanOrEqual,
            lhs,
            rhs,
        ),
        BinOp::GtSigned => builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThan,
            lhs,
            rhs,
        ),
        BinOp::GeSigned => builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThanOrEqual,
            lhs,
            rhs,
        ),
        BinOp::LtUnsigned => builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::UnsignedLessThan,
            lhs,
            rhs,
        ),
        BinOp::LeUnsigned => builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::UnsignedLessThanOrEqual,
            lhs,
            rhs,
        ),
        BinOp::GtUnsigned => builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::UnsignedGreaterThan,
            lhs,
            rhs,
        ),
        BinOp::GeUnsigned => builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::UnsignedGreaterThanOrEqual,
            lhs,
            rhs,
        ),

        // Float comparisons
        // Note: fcmp returns I8 on most platforms, no need to extend
        BinOp::FEq => {
            builder
                .ins()
                .fcmp(cranelift_codegen::ir::condcodes::FloatCC::Equal, lhs, rhs)
        }
        BinOp::FNe => builder.ins().fcmp(
            cranelift_codegen::ir::condcodes::FloatCC::NotEqual,
            lhs,
            rhs,
        ),
        BinOp::FLt => builder.ins().fcmp(
            cranelift_codegen::ir::condcodes::FloatCC::LessThan,
            lhs,
            rhs,
        ),
        BinOp::FLe => builder.ins().fcmp(
            cranelift_codegen::ir::condcodes::FloatCC::LessThanOrEqual,
            lhs,
            rhs,
        ),
        BinOp::FGt => builder.ins().fcmp(
            cranelift_codegen::ir::condcodes::FloatCC::GreaterThan,
            lhs,
            rhs,
        ),
        BinOp::FGe => builder.ins().fcmp(
            cranelift_codegen::ir::condcodes::FloatCC::GreaterThanOrEqual,
            lhs,
            rhs,
        ),

        // Boolean operations
        BinOp::BoolAnd => builder.ins().band(lhs, rhs),
        BinOp::BoolOr => builder.ins().bor(lhs, rhs),
    };

    Ok(result)
}

/// Compile a unary operation.
fn compile_unop(
    ctx: &CodegenContext<'_>,
    op: UnOp,
    operand: CraneliftValue,
    builder: &mut FunctionBuilder<'_>,
) -> Result<CraneliftValue, CodegenError> {
    let result = match op {
        UnOp::Neg => builder.ins().ineg(operand),
        UnOp::FNeg => builder.ins().fneg(operand),
        UnOp::Not => builder.ins().bnot(operand),
        UnOp::BoolNot => {
            // Boolean not: xor with 1
            let one = builder.ins().iconst(cl_types::I8, 1);
            builder.ins().bxor(operand, one)
        }
    };

    Ok(result)
}

/// Compile a function call.
///
/// Returns the return value of the call. For unit-returning functions,
/// returns a dummy I8 value.
pub fn compile_call(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    callee: &Callee,
    args: &[CallArg],
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<CraneliftValue, CodegenError> {
    match callee {
        Callee::Direct { name, type_args } => {
            // Apply substitution to type args
            let concrete_args: Vec<_> = type_args
                .iter()
                .map(|ty| subst.apply_ty_readonly(ctx.mir, *ty).unwrap_or(*ty))
                .collect();

            // Look up the Cranelift FuncId for this function.
            // For extern functions, use the symbol name from extern_info.
            // Otherwise, use the mangled name.
            let callee_def = ctx
                .mir
                .functions
                .iter()
                .find(|(_, def)| def.name == *name)
                .map(|(_, def)| def);

            let lookup_name = match callee_def {
                Some(def) if def.extern_info.is_some() => {
                    def.extern_info.as_ref().unwrap().symbol_name.clone()
                }
                _ => mangle_name(ctx.mir, *name, &concrete_args),
            };

            let cl_func_id = ctx.func_ids_by_name.get(&lookup_name).ok_or_else(|| {
                CodegenError::Unsupported(format!(
                    "function not found: {} (lookup: {})",
                    ctx.mir.name(*name),
                    lookup_name
                ))
            })?;

            // Get the function reference for use in this function
            let func_ref = ctx.module.declare_func_in_func(*cl_func_id, builder.func);

            // Compile arguments
            let mut arg_values = Vec::with_capacity(args.len());
            for arg in args {
                let val = compile_value(ctx, func_def, subst, &arg.value, builder, local_map)?;
                arg_values.push(val);
            }

            // Emit the call instruction
            let call_inst = builder.ins().call(func_ref, &arg_values);

            // Get the return value (if any)
            let results = builder.inst_results(call_inst);
            if results.is_empty() {
                // Unit return - return a dummy value
                Ok(builder.ins().iconst(cl_types::I8, 0))
            } else {
                Ok(results[0])
            }
        }

        Callee::Thin(place) => {
            compile_thin_call(ctx, func_def, subst, place, args, builder, local_map)
        }

        Callee::Thick(place) => {
            compile_thick_call(ctx, func_def, subst, place, args, builder, local_map)
        }

        Callee::Witness {
            protocol,
            method,
            for_type,
        } => {
            // Apply substitution to for_type
            let concrete_for_type = subst
                .apply_ty_readonly(ctx.mir, *for_type)
                .unwrap_or(*for_type);

            // Resolve the witness to get the concrete implementation
            let (impl_name, impl_type_args) =
                resolve_witness(ctx.mir, *protocol, method, concrete_for_type)?;

            // Look up the function
            let mangled_name = mangle_name(ctx.mir, impl_name, &impl_type_args);
            let cl_func_id = ctx.func_ids_by_name.get(&mangled_name).ok_or_else(|| {
                CodegenError::Unsupported(format!(
                    "witness method function not found: {} (mangled: {})",
                    ctx.mir.name(impl_name),
                    mangled_name
                ))
            })?;

            let func_ref = ctx.module.declare_func_in_func(*cl_func_id, builder.func);

            // Compile arguments
            let mut arg_values = Vec::with_capacity(args.len());
            for arg in args {
                let val = compile_value(ctx, func_def, subst, &arg.value, builder, local_map)?;
                arg_values.push(val);
            }

            // Emit the call instruction
            let call_inst = builder.ins().call(func_ref, &arg_values);

            // Get the return value (if any)
            let results = builder.inst_results(call_inst);
            if results.is_empty() {
                Ok(builder.ins().iconst(cl_types::I8, 0))
            } else {
                Ok(results[0])
            }
        }
    }
}

/// Compile a type cast operation.
fn compile_cast(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    kind: CastKind,
    operand: &Value,
    target: Id<Ty>,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<CraneliftValue, CodegenError> {
    let val = compile_value(ctx, func_def, subst, operand, builder, local_map)?;
    let target_ty = translate_type(ctx.mir, target, ctx.target);

    match kind {
        CastKind::IntWiden => {
            // Integer widening - sign-extend to larger integer type
            // Kestrel integers are signed, so use sextend
            let src_ty = builder.func.dfg.value_type(val);
            if target_ty.bits() > src_ty.bits() {
                Ok(builder.ins().sextend(target_ty, val))
            } else {
                // Same size or smaller - this shouldn't happen for IntWiden
                // but handle gracefully
                Ok(val)
            }
        }

        CastKind::IntTruncate => {
            // Integer narrowing - truncate to smaller integer type
            let src_ty = builder.func.dfg.value_type(val);
            if target_ty.bits() < src_ty.bits() {
                Ok(builder.ins().ireduce(target_ty, val))
            } else {
                // Same size or larger - this shouldn't happen for IntTruncate
                Ok(val)
            }
        }

        CastKind::IntToFloat => {
            // Convert signed integer to float
            Ok(builder.ins().fcvt_from_sint(target_ty, val))
        }

        CastKind::FloatToInt => {
            // Convert float to signed integer
            // Use fcvt_to_sint_sat for saturating conversion (safer, no undefined behavior)
            Ok(builder.ins().fcvt_to_sint_sat(target_ty, val))
        }

        CastKind::FloatWiden => {
            // f32 -> f64 promotion
            Ok(builder.ins().fpromote(target_ty, val))
        }

        CastKind::FloatTruncate => {
            // f64 -> f32 demotion
            Ok(builder.ins().fdemote(target_ty, val))
        }

        CastKind::PtrBitcast => {
            // Pointer bitcast - same representation, just reinterpret the type
            // At the IR level, all pointers have the same representation
            Ok(val)
        }

        CastKind::RefToImmut => {
            // &var T -> &T conversion - same representation, just type change
            Ok(val)
        }
    }
}

/// Compile str.ptr operation - extract the pointer from a string fat pointer.
fn compile_str_ptr(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    value: &Value,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<CraneliftValue, CodegenError> {
    let str_ptr = compile_value(ctx, func_def, subst, value, builder, local_map)?;

    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    // String is a fat pointer: { ptr: p[i8], len: i64 }
    // Load the ptr field at offset 0
    Ok(builder.ins().load(ptr_type, MemFlags::new(), str_ptr, 0))
}

/// Compile str.len operation - extract the length from a string fat pointer.
fn compile_str_len(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    value: &Value,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<CraneliftValue, CodegenError> {
    let str_ptr = compile_value(ctx, func_def, subst, value, builder, local_map)?;

    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };
    let ptr_size = if ctx.target.is_64bit() { 8 } else { 4 };

    // String is a fat pointer: { ptr: p[i8], len: i64 }
    // Load the len field at offset ptr_size
    Ok(builder
        .ins()
        .load(ptr_type, MemFlags::new(), str_ptr, ptr_size))
}

/// Compile str.from_parts operation - create a string fat pointer from ptr and len.
fn compile_str_from_parts(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    ptr: &Value,
    len: &Value,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<CraneliftValue, CodegenError> {
    let ptr_val = compile_value(ctx, func_def, subst, ptr, builder, local_map)?;
    let len_val = compile_value(ctx, func_def, subst, len, builder, local_map)?;

    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };
    let ptr_size: i32 = if ctx.target.is_64bit() { 8 } else { 4 };

    // Allocate stack slot for the fat pointer struct (ptr + len)
    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        (ptr_size * 2) as u32,
        ptr_size as u8,
    ));
    let struct_ptr = builder.ins().stack_addr(ptr_type, slot, 0);

    // Store ptr at offset 0
    builder.ins().store(MemFlags::new(), ptr_val, struct_ptr, 0);
    // Store len at offset ptr_size
    builder
        .ins()
        .store(MemFlags::new(), len_val, struct_ptr, ptr_size);

    Ok(struct_ptr)
}

/// Get the type of a place expression (for determining function signature in indirect calls).
fn get_place_type_for_call(
    ctx: &CodegenContext<'_>,
    place: &Place,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<Id<Ty>, CodegenError> {
    match &place.kind {
        PlaceKind::Local(local_id) => {
            let local_def = ctx.mir.local(*local_id);
            Ok(local_def.ty)
        }

        PlaceKind::Field { parent, name } => {
            let parent_ty = get_place_type_for_call(ctx, parent, local_map)?;
            get_field_type_for_call(ctx, parent_ty, name)
        }

        PlaceKind::Index { parent, index } => {
            let parent_ty = get_place_type_for_call(ctx, parent, local_map)?;
            get_field_type_by_index_for_call(ctx, parent_ty, *index)
        }

        PlaceKind::Downcast { parent, .. } => get_place_type_for_call(ctx, parent, local_map),

        PlaceKind::Deref(inner) => {
            let inner_ty = get_place_type_for_call(ctx, inner, local_map)?;
            match ctx.mir.ty(inner_ty) {
                MirTy::Pointer(pointee) | MirTy::Ref(pointee) | MirTy::RefMut(pointee) => {
                    Ok(*pointee)
                }
                _ => Err(CodegenError::Unsupported(format!(
                    "deref of non-pointer type: {:?}",
                    ctx.mir.ty(inner_ty)
                ))),
            }
        }
    }
}

/// Get the type of a field by name.
fn get_field_type_for_call(
    ctx: &CodegenContext<'_>,
    parent_ty: Id<Ty>,
    field_name: &str,
) -> Result<Id<Ty>, CodegenError> {
    let mir_ty = ctx.mir.ty(parent_ty);

    if let MirTy::Named { name, .. } = mir_ty {
        let name_data = ctx.mir.name(*name);
        for (_struct_id, def) in ctx.mir.structs.iter() {
            if ctx.mir.name(def.name) == name_data {
                for field_id in &def.fields {
                    let field_def = &ctx.mir.fields[*field_id];
                    if field_def.name == field_name {
                        return Ok(field_def.ty);
                    }
                }
            }
        }
    }

    Err(CodegenError::Unsupported(format!(
        "field {} not found in type {:?}",
        field_name, mir_ty
    )))
}

/// Get the type of a field by index.
fn get_field_type_by_index_for_call(
    ctx: &CodegenContext<'_>,
    parent_ty: Id<Ty>,
    index: usize,
) -> Result<Id<Ty>, CodegenError> {
    let mir_ty = ctx.mir.ty(parent_ty);

    match mir_ty {
        MirTy::Tuple(elements) => {
            if index < elements.len() {
                Ok(elements[index])
            } else {
                Err(CodegenError::Unsupported(format!(
                    "tuple index {} out of bounds",
                    index
                )))
            }
        }
        MirTy::Named { name, .. } => {
            let name_data = ctx.mir.name(*name);
            for (_struct_id, def) in ctx.mir.structs.iter() {
                if ctx.mir.name(def.name) == name_data {
                    if index < def.fields.len() {
                        let field_id = def.fields[index];
                        let field_def = &ctx.mir.fields[field_id];
                        return Ok(field_def.ty);
                    }
                }
            }
            Err(CodegenError::Unsupported(format!(
                "field index {} out of bounds in {:?}",
                index, name_data
            )))
        }
        _ => Err(CodegenError::Unsupported(format!(
            "index access on unsupported type: {:?}",
            mir_ty
        ))),
    }
}

/// Resolve through references to find the underlying function type.
fn resolve_func_type(ctx: &CodegenContext<'_>, ty: Id<Ty>) -> Id<Ty> {
    let mir_ty = ctx.mir.ty(ty);
    match mir_ty {
        MirTy::Ref(inner) | MirTy::RefMut(inner) | MirTy::Pointer(inner) => {
            resolve_func_type(ctx, *inner)
        }
        _ => ty,
    }
}

/// Build a Cranelift signature from a function type.
fn build_signature_from_func_type(
    ctx: &CodegenContext<'_>,
    func_ty: Id<Ty>,
    builder: &FunctionBuilder<'_>,
) -> Result<Signature, CodegenError> {
    let mir_ty = ctx.mir.ty(func_ty);

    let (params, ret) = match mir_ty {
        MirTy::FuncThin { params, ret } => (params.clone(), *ret),
        MirTy::FuncThick { params, ret } => (params.clone(), *ret),
        _ => {
            return Err(CodegenError::Unsupported(format!(
                "not a function type: {:?}",
                mir_ty
            )))
        }
    };

    let call_conv = builder.func.signature.call_conv;
    let mut sig = Signature::new(call_conv);

    // Add parameters
    for param_ty in &params {
        let cl_type = translate_type(ctx.mir, *param_ty, ctx.target);
        sig.params.push(AbiParam::new(cl_type));
    }

    // Add return type if not unit
    let ret_mir_ty = ctx.mir.ty(ret);
    if !matches!(ret_mir_ty, MirTy::Unit) {
        let cl_type = translate_type(ctx.mir, ret, ctx.target);
        sig.returns.push(AbiParam::new(cl_type));
    }

    Ok(sig)
}

/// Compile an ApplyPartial rvalue.
///
/// ApplyPartial creates a thick callable (closure) from a function reference and
/// captured values. The result is a struct: { func_ptr: *const (), env_ptr: *const () }
///
/// For non-capturing closures (empty captures), we still allocate an environment struct
/// (possibly zero-sized) to keep the code path uniform.
///
/// For capturing closures, we:
/// 1. Get the function pointer for the closure's call function
/// 2. Allocate stack space for the environment struct
/// 3. Store each captured value into the appropriate field
/// 4. Create the thick callable struct with (func_ptr, env_ptr)
fn compile_apply_partial(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    func: Id<QualifiedName>,
    captures: &[Value],
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<CraneliftValue, CodegenError> {
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };
    let ptr_size = if ctx.target.is_64bit() { 8 } else { 4 };

    // 1. Find the closure function and get its environment struct
    let (closure_func_id, env_struct_id) = find_closure_function_and_env(ctx, func)?;

    // 2. Get the function pointer for the closure function
    // The closure function is non-generic, so we use empty type args
    let mangled_name = mangle_name(ctx.mir, func, &[]);
    let cl_func_id = ctx.func_ids_by_name.get(&mangled_name).ok_or_else(|| {
        CodegenError::Unsupported(format!(
            "closure function not found: {} (mangled: {})",
            ctx.mir.name(func),
            mangled_name
        ))
    })?;

    let func_ref = ctx.module.declare_func_in_func(*cl_func_id, builder.func);
    let func_ptr = builder.ins().func_addr(ptr_type, func_ref);

    // 3. Allocate and populate the environment struct
    let env_ptr = if let Some(env_struct_id) = env_struct_id {
        // Get the environment struct layout
        let env_layout = ctx.layouts.struct_layout(env_struct_id);
        let layout = env_layout.layout;
        let field_offsets = env_layout.field_offsets.clone();

        // Allocate stack space for the environment
        // Use at least 1 byte to avoid zero-sized allocations
        let alloc_size = if layout.size == 0 { 1 } else { layout.size };
        let alloc_align = if layout.align == 0 { 1 } else { layout.align };

        let slot = builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            alloc_size as u32,
            alloc_align as u8,
        ));
        let env_ptr = builder.ins().stack_addr(ptr_type, slot, 0);

        // Store each capture into the environment struct
        let env_struct_def = ctx.mir.struct_def(env_struct_id);
        let field_ids: Vec<_> = env_struct_def.fields.clone();

        for (i, capture_value) in captures.iter().enumerate() {
            if i >= field_ids.len() {
                break;
            }
            let field_id = field_ids[i];
            let field_def = &ctx.mir.fields[field_id];
            let field_name = &field_def.name;
            let field_ty = field_def.ty;

            let offset = field_offsets.get(field_name).copied().unwrap_or(0);

            // Compile the capture value
            let val = compile_value(ctx, func_def, subst, capture_value, builder, local_map)?;

            // Check if this is a nested struct that needs copying
            let concrete_field_ty = subst
                .apply_ty_readonly(ctx.mir, field_ty)
                .unwrap_or(field_ty);
            let field_mir_ty = ctx.mir.ty(concrete_field_ty);
            let is_nested_struct = matches!(field_mir_ty, MirTy::Named { .. })
                && is_struct_type(ctx, concrete_field_ty);

            if is_nested_struct {
                // Copy nested struct data
                let nested_layout = ctx.layouts.layout_of(concrete_field_ty);
                let dest_ptr = if offset == 0 {
                    env_ptr
                } else {
                    builder.ins().iadd_imm(env_ptr, offset as i64)
                };
                let words = (nested_layout.size + 7) / 8;
                for w in 0..words {
                    let word_offset = (w * 8) as i32;
                    let word = builder
                        .ins()
                        .load(cl_types::I64, MemFlags::new(), val, word_offset);
                    builder
                        .ins()
                        .store(MemFlags::new(), word, dest_ptr, word_offset);
                }
            } else {
                // Store primitive value directly
                builder
                    .ins()
                    .store(MemFlags::new(), val, env_ptr, offset as i32);
            }
        }

        env_ptr
    } else {
        // No environment struct found - use null pointer
        // This shouldn't happen for properly lowered closures, but handle gracefully
        builder.ins().iconst(ptr_type, 0)
    };

    // 4. Create the thick callable struct: { func_ptr, env_ptr }
    let thick_slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        (ptr_size * 2) as u32,
        ptr_size as u8,
    ));
    let thick_ptr = builder.ins().stack_addr(ptr_type, thick_slot, 0);

    // Store func_ptr at offset 0
    builder.ins().store(MemFlags::new(), func_ptr, thick_ptr, 0);
    // Store env_ptr at offset ptr_size
    builder
        .ins()
        .store(MemFlags::new(), env_ptr, thick_ptr, ptr_size as i32);

    Ok(thick_ptr)
}

/// Find a closure function by its qualified name and return its ID along with
/// the environment struct ID (if any).
fn find_closure_function_and_env(
    ctx: &CodegenContext<'_>,
    func_name: Id<QualifiedName>,
) -> Result<(Id<Function>, Option<Id<Struct>>), CodegenError> {
    // Find the function by name
    for (func_id, func_def) in ctx.mir.functions.iter() {
        if func_def.name == func_name {
            // Check if it has ClosureCall origin with an env struct
            let env_struct_id = match &func_def.meta.origin {
                Some(Origin::ClosureCall { env_struct, .. }) => Some(*env_struct),
                _ => None,
            };
            return Ok((func_id, env_struct_id));
        }
    }

    Err(CodegenError::Unsupported(format!(
        "closure function not found: {}",
        ctx.mir.name(func_name)
    )))
}

/// Check if a local is a function parameter (passed by pointer).
fn is_parameter_local(
    func_def: &FunctionDef,
    local_id: Id<Local>,
    ctx: &CodegenContext<'_>,
) -> bool {
    for &param_id in &func_def.params {
        let param = &ctx.mir.params[param_id];
        if param.local == local_id {
            return true;
        }
    }
    false
}

/// Compile a thin function pointer call.
///
/// A thin function pointer is just an address - we load it and call indirectly.
fn compile_thin_call(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    place: &Place,
    args: &[CallArg],
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<CraneliftValue, CodegenError> {
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };

    // Get the function pointer value
    let place_value = compile_place_read(ctx, place, builder, local_map, subst)?;

    // Check if this is a parameter local - if so, we need to dereference
    // because parameters are passed by pointer.
    let func_ptr = if let PlaceKind::Local(local_id) = place.kind {
        if is_parameter_local(func_def, local_id, ctx) {
            // Load the actual function pointer from the parameter pointer
            builder
                .ins()
                .load(ptr_type, MemFlags::new(), place_value, 0)
        } else {
            // Regular local - value is the function pointer directly
            place_value
        }
    } else {
        place_value
    };

    // Get the type of the place to determine the function signature
    let func_ty = get_place_type_for_call(ctx, place, local_map)?;

    // Build the signature
    let sig = build_signature_from_func_type(ctx, func_ty, builder)?;
    let sig_ref = builder.import_signature(sig);

    // Compile arguments
    let mut arg_values = Vec::with_capacity(args.len());
    for arg in args {
        let val = compile_value(ctx, func_def, subst, &arg.value, builder, local_map)?;
        arg_values.push(val);
    }

    // Make the indirect call
    let call_inst = builder.ins().call_indirect(sig_ref, func_ptr, &arg_values);

    // Get the return value (if any)
    let results = builder.inst_results(call_inst);
    if results.is_empty() {
        // Unit return - return a dummy value
        Ok(builder.ins().iconst(cl_types::I8, 0))
    } else {
        Ok(results[0])
    }
}

/// Compile a thick function pointer (closure) call.
///
/// A thick callable has the layout: { func_ptr: *const (), env_ptr: *const () }
/// The function pointer expects the environment pointer as the first argument.
///
/// Note: The MIR lowering may use Callee::Thick for all function calls for
/// simplicity. We check the actual type and handle FuncThin types appropriately.
fn compile_thick_call(
    ctx: &mut CodegenContext<'_>,
    func_def: &FunctionDef,
    subst: &Substitution,
    place: &Place,
    args: &[CallArg],
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<CraneliftValue, CodegenError> {
    let ptr_type = if ctx.target.is_64bit() {
        cl_types::I64
    } else {
        cl_types::I32
    };
    let ptr_size = if ctx.target.is_64bit() { 8 } else { 4 };

    // Get the type of the place to determine how to handle the call
    let func_ty = get_place_type_for_call(ctx, place, local_map)?;
    let resolved_ty = resolve_func_type(ctx, func_ty);
    let mir_ty = ctx.mir.ty(resolved_ty);

    // Check if this is actually a thin function type
    // The MIR lowering may use Callee::Thick for all function calls
    match mir_ty {
        MirTy::FuncThin { params, ret } => {
            // For thin function types, the value is the function pointer directly
            // (not a struct with func_ptr + env_ptr)
            //
            // Note: If this is a parameter local, the value is a POINTER to the
            // function pointer (because Kestrel passes all parameters by pointer).
            // In that case, we need to load from it.
            let place_value = compile_place_read(ctx, place, builder, local_map, subst)?;

            // Check if this is a parameter local - if so, we need to dereference
            // because parameters are passed by pointer.
            let func_ptr = if let PlaceKind::Local(local_id) = place.kind {
                if is_parameter_local(func_def, local_id, ctx) {
                    // Load the actual function pointer from the parameter pointer
                    builder
                        .ins()
                        .load(ptr_type, MemFlags::new(), place_value, 0)
                } else {
                    // Regular local - value is the function pointer directly
                    place_value
                }
            } else {
                place_value
            };

            let call_conv = builder.func.signature.call_conv;
            let mut sig = Signature::new(call_conv);

            for param_ty in params {
                let cl_type = translate_type(ctx.mir, *param_ty, ctx.target);
                sig.params.push(AbiParam::new(cl_type));
            }

            let ret_mir_ty = ctx.mir.ty(*ret);
            if !matches!(ret_mir_ty, MirTy::Unit) {
                let cl_type = translate_type(ctx.mir, *ret, ctx.target);
                sig.returns.push(AbiParam::new(cl_type));
            }

            let sig_ref = builder.import_signature(sig);

            let mut arg_values = Vec::with_capacity(args.len());
            for arg in args {
                let val = compile_value(ctx, func_def, subst, &arg.value, builder, local_map)?;
                arg_values.push(val);
            }

            let call_inst = builder.ins().call_indirect(sig_ref, func_ptr, &arg_values);
            let results = builder.inst_results(call_inst);
            if results.is_empty() {
                return Ok(builder.ins().iconst(cl_types::I8, 0));
            } else {
                return Ok(results[0]);
            }
        }
        MirTy::FuncThick { params, ret } => {
            // For thick function types, the value is a struct with func_ptr and env_ptr
            let thick_ptr = compile_place_read(ctx, place, builder, local_map, subst)?;

            // Load the function pointer from offset 0
            let func_ptr = builder.ins().load(ptr_type, MemFlags::new(), thick_ptr, 0);

            // Load the environment pointer from offset ptr_size
            let env_ptr = builder
                .ins()
                .load(ptr_type, MemFlags::new(), thick_ptr, ptr_size as i32);

            let call_conv = builder.func.signature.call_conv;
            let mut sig = Signature::new(call_conv);

            // First parameter is the environment pointer
            sig.params.push(AbiParam::new(ptr_type));

            // Then add the regular parameters
            for param_ty in params {
                let cl_type = translate_type(ctx.mir, *param_ty, ctx.target);
                sig.params.push(AbiParam::new(cl_type));
            }

            // Add return type if not unit
            let ret_mir_ty = ctx.mir.ty(*ret);
            if !matches!(ret_mir_ty, MirTy::Unit) {
                let cl_type = translate_type(ctx.mir, *ret, ctx.target);
                sig.returns.push(AbiParam::new(cl_type));
            }

            let sig_ref = builder.import_signature(sig);

            // Compile arguments - env_ptr is the first argument
            let mut arg_values = Vec::with_capacity(args.len() + 1);
            arg_values.push(env_ptr);
            for arg in args {
                let val = compile_value(ctx, func_def, subst, &arg.value, builder, local_map)?;
                arg_values.push(val);
            }

            // Make the indirect call
            let call_inst = builder.ins().call_indirect(sig_ref, func_ptr, &arg_values);

            let results = builder.inst_results(call_inst);
            if results.is_empty() {
                return Ok(builder.ins().iconst(cl_types::I8, 0));
            } else {
                return Ok(results[0]);
            }
        }
        _ => {
            return Err(CodegenError::Unsupported(format!(
                "not a function type: {:?}",
                mir_ty
            )))
        }
    }
}
