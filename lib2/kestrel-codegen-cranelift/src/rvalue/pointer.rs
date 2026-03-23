//! Pointer and memory operations.

use crate::common::{self, is_aggregate_type};
use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::function::FunctionState;
use crate::types;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, StackSlotData, StackSlotKind, Value as CrValue};
use cranelift_frontend::FunctionBuilder;
use kestrel_mir::{MirTy, Op};

/// Compile pointer unary ops (PtrNull, PtrFromAddress, PtrToAddress, PtrIsNull, PtrCast, RefToPtr, PtrRead).
pub fn compile_pointer_op1(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    op: &Op,
    arg: CrValue,
) -> Result<CrValue, CodegenError> {
    let ptr_ty = common::ptr_type(ctx.target);

    match op {
        Op::PtrNull(_) => Ok(builder.ins().iconst(ptr_ty, 0)),
        Op::PtrFromAddress(_) => Ok(arg), // int → ptr, same representation
        Op::PtrToAddress => Ok(arg),       // ptr → int, same representation
        Op::PtrIsNull => {
            let zero = builder.ins().iconst(ptr_ty, 0);
            Ok(builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, arg, zero))
        }
        Op::PtrCast(_) | Op::PtrBitcast(_) => Ok(arg), // Same representation
        Op::RefToPtr => Ok(arg), // &T → p[T], same representation

        Op::PtrRead(ty) => {
            // Substitute type params with concrete types from the current instantiation
            let concrete_ty = kestrel_codegen2::substitute_type(ty, &state.subst);
            if is_aggregate_type(&concrete_ty) && !common::type_has_unresolved_params(&concrete_ty) {
                // Fully resolved aggregate: copy to a local stack slot
                let layout = ctx.layouts.layout_of(&concrete_ty);
                let size = if layout.size == 0 { 1 } else { layout.size };
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    size as u32,
                    common::align_to_shift(layout.align),
                ));
                let ptr_ty = common::ptr_type(ctx.target);
                let addr = builder.ins().stack_addr(ptr_ty, slot, Offset32::new(0));
                common::copy_aggregate(builder, &mut ctx.layouts, &concrete_ty, addr, arg);
                Ok(addr)
            } else if is_aggregate_type(&concrete_ty) {
                // Unresolved aggregate: load as pointer-sized value (fallback)
                let ptr_ty = common::ptr_type(ctx.target);
                Ok(builder.ins().load(ptr_ty, MemFlags::new(), arg, Offset32::new(0)))
            } else {
                let cl_ty = types::translate_type(&concrete_ty, ctx.target);
                Ok(builder.ins().load(cl_ty, MemFlags::new(), arg, Offset32::new(0)))
            }
        }

        _ => Err(CodegenError::Unsupported(format!("pointer op1: {op:?}"))),
    }
}

/// Compile memory-related unary ops (SizeOf, AlignOf, StackAlloc).
pub fn compile_memory_op1(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    op: &Op,
    arg: CrValue,
) -> Result<CrValue, CodegenError> {
    let ptr_ty = common::ptr_type(ctx.target);

    match op {
        Op::SizeOf(ty) => {
            let concrete_ty = kestrel_codegen2::substitute_type(ty, &state.subst);
            let layout = ctx.layouts.layout_of(&concrete_ty);


            Ok(builder.ins().iconst(ptr_ty, layout.size as i64))
        }

        Op::AlignOf(ty) => {
            let concrete_ty = kestrel_codegen2::substitute_type(ty, &state.subst);
            let layout = ctx.layouts.layout_of(&concrete_ty);
            Ok(builder.ins().iconst(ptr_ty, layout.align as i64))
        }

        Op::StackAlloc(ty) => {
            // arg is the count (must be a compile-time constant for stack allocation)
            // For now, allocate a fixed-size slot
            let layout = ctx.layouts.layout_of(ty);
            // Use a reasonable default size; dynamic count not supported for stack alloc
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                (layout.size * 64) as u32, // Reasonable max for stack arrays
                common::align_to_shift(layout.align),
            ));
            Ok(builder.ins().stack_addr(ptr_ty, slot, Offset32::new(0)))
        }

        _ => Err(CodegenError::Unsupported(format!("memory op1: {op:?}"))),
    }
}

/// Compile pointer binary ops (PtrOffset, PtrWrite).
pub fn compile_pointer_op2(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    op: &Op,
    lhs: CrValue,
    rhs: CrValue,
) -> Result<CrValue, CodegenError> {
    match op {
        Op::PtrOffset => {
            // ptr + byte_offset
            Ok(builder.ins().iadd(lhs, rhs))
        }

        Op::PtrWrite(ty) => {
            // Substitute type params to get the concrete pointee type
            let concrete_ty = kestrel_codegen2::substitute_type(ty, &state.subst);
            if is_aggregate_type(&concrete_ty) {
                if common::type_has_unresolved_params(&concrete_ty) {
                    builder.ins().store(MemFlags::new(), rhs, lhs, Offset32::new(0));
                } else {
                    common::copy_aggregate(builder, &mut ctx.layouts, &concrete_ty, lhs, rhs);
                }
            } else {
                builder.ins().store(MemFlags::new(), rhs, lhs, Offset32::new(0));
            }
            Ok(builder.ins().iconst(ir::types::I8, 0))
        }

        _ => Err(CodegenError::Unsupported(format!("pointer op2: {op:?}"))),
    }
}

/// Compile atomic binary ops (AtomicAdd, AtomicSub).
pub fn compile_atomic_op2(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    op: &Op,
    ptr: CrValue,
    delta: CrValue,
) -> Result<CrValue, CodegenError> {
    match op {
        Op::AtomicAdd => {
            Ok(builder.ins().atomic_rmw(
                ir::types::I64,
                MemFlags::new(),
                ir::AtomicRmwOp::Add,
                ptr,
                delta,
            ))
        }

        Op::AtomicSub => {
            Ok(builder.ins().atomic_rmw(
                ir::types::I64,
                MemFlags::new(),
                ir::AtomicRmwOp::Sub,
                ptr,
                delta,
            ))
        }

        _ => Err(CodegenError::Unsupported(format!("atomic op2: {op:?}"))),
    }
}
