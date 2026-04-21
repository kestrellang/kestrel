//! Pointer and memory operations.

use crate::common::{self, is_aggregate};
use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::function::FunctionState;
use crate::types;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{
    self, InstBuilder, MemFlags, StackSlotData, StackSlotKind, Value as CrValue,
};
use cranelift_frontend::FunctionBuilder;
use kestrel_mir::{Immediate, ImmediateKind, MirTy, Op, Value};

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
        Op::PtrToAddress => Ok(arg),      // ptr → int, same representation
        Op::PtrIsNull => {
            let zero = builder.ins().iconst(ptr_ty, 0);
            Ok(builder
                .ins()
                .icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, arg, zero))
        },
        Op::PtrCast(_) | Op::PtrBitcast(_) => Ok(arg), // Same representation
        Op::RefToPtr => Ok(arg),                       // &T → p[T], same representation

        Op::PtrRead(ty) => {
            // Substitute type params with concrete types from the current instantiation
            let concrete_ty = kestrel_codegen2::substitute_type_with_self(ty, &state.subst, state.self_type.as_ref());
            if is_aggregate(&concrete_ty, &mut ctx.layouts)
                && !common::type_has_unresolved_params(&concrete_ty)
            {
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
            } else if is_aggregate(&concrete_ty, &mut ctx.layouts) {
                // Unresolved aggregate: load as pointer-sized value (fallback)
                let ptr_ty = common::ptr_type(ctx.target);
                Ok(builder
                    .ins()
                    .load(ptr_ty, MemFlags::new(), arg, Offset32::new(0)))
            } else {
                let cl_ty = types::translate_type(&concrete_ty, ctx.target);
                Ok(builder
                    .ins()
                    .load(cl_ty, MemFlags::new(), arg, Offset32::new(0)))
            }
        },

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
            let concrete_ty = kestrel_codegen2::substitute_type_with_self(ty, &state.subst, state.self_type.as_ref());
            if matches!(concrete_ty, kestrel_mir::MirTy::Error) {
                return Err(CodegenError::Unsupported(format!(
                    "SizeOf on unresolved type in {} (raw: {:?})",
                    state.func_def.name, ty
                )));
            }
            let layout = ctx.layouts.layout_of(&concrete_ty);
            Ok(builder.ins().iconst(ptr_ty, layout.size as i64))
        },

        Op::AlignOf(ty) => {
            let concrete_ty = kestrel_codegen2::substitute_type_with_self(ty, &state.subst, state.self_type.as_ref());
            if matches!(concrete_ty, kestrel_mir::MirTy::Error) {
                return Err(CodegenError::Unsupported(format!(
                    "AlignOf on unresolved type in {} (raw: {:?})",
                    state.func_def.name, ty
                )));
            }
            let layout = ctx.layouts.layout_of(&concrete_ty);
            Ok(builder.ins().iconst(ptr_ty, layout.align as i64))
        },

        _ => Err(CodegenError::Unsupported(format!("memory op1: {op:?}"))),
    }
}

/// Maximum element count accepted for `Op::StackAlloc`. Stack slots are created
/// with a fixed size at compile time, so the count must fit in this bound.
const STACK_ALLOC_MAX_COUNT: u64 = 4096;

/// Compile `Op::StackAlloc` — allocate `count * sizeof(element_ty)` bytes on
/// the stack and return a pointer to the slot.
///
/// Requires `count` to be a compile-time integer literal. Runtime counts are
/// not supported: stack-slot sizes are baked into the frame layout when the
/// function is compiled, so a variable count has no well-defined meaning here.
/// Callers that need a runtime-sized buffer must use heap allocation instead.
pub fn compile_stack_alloc(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    element_ty: &MirTy,
    arg: &Value,
) -> Result<CrValue, CodegenError> {
    let concrete_ty = kestrel_codegen2::substitute_type_with_self(element_ty, &state.subst, state.self_type.as_ref());
    let layout = ctx.layouts.layout_of(&concrete_ty);

    let count = match arg {
        Value::Immediate(Immediate {
            kind: ImmediateKind::IntLiteral { value, .. },
        }) => {
            if *value < 0 {
                return Err(CodegenError::Unsupported(format!(
                    "StackAlloc count must be non-negative, got {value}"
                )));
            }
            *value as u64
        },
        _ => {
            return Err(CodegenError::Unsupported(
                "StackAlloc requires a compile-time integer count".into(),
            ));
        },
    };

    if count > STACK_ALLOC_MAX_COUNT {
        return Err(CodegenError::Unsupported(format!(
            "StackAlloc count {count} exceeds maximum {STACK_ALLOC_MAX_COUNT}"
        )));
    }

    let total_size = layout.size.saturating_mul(count).max(1);
    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        total_size as u32,
        common::align_to_shift(layout.align),
    ));
    let ptr_ty = common::ptr_type(ctx.target);
    Ok(builder.ins().stack_addr(ptr_ty, slot, Offset32::new(0)))
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
        },

        Op::PtrWrite(ty) => {
            // Substitute type params to get the concrete pointee type
            let concrete_ty = kestrel_codegen2::substitute_type_with_self(ty, &state.subst, state.self_type.as_ref());
            if is_aggregate(&concrete_ty, &mut ctx.layouts) {
                if common::type_has_unresolved_params(&concrete_ty) {
                    builder
                        .ins()
                        .store(MemFlags::new(), rhs, lhs, Offset32::new(0));
                } else {
                    common::copy_aggregate(builder, &mut ctx.layouts, &concrete_ty, lhs, rhs);
                }
            } else {
                builder
                    .ins()
                    .store(MemFlags::new(), rhs, lhs, Offset32::new(0));
            }
            Ok(builder.ins().iconst(ir::types::I8, 0))
        },

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
        Op::AtomicAdd => Ok(builder.ins().atomic_rmw(
            ir::types::I64,
            MemFlags::new(),
            ir::AtomicRmwOp::Add,
            ptr,
            delta,
        )),

        Op::AtomicSub => Ok(builder.ins().atomic_rmw(
            ir::types::I64,
            MemFlags::new(),
            ir::AtomicRmwOp::Sub,
            ptr,
            delta,
        )),

        _ => Err(CodegenError::Unsupported(format!("atomic op2: {op:?}"))),
    }
}
