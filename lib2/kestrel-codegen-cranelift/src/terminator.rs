//! Terminator compilation — block exit instructions.
//!
//! Fixes the lib1 Switch last-case bug: uses `jump` instead of `brif(same, same)`.

use crate::common::{self, is_aggregate_type};
use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::function::FunctionState;
use crate::place;
use crate::rvalue;
use crate::types;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, TrapCode, Value as CrValue};
use cranelift_frontend::FunctionBuilder;
use kestrel_codegen2::{substitute_type, NamedKind};
use kestrel_mir::{MirTy, Terminator, TerminatorKind, Value};

/// Compile a block terminator.
pub fn compile_terminator(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    terminator: &Terminator,
) -> Result<(), CodegenError> {
    match &terminator.kind {
        TerminatorKind::Return(value) => compile_return(ctx, state, builder, value),
        TerminatorKind::Jump(target) => {
            let cl_block = state.block_map[target];
            builder.ins().jump(cl_block, &[]);
            Ok(())
        }
        TerminatorKind::Branch {
            condition,
            then_block,
            else_block,
        } => compile_branch(ctx, state, builder, condition, *then_block, *else_block),
        TerminatorKind::Switch {
            discriminant,
            cases,
        } => compile_switch(ctx, state, builder, discriminant, cases),
        TerminatorKind::Panic(_msg) => {
            builder.ins().trap(TrapCode::unwrap_user(1));
            Ok(())
        }
        TerminatorKind::Unreachable => {
            builder.ins().trap(TrapCode::unwrap_user(2));
            Ok(())
        }
    }
}

fn compile_return(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    value: &Value,
) -> Result<(), CodegenError> {
    let ret_ty = substitute_type(&state.func_def.ret, &state.subst);

    // Unit return
    if matches!(ret_ty, MirTy::Unit) {
        if state.is_main {
            let zero = builder.ins().iconst(ir::types::I64, 0);
            builder.ins().return_(&[zero]);
        } else {
            builder.ins().return_(&[]);
        }
        return Ok(());
    }

    let val = rvalue::compile_value(ctx, state, builder, value)?;

    if let Some(sret_ptr) = state.sret_ptr {
        // Aggregate return via sret pointer
        common::copy_aggregate(builder, &mut ctx.layouts, &ret_ty, sret_ptr, val);
        builder.ins().return_(&[]);
    } else if state.is_main {
        // Main returns i64 — may need to extract from wrapper struct
        if is_aggregate_type(&ret_ty) {
            let loaded = builder.ins().load(
                ir::types::I64,
                MemFlags::new(),
                val,
                Offset32::new(0),
            );
            builder.ins().return_(&[loaded]);
        } else {
            builder.ins().return_(&[val]);
        }
    } else if is_aggregate_type(&ret_ty) {
        // Non-sret aggregate return: if the value is a scalar (e.g., Bool literal
        // compiled as i8 but return type is Named{Bool} which is a pointer),
        // we need to check the value's Cranelift type vs the signature's return type.
        let val_type = builder.func.dfg.value_type(val);
        let sig_ret_type = builder.func.signature.returns.first().map(|r| r.value_type);
        if Some(val_type) != sig_ret_type && sig_ret_type.is_some() {
            // Value type mismatch — store scalar to stack, return pointer
            let ptr_ty = common::ptr_type(ctx.target);
            let layout = ctx.layouts.layout_of(&ret_ty);
            let size = if layout.size == 0 { 1 } else { layout.size };
            let slot = builder.create_sized_stack_slot(ir::StackSlotData::new(
                ir::StackSlotKind::ExplicitSlot,
                size as u32,
                common::align_to_shift(layout.align),
            ));
            let addr = builder.ins().stack_addr(ptr_ty, slot, Offset32::new(0));
            builder.ins().store(MemFlags::new(), val, addr, Offset32::new(0));
            builder.ins().return_(&[addr]);
        } else {
            builder.ins().return_(&[val]);
        }
    } else {
        builder.ins().return_(&[val]);
    }

    Ok(())
}

fn compile_branch(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    condition: &Value,
    then_block: kestrel_mir::BlockId,
    else_block: kestrel_mir::BlockId,
) -> Result<(), CodegenError> {
    let cond_val = rvalue::compile_value(ctx, state, builder, condition)?;

    // Bool is i8; convert to branch condition
    let cmp = builder
        .ins()
        .icmp_imm(IntCC::NotEqual, cond_val, 0);

    let then_cl = state.block_map[&then_block];
    let else_cl = state.block_map[&else_block];
    builder.ins().brif(cmp, then_cl, &[], else_cl, &[]);

    Ok(())
}

fn compile_switch(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    discriminant: &kestrel_mir::Place,
    cases: &[(String, kestrel_mir::BlockId)],
) -> Result<(), CodegenError> {
    // Fast path: single wildcard or single case → unconditional jump
    if cases.len() == 1 {
        let (_, target_block) = &cases[0];
        let target_cl = state.block_map[target_block];
        builder.ins().jump(target_cl, &[]);
        return Ok(());
    }

    // Resolve the discriminant type to determine how to read it
    let enum_ty = common::get_place_type(
        ctx.module,
        state.body,
        discriminant,
        &state.subst,
        &ctx.layouts,
    )?;
    let enum_id = match &enum_ty {
        MirTy::Named { entity, .. } => match ctx.layouts.resolve_named(*entity) {
            NamedKind::Enum(id) => Some(id),
            _ => None,
        },
        _ => None,
    };

    let disc_val_raw = place::compile_place_read(ctx, state, builder, discriminant)?;

    // For enums: the discriminant is at offset 0 of the enum pointer.
    // For primitives (I32, I8, etc.): the value IS the discriminant.
    let discr_val = if enum_id.is_some() {
        builder.ins().load(
            ir::types::I32,
            MemFlags::new(),
            disc_val_raw,
            Offset32::new(0),
        )
    } else {
        disc_val_raw
    };

    for (i, (case_name, target_block)) in cases.iter().enumerate() {
        let target_cl = state.block_map[target_block];

        // Wildcard case: unconditional jump
        if case_name == "_" {
            builder.ins().jump(target_cl, &[]);
            return Ok(());
        }

        // Last case: unconditional jump (exhaustive match, no need to compare)
        if i == cases.len() - 1 {
            builder.ins().jump(target_cl, &[]);
            return Ok(());
        }

        // Look up the discriminant value for this case
        let expected = if let Some(eid) = enum_id {
            let enum_def = &ctx.module.enums[eid.index()];
            enum_def
                .case_by_name(case_name)
                .map(|c| c.discriminant as i64)
                .unwrap_or(i as i64)
        } else {
            i as i64
        };

        let cmp = builder.ins().icmp_imm(IntCC::Equal, discr_val, expected);
        let next_block = builder.create_block();
        builder.ins().brif(cmp, target_cl, &[], next_block, &[]);
        builder.switch_to_block(next_block);
        builder.seal_block(next_block);
    }

    // Fallthrough (shouldn't reach here for exhaustive matches)
    builder.ins().trap(TrapCode::unwrap_user(4));
    Ok(())
}
