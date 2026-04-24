//! Composite value construction — structs, tuples, enums, array/dict literals.

use crate::common::{self, get_enum_payload_offset, get_field_info, is_aggregate};
use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::function::FunctionState;
use crate::rvalue;
use crate::types;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{
    self, InstBuilder, MemFlags, StackSlotData, StackSlotKind, Value as CrValue,
};
use cranelift_frontend::FunctionBuilder;
use kestrel_codegen::{NamedKind, substitute_type, substitute_type_with_self};
use kestrel_mir::{MirTy, Value};
use std::collections::HashMap;

/// Compile struct construction: `construct Type { field: value, ... }`
pub fn compile_construct(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    ty: &MirTy,
    fields: &[(String, Value)],
) -> Result<CrValue, CodegenError> {
    let ptr_ty = common::ptr_type(ctx.target);
    let concrete_ty =
        substitute_type_with_self(ty, &state.subst, state.self_type.as_ref(), ctx.module);
    let layout = ctx.layouts.layout_of(&concrete_ty);

    // Allocate stack slot for the struct
    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        layout.size as u32,
        common::align_to_shift(layout.align),
    ));
    let addr = builder.ins().stack_addr(ptr_ty, slot, Offset32::new(0));
    common::zero_memory(builder, addr, layout.size, ptr_ty);

    // Store each field at its offset
    if let MirTy::Named { entity, type_args } = &concrete_ty {
        match ctx.layouts.resolve_named(*entity) {
            NamedKind::Struct(struct_id) => {
                let type_args = common::substitute_type_args(
                    type_args,
                    &state.subst,
                    state.self_type.as_ref(),
                    ctx.module,
                );

                for (name, value) in fields {
                    let (offset, field_ty) =
                        get_field_info(ctx.module, &mut ctx.layouts, struct_id, &type_args, name)?;
                    let val = rvalue::compile_value(ctx, state, builder, value)?;
                    let field_ptr = builder.ins().iadd_imm(addr, offset as i64);

                    if is_aggregate(&field_ty, &mut ctx.layouts) {
                        common::copy_aggregate(
                            builder,
                            &mut ctx.layouts,
                            &field_ty,
                            field_ptr,
                            val,
                        );
                    } else {
                        builder
                            .ins()
                            .store(MemFlags::new(), val, field_ptr, Offset32::new(0));
                    }
                }
            },
            _ => {
                return Err(CodegenError::Unsupported(
                    "construct non-struct Named type".into(),
                ));
            },
        }
    }

    Ok(addr)
}

/// Compile tuple construction: `tuple (v0, v1, ...)`
pub fn compile_tuple(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    values: &[Value],
) -> Result<CrValue, CodegenError> {
    let ptr_ty = common::ptr_type(ctx.target);

    // Build the tuple type to get the layout
    let elem_types: Vec<MirTy> = values
        .iter()
        .map(|v| match v {
            Value::Place(p) => common::get_place_type(
                ctx.module,
                state.body,
                p,
                &state.subst,
                state.self_type.as_ref(),
                &ctx.layouts,
            ),
            Value::Immediate(imm) => Ok(MirTy::I64), // Approximate; ideally infer from imm
        })
        .collect::<Result<_, _>>()?;

    let tuple_ty = MirTy::Tuple(elem_types.clone());
    let layout = ctx.layouts.layout_of(&tuple_ty);

    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        layout.size as u32,
        common::align_to_shift(layout.align),
    ));
    let addr = builder.ins().stack_addr(ptr_ty, slot, Offset32::new(0));
    common::zero_memory(builder, addr, layout.size, ptr_ty);

    // Store each element
    let mut offset_layout = kestrel_codegen::Layout::zero(1);
    for (i, value) in values.iter().enumerate() {
        let elem_layout = ctx.layouts.layout_of(&elem_types[i]);
        let (offset, new_layout) = offset_layout.append(elem_layout);
        offset_layout = new_layout;

        let val = rvalue::compile_value(ctx, state, builder, value)?;
        let elem_ptr = builder.ins().iadd_imm(addr, offset as i64);

        if is_aggregate(&elem_types[i], &mut ctx.layouts) {
            common::copy_aggregate(builder, &mut ctx.layouts, &elem_types[i], elem_ptr, val);
        } else {
            builder
                .ins()
                .store(MemFlags::new(), val, elem_ptr, Offset32::new(0));
        }
    }

    Ok(addr)
}

/// Compile enum variant construction.
pub fn compile_enum_variant(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    enum_ty: &MirTy,
    variant: &str,
    payload: &[Value],
) -> Result<CrValue, CodegenError> {
    let ptr_ty = common::ptr_type(ctx.target);
    let concrete_ty =
        substitute_type_with_self(enum_ty, &state.subst, state.self_type.as_ref(), ctx.module);
    let layout = ctx.layouts.layout_of(&concrete_ty);

    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        layout.size as u32,
        common::align_to_shift(layout.align),
    ));
    let addr = builder.ins().stack_addr(ptr_ty, slot, Offset32::new(0));
    common::zero_memory(builder, addr, layout.size, ptr_ty);

    if let MirTy::Named { entity, type_args } = &concrete_ty {
        match ctx.layouts.resolve_named(*entity) {
            NamedKind::Enum(enum_id) => {
                let type_args = common::substitute_type_args(
                    type_args,
                    &state.subst,
                    state.self_type.as_ref(),
                    ctx.module,
                );

                let enum_def = &ctx.module.enums[enum_id.index()];
                let case = enum_def.case_by_name(variant).ok_or_else(|| {
                    CodegenError::Unsupported(format!("unknown variant: {variant}"))
                })?;
                let discriminant = case.discriminant;
                let payload_struct_id = case.payload_struct;

                // Store discriminant at offset 0
                let discr_val = builder.ins().iconst(ir::types::I32, discriminant as i64);
                builder
                    .ins()
                    .store(MemFlags::new(), discr_val, addr, Offset32::new(0));

                // Store payload fields at payload offset
                if !payload.is_empty() {
                    let payload_offset =
                        get_enum_payload_offset(ctx.module, &mut ctx.layouts, enum_id, &type_args);
                    let payload_ptr = builder.ins().iadd_imm(addr, payload_offset as i64);

                    let payload_struct = &ctx.module.structs[payload_struct_id.index()];
                    let payload_sl = ctx.layouts.struct_layout(payload_struct_id, &type_args);

                    // Build substitution from the enum's type params to the concrete
                    // type args. The payload struct's field types reference the enum's
                    // type params (e.g., T in Result[T, E]), so we need this mapping
                    // to resolve them — state.subst only has the function's type params.
                    let enum_subst: HashMap<kestrel_hecs::Entity, MirTy> = enum_def
                        .type_params
                        .iter()
                        .zip(type_args.iter())
                        .map(|(tp, arg)| (tp.entity, arg.clone()))
                        .collect();

                    for (i, value) in payload.iter().enumerate() {
                        if i < payload_sl.field_offsets.len() {
                            let field_offset = payload_sl.field_offsets[i];
                            let val = rvalue::compile_value(ctx, state, builder, value)?;
                            let field_ptr =
                                builder.ins().iadd_imm(payload_ptr, field_offset as i64);

                            let field_ty = &payload_struct.fields[i].ty;
                            let concrete_field = substitute_type(field_ty, &enum_subst, ctx.module);

                            if is_aggregate(&concrete_field, &mut ctx.layouts) {
                                common::copy_aggregate(
                                    builder,
                                    &mut ctx.layouts,
                                    &concrete_field,
                                    field_ptr,
                                    val,
                                );
                            } else {
                                builder.ins().store(
                                    MemFlags::new(),
                                    val,
                                    field_ptr,
                                    Offset32::new(0),
                                );
                            }
                        }
                    }
                }
            },
            _ => {
                return Err(CodegenError::Unsupported(
                    "enum variant on non-enum type".into(),
                ));
            },
        }
    }

    Ok(addr)
}

/// Compile array literal. This creates a stack-allocated array.
pub fn compile_array_literal(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    element_ty: &MirTy,
    values: &[Value],
) -> Result<CrValue, CodegenError> {
    let ptr_ty = common::ptr_type(ctx.target);
    let concrete_elem = substitute_type_with_self(
        element_ty,
        &state.subst,
        state.self_type.as_ref(),
        ctx.module,
    );
    let elem_layout = ctx.layouts.layout_of(&concrete_elem);
    let total_size = elem_layout.size * values.len() as u64;

    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        total_size as u32,
        common::align_to_shift(elem_layout.align),
    ));
    let addr = builder.ins().stack_addr(ptr_ty, slot, Offset32::new(0));

    for (i, value) in values.iter().enumerate() {
        let offset = elem_layout.size * i as u64;
        let val = rvalue::compile_value(ctx, state, builder, value)?;
        let elem_ptr = builder.ins().iadd_imm(addr, offset as i64);

        if is_aggregate(&concrete_elem, &mut ctx.layouts) {
            common::copy_aggregate(builder, &mut ctx.layouts, &concrete_elem, elem_ptr, val);
        } else {
            builder
                .ins()
                .store(MemFlags::new(), val, elem_ptr, Offset32::new(0));
        }
    }

    Ok(addr)
}

// DictLiteral was removed from MIR — dict literals are lowered to
// ArrayLiteral { element_ty: Tuple(K, V), values } at the MIR level.
