use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::instructions::BlockArg;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, TrapCode, Value};
use cranelift_frontend::FunctionBuilder;

use kestrel_mir_3::terminator::{SwitchCase, Terminator, TerminatorKind};
use kestrel_mir_3::{MirTy, ValueId};

use crate::abi::{self, ReturnMode};
use crate::error::CodegenError;
use crate::func::FuncCompiler;
use crate::inst::find_mono_enum;
use crate::mem;
use crate::ty::TypeRepr;

fn to_block_args(vals: &[Value]) -> Vec<BlockArg> {
    vals.iter().map(|v| BlockArg::Value(*v)).collect()
}

/// Coerce values to match the target block's declared param types.
fn coerce_block_args(
    builder: &mut FunctionBuilder,
    target: ir::Block,
    vals: &[Value],
) -> Vec<Value> {
    let param_types: Vec<ir::Type> = builder.block_params(target)
        .iter()
        .map(|&p| builder.func.dfg.value_type(p))
        .collect();

    vals.iter().enumerate().map(|(i, &val)| {
        let actual = builder.func.dfg.value_type(val);
        let expected = param_types.get(i).copied().unwrap_or(actual);
        if actual == expected {
            val
        } else if actual.bytes() < expected.bytes() && actual.is_int() && expected.is_int() {
            builder.ins().uextend(expected, val)
        } else if actual.bytes() > expected.bytes() && actual.is_int() && expected.is_int() {
            builder.ins().ireduce(expected, val)
        } else {
            val
        }
    }).collect()
}

pub fn compile_terminator(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    term: &Terminator,
) -> Result<(), CodegenError> {
    match &term.kind {
        TerminatorKind::Return(value_id) => compile_return(fc, builder, *value_id),
        TerminatorKind::Jump { target, args } => {
            let block = fc.block_map[target.index()];
            let cl_args: Vec<Value> = args.iter().map(|v| fc.get_value(builder, *v)).collect();
            let coerced = coerce_block_args(builder, block, &cl_args);
            let ba = to_block_args(&coerced);
            builder.ins().jump(block, &ba);
            Ok(())
        }
        TerminatorKind::Branch {
            condition,
            then_block,
            then_args,
            else_block,
            else_args,
        } => compile_branch(fc, builder, *condition, *then_block, then_args, *else_block, else_args),
        TerminatorKind::Switch { discriminant, cases } => {
            compile_switch(fc, builder, *discriminant, cases)
        }
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
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    value_id: ValueId,
) -> Result<(), CodegenError> {
    let ret_repr = fc.ctx.tc.repr(fc.func.ret, &fc.ctx.module.ty_arena, fc.ctx.module);
    let ret_mode = abi::return_mode(ret_repr, fc.is_main);

    match ret_mode {
        ReturnMode::Direct(_t) => {
            let val = fc.get_value(builder, value_id);
            if fc.is_main {
                let final_val = match ret_repr {
                    TypeRepr::Scalar(st) if st == ir::types::I64 => val,
                    TypeRepr::Scalar(st) if st.bytes() < 8 => {
                        builder.ins().sextend(ir::types::I64, val)
                    }
                    TypeRepr::Aggregate { .. } => {
                        builder.ins().load(ir::types::I64, MemFlags::new(), val, Offset32::new(0))
                    }
                    TypeRepr::Zst => builder.ins().iconst(ir::types::I64, 0),
                    _ => val,
                };
                builder.ins().return_(&[final_val]);
            } else {
                builder.ins().return_(&[val]);
            }
        }
        ReturnMode::Sret => {
            let sret_ptr = fc.sret_ptr.expect("sret_ptr must be set for Sret return mode");
            let val = fc.get_value(builder, value_id);
            mem::copy_aggregate(builder, ret_repr.size(), sret_ptr, val);
            builder.ins().return_(&[]);
        }
        ReturnMode::Void => {
            builder.ins().return_(&[]);
        }
    }

    Ok(())
}

fn compile_branch(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    condition: ValueId,
    then_block: kestrel_mir_3::BlockId,
    then_args: &[ValueId],
    else_block: kestrel_mir_3::BlockId,
    else_args: &[ValueId],
) -> Result<(), CodegenError> {
    let cond_val = fc.get_value(builder, condition);
    let cmp = builder.ins().icmp_imm(IntCC::NotEqual, cond_val, 0);

    let then_cl = fc.block_map[then_block.index()];
    let else_cl = fc.block_map[else_block.index()];

    let then_vals: Vec<Value> = then_args.iter().map(|v| fc.get_value(builder, *v)).collect();
    let else_vals: Vec<Value> = else_args.iter().map(|v| fc.get_value(builder, *v)).collect();
    let then_coerced = coerce_block_args(builder, then_cl, &then_vals);
    let else_coerced = coerce_block_args(builder, else_cl, &else_vals);
    let then_ba = to_block_args(&then_coerced);
    let else_ba = to_block_args(&else_coerced);

    builder.ins().brif(cmp, then_cl, &then_ba, else_cl, &else_ba);

    Ok(())
}

fn compile_switch(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    discriminant: ValueId,
    cases: &[kestrel_mir_3::terminator::SwitchArm],
) -> Result<(), CodegenError> {
    let disc_val = fc.get_value(builder, discriminant);

    // Determine discriminant width from the value's type
    let disc_ty = fc.body.values[discriminant.index()].ty;
    let arena = &fc.ctx.module.ty_arena;
    let disc_width = match arena.get(disc_ty) {
        MirTy::Named { entity, type_args } => {
            let entity = *entity;
            let type_args = type_args.clone();
            if let Some(e) = find_mono_enum(&entity, &type_args, fc.ctx.module, &fc.ctx.tc) {
                crate::ty::int_bits_to_cl(e.discriminant_width)
            } else {
                ir::types::I64
            }
        }
        MirTy::Bool => ir::types::I8,
        MirTy::I8 => ir::types::I8,
        MirTy::I16 => ir::types::I16,
        MirTy::I32 => ir::types::I32,
        MirTy::I64 => ir::types::I64,
        _ => ir::types::I64,
    };

    // If the discriminant was loaded via Discriminant instruction, its Cranelift
    // value already has the right width. But if it's a wider type (e.g., the
    // value IS the enum as a scalar), we need to load the tag.
    let disc_cl_ty = builder.func.dfg.value_type(disc_val);
    let disc_val = if disc_cl_ty != disc_width {
        // The value is the enum itself (scalar repr); load the discriminant from it
        if disc_cl_ty.bytes() > disc_width.bytes() {
            builder.ins().ireduce(disc_width, disc_val)
        } else {
            disc_val
        }
    } else {
        disc_val
    };

    // Find wildcard
    let wildcard = cases.iter().find(|arm| matches!(arm.pattern, SwitchCase::Wildcard));
    let wildcard_info = wildcard.map(|arm| {
        let cl_block = fc.block_map[arm.target.index()];
        let cl_args: Vec<Value> = arm.args.iter().map(|v| fc.get_value(builder, *v)).collect();
        let coerced = coerce_block_args(builder, cl_block, &cl_args);
        (cl_block, coerced)
    });

    let concrete_cases: Vec<_> = cases
        .iter()
        .filter(|arm| !matches!(arm.pattern, SwitchCase::Wildcard))
        .collect();

    if concrete_cases.is_empty() {
        if let Some((default_block, default_args)) = &wildcard_info {
            let ba = to_block_args(default_args);
            builder.ins().jump(*default_block, &ba);
        } else {
            builder.ins().trap(TrapCode::unwrap_user(2));
        }
        return Ok(());
    }

    for (i, arm) in concrete_cases.iter().enumerate() {
        let target = fc.block_map[arm.target.index()];
        let raw_args: Vec<Value> = arm.args.iter().map(|v| fc.get_value(builder, *v)).collect();
        let target_args = coerce_block_args(builder, target, &raw_args);
        let is_last = i == concrete_cases.len() - 1;

        match &arm.pattern {
            SwitchCase::Variant(idx) => {
                let cmp = builder.ins().icmp_imm(IntCC::Equal, disc_val, idx.index() as i64);
                emit_case_branch(builder, cmp, target, &target_args, is_last, &wildcard_info);
            }
            SwitchCase::Bool(b) => {
                let cmp = builder.ins().icmp_imm(IntCC::Equal, disc_val, *b as i64);
                emit_case_branch(builder, cmp, target, &target_args, is_last, &wildcard_info);
            }
            SwitchCase::IntLiteral(v) => {
                let cmp = builder.ins().icmp_imm(IntCC::Equal, disc_val, *v);
                emit_case_branch(builder, cmp, target, &target_args, is_last, &wildcard_info);
            }
            SwitchCase::CharLiteral(c) => {
                let cmp = builder.ins().icmp_imm(IntCC::Equal, disc_val, *c as i64);
                emit_case_branch(builder, cmp, target, &target_args, is_last, &wildcard_info);
            }
            SwitchCase::IntRange { start, end } => {
                let ge = builder.ins().icmp_imm(IntCC::SignedGreaterThanOrEqual, disc_val, *start);
                let le = builder.ins().icmp_imm(IntCC::SignedLessThanOrEqual, disc_val, *end);
                let in_range = builder.ins().band(ge, le);
                emit_case_branch(builder, in_range, target, &target_args, is_last, &wildcard_info);
            }
            SwitchCase::CharRange { start, end } => {
                let ge = builder.ins().icmp_imm(IntCC::UnsignedGreaterThanOrEqual, disc_val, *start as i64);
                let le = builder.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, disc_val, *end as i64);
                let in_range = builder.ins().band(ge, le);
                emit_case_branch(builder, in_range, target, &target_args, is_last, &wildcard_info);
            }
            SwitchCase::Wildcard => unreachable!(),
        }
    }

    Ok(())
}

fn emit_case_branch(
    builder: &mut FunctionBuilder,
    cmp: Value,
    target: ir::Block,
    target_args: &[Value],
    is_last: bool,
    wildcard_info: &Option<(ir::Block, Vec<Value>)>,
) {
    let target_ba = to_block_args(target_args);
    if is_last {
        let (fallthrough, fallthrough_ba) = if let Some((block, args)) = wildcard_info {
            (*block, to_block_args(args))
        } else {
            (target, target_ba.clone())
        };
        builder.ins().brif(cmp, target, &target_ba, fallthrough, &fallthrough_ba);
    } else {
        let next_block = builder.create_block();
        builder.ins().brif(cmp, target, &target_ba, next_block, &[]);
        builder.switch_to_block(next_block);
        builder.seal_block(next_block);
    }
}
