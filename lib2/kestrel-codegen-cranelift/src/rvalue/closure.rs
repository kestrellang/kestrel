//! Closure construction — ApplyPartial compilation.
//!
//! Creates a thick callable by allocating an environment struct,
//! storing captures, and pairing it with a thunk function pointer.

use crate::common::{self, is_aggregate_type};
use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::function::FunctionState;
use crate::rvalue;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, StackSlotData, StackSlotKind, Value as CrValue};
use cranelift_frontend::FunctionBuilder;
use kestrel_hecs::Entity;
use kestrel_mir::Value;

/// Compile `apply partial func(captures...)` → thick callable (func_ptr, env_ptr).
pub fn compile_apply_partial(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    func: &Entity,
    captures: &[Value],
) -> Result<CrValue, CodegenError> {
    let ptr_ty = common::ptr_type(ctx.target);
    let ptr_size = ctx.target.pointer_size();

    // Allocate the thick callable struct: (func_ptr, env_ptr)
    let thick_size = ptr_size * 2;
    let thick_slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        thick_size as u32,
        common::align_to_shift(ptr_size),
    ));
    let thick_addr = builder.ins().stack_addr(ptr_ty, thick_slot, Offset32::new(0));

    // TODO: Resolve the thunk function address and store it
    // For now, store a null func_ptr (needs thunk resolution)
    let null = builder.ins().iconst(ptr_ty, 0);
    builder.ins().store(MemFlags::new(), null, thick_addr, Offset32::new(0));

    // Allocate environment struct with captures
    if !captures.is_empty() {
        let env_size = ptr_size * captures.len() as u64;
        let env_slot = builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            env_size as u32,
            common::align_to_shift(ptr_size),
        ));
        let env_addr = builder.ins().stack_addr(ptr_ty, env_slot, Offset32::new(0));

        // Store each capture
        for (i, capture) in captures.iter().enumerate() {
            let val = rvalue::compile_value(ctx, state, builder, capture)?;
            let offset = ptr_size * i as u64;
            builder.ins().store(
                MemFlags::new(),
                val,
                env_addr,
                Offset32::new(offset as i32),
            );
        }

        // Store env_ptr at offset ptr_size in the thick struct
        builder.ins().store(
            MemFlags::new(),
            env_addr,
            thick_addr,
            Offset32::new(ptr_size as i32),
        );
    } else {
        // No captures: store null env
        builder.ins().store(
            MemFlags::new(),
            null,
            thick_addr,
            Offset32::new(ptr_size as i32),
        );
    }

    Ok(thick_addr)
}
