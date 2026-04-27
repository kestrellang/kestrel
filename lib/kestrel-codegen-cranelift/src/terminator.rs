//! Terminator compilation — block exit instructions.
//!
//! Fixes the lib1 Switch last-case bug: uses `jump` instead of `brif(same, same)`.

use crate::common::{self, is_aggregate};
use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::function::FunctionState;
use crate::place;
use crate::rvalue;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, TrapCode, Value as CrValue};
use cranelift_frontend::FunctionBuilder;
use kestrel_codegen::{NamedKind, substitute_type_with_self};
use kestrel_mir::{MirTy, SwitchCase, Terminator, TerminatorKind, Value};

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
        },
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
        },
        TerminatorKind::Unreachable => {
            builder.ins().trap(TrapCode::unwrap_user(2));
            Ok(())
        },
    }
}

fn compile_return(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    value: &Value,
) -> Result<(), CodegenError> {
    let ret_ty = substitute_type_with_self(
        &state.func_def.ret,
        &state.subst,
        state.self_type.as_ref(),
        ctx.module,
    );

    // Unit return — empty tuple is the canonical unit value.
    if ret_ty.is_unit() {
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
        if is_aggregate(&ret_ty, &mut ctx.layouts) {
            let loaded = builder
                .ins()
                .load(ir::types::I64, MemFlags::new(), val, Offset32::new(0));
            builder.ins().return_(&[loaded]);
        } else {
            builder.ins().return_(&[val]);
        }
    } else if is_aggregate(&ret_ty, &mut ctx.layouts) {
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
            builder
                .ins()
                .store(MemFlags::new(), val, addr, Offset32::new(0));
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
    let cond_raw = rvalue::compile_value(ctx, state, builder, condition)?;

    // Bool is Named (aggregate) — a Place read returns a pointer; a bool
    // immediate arrives as a scalar I8. Discriminate by the cranelift value
    // type: I8 means scalar, anything else is the aggregate pointer and we
    // load the byte at offset 0.
    //
    // This width-equality check is safe *only* because I8 can never equal
    // any supported target's pointer size. Do NOT copy this pattern for
    // wider primitive wrappers (Int64, UInt64, Float64) — their widths
    // collide with 64-bit `ptr_type` and the wrong branch would be taken.
    // For switch discriminants and anywhere else that needs a scalar out
    // of a possibly-wrapped primitive Place, use
    // `place::compile_place_read_scalar` instead.
    let cond_val = if builder.func.dfg.value_type(cond_raw) == ir::types::I8 {
        cond_raw
    } else {
        builder
            .ins()
            .load(ir::types::I8, MemFlags::new(), cond_raw, Offset32::new(0))
    };

    // Convert i8 bool to branch condition
    let cmp = builder.ins().icmp_imm(IntCC::NotEqual, cond_val, 0);

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
    cases: &[(SwitchCase, kestrel_mir::BlockId)],
) -> Result<(), CodegenError> {
    // Fast path: single case → unconditional jump.
    if cases.len() == 1 {
        let (_, target_block) = &cases[0];
        let target_cl = state.block_map[target_block];
        builder.ins().jump(target_cl, &[]);
        return Ok(());
    }

    // Probe the discriminant's type to decide the scalar width we need.
    //   - Enum: always I32 (the discriminant tag at offset 0).
    //   - Everything else: the primitive width of the type (I8 for Bool, I64
    //     for Int64, …). `compile_place_read_scalar` centralizes the
    //     aggregate-vs-scalar load — do NOT reinvent that decision here.
    let probe_ty = common::get_place_type(
        ctx.module,
        state.body,
        discriminant,
        &state.subst,
        state.self_type.as_ref(),
        &ctx.layouts,
    )?;
    let enum_id = match &probe_ty {
        MirTy::Named { entity, .. } => match ctx.layouts.resolve_named(*entity) {
            NamedKind::Enum(id) => Some(id),
            _ => None,
        },
        _ => None,
    };
    let width_ty = if enum_id.is_some() {
        ir::types::I32
    } else {
        primitive_width_ty(ctx, &probe_ty)
    };

    let (discr_val, _) =
        place::compile_place_read_scalar(ctx, state, builder, discriminant, width_ty)?;

    for (i, (case, target_block)) in cases.iter().enumerate() {
        let target_cl = state.block_map[target_block];

        // Wildcard case or exhaustive last arm: unconditional jump.
        if case.is_wildcard() || i == cases.len() - 1 {
            builder.ins().jump(target_cl, &[]);
            return Ok(());
        }

        let cmp = match case {
            SwitchCase::Wildcard => unreachable!("handled above"),
            SwitchCase::Variant(name) => {
                // case_by_name keys on short names; fully-qualified names
                // (e.g. "std.core.Ordering.Less") get trimmed here.
                let expected = if let Some(eid) = enum_id {
                    let enum_def = &ctx.module.enums[eid.index()];
                    enum_def
                        .case_by_name(common::short_name(name))
                        .map(|c| c.discriminant as i64)
                        .unwrap_or(i as i64)
                } else {
                    i as i64
                };
                builder.ins().icmp_imm(IntCC::Equal, discr_val, expected)
            },
            SwitchCase::Bool(b) => builder.ins().icmp_imm(IntCC::Equal, discr_val, *b as i64),
            SwitchCase::IntLiteral(v) => builder.ins().icmp_imm(IntCC::Equal, discr_val, *v),
            SwitchCase::IntRange { start, end } => {
                range_test(builder, discr_val, *start, *end, /*signed*/ true)
            },
            SwitchCase::CharLiteral(c) => {
                builder.ins().icmp_imm(IntCC::Equal, discr_val, *c as i64)
            },
            SwitchCase::CharRange { start, end } => {
                range_test(
                    builder,
                    discr_val,
                    start.map(|s| s as i64),
                    end.map(|e| e as i64),
                    /*signed*/ false,
                )
            },
            SwitchCase::StringLiteral(_) => {
                // Not yet implemented — fall through to the next case.
                builder.ins().iconst(ir::types::I8, 0)
            },
        };
        let next_block = builder.create_block();
        builder.ins().brif(cmp, target_cl, &[], next_block, &[]);
        builder.switch_to_block(next_block);
        builder.seal_block(next_block);
    }

    // Fallthrough (shouldn't reach here for exhaustive matches)
    builder.ins().trap(TrapCode::unwrap_user(4));
    Ok(())
}

/// Pick the cranelift integer type that matches the scrutinee's layout size.
/// Works for `lang.iN` primitives and their stdlib wrappers (Bool, Char, Int64, …),
/// which are all single-field structs whose byte layout matches the primitive.
fn primitive_width_ty(ctx: &mut CodegenContext, ty: &MirTy) -> ir::Type {
    match ty {
        MirTy::Bool | MirTy::I8 => ir::types::I8,
        MirTy::I16 => ir::types::I16,
        MirTy::I32 | MirTy::F32 => ir::types::I32,
        MirTy::I64 | MirTy::F64 => ir::types::I64,
        _ => match ctx.layouts.layout_of(ty).size {
            1 => ir::types::I8,
            2 => ir::types::I16,
            4 => ir::types::I32,
            _ => ir::types::I64,
        },
    }
}

/// Build a boolean condition for `start <= val <= end`. Open bounds act as `true`.
fn range_test(
    builder: &mut FunctionBuilder,
    val: CrValue,
    start: Option<i64>,
    end: Option<i64>,
    signed: bool,
) -> CrValue {
    let (gte, lte) = if signed {
        (
            IntCC::SignedGreaterThanOrEqual,
            IntCC::SignedLessThanOrEqual,
        )
    } else {
        (
            IntCC::UnsignedGreaterThanOrEqual,
            IntCC::UnsignedLessThanOrEqual,
        )
    };
    let low_ok = match start {
        Some(s) => builder.ins().icmp_imm(gte, val, s),
        None => builder.ins().iconst(ir::types::I8, 1),
    };
    let high_ok = match end {
        Some(e) => builder.ins().icmp_imm(lte, val, e),
        None => builder.ins().iconst(ir::types::I8, 1),
    };
    builder.ins().band(low_ok, high_ok)
}
