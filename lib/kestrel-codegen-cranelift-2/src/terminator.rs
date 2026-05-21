use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, TrapCode, Value};
use cranelift_frontend::FunctionBuilder;

use kestrel_mir_2::{MirTy, SwitchCase, Terminator, TerminatorKind};

use crate::abi::{self, ReturnMode};
use crate::error::CodegenError;
use crate::func::FuncCompiler;
use crate::mem;
use crate::place;
use crate::rvalue;
use crate::ty::TypeRepr;

pub fn compile_terminator(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    term: &Terminator,
) -> Result<(), CodegenError> {
    match &term.kind {
        TerminatorKind::Return(operand) => compile_return(fc, builder, operand),
        TerminatorKind::Jump(target) => {
            let block = fc.block_map[target.index()];
            builder.ins().jump(block, &[]);
            Ok(())
        }
        TerminatorKind::Branch {
            condition,
            then_block,
            else_block,
        } => compile_branch(fc, builder, condition, *then_block, *else_block),
        TerminatorKind::Switch {
            discriminant,
            cases,
        } => compile_switch(fc, builder, discriminant, cases),
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
    operand: &kestrel_mir_2::Operand,
) -> Result<(), CodegenError> {
    let ret_repr = fc
        .ctx
        .tc
        .repr(fc.func.ret, &fc.ctx.module.ty_arena, fc.ctx.module);
    let ret_mode = abi::return_mode(ret_repr, fc.is_main);

    match ret_mode {
        ReturnMode::Direct(_t) => {
            let val = rvalue::compile_operand(fc, builder, operand)?;

            // If main: ensure we return i64
            if fc.is_main {
                let final_val = match ret_repr {
                    TypeRepr::Scalar(st) if st == ir::types::I64 => val,
                    TypeRepr::Scalar(st) if st.bytes() < 8 => {
                        builder.ins().sextend(ir::types::I64, val)
                    }
                    TypeRepr::Aggregate { .. } => {
                        // Load first 8 bytes as i64
                        builder
                            .ins()
                            .load(ir::types::I64, MemFlags::new(), val, Offset32::new(0))
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
            let sret_ptr = fc
                .sret_ptr
                .expect("sret_ptr must be set for Sret return mode");
            let val = rvalue::compile_operand(fc, builder, operand)?;
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
    condition: &kestrel_mir_2::Operand,
    then_block: kestrel_mir_2::BlockId,
    else_block: kestrel_mir_2::BlockId,
) -> Result<(), CodegenError> {
    let cond_val = rvalue::compile_operand(fc, builder, condition)?;

    // Condition should be I8 (bool). Compare against 0.
    let cmp = builder.ins().icmp_imm(IntCC::NotEqual, cond_val, 0);

    let then_cl = fc.block_map[then_block.index()];
    let else_cl = fc.block_map[else_block.index()];
    builder.ins().brif(cmp, then_cl, &[], else_cl, &[]);

    Ok(())
}

fn compile_switch(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    discriminant: &kestrel_mir_2::Place,
    cases: &[(SwitchCase, kestrel_mir_2::BlockId)],
) -> Result<(), CodegenError> {
    // Determine discriminant type and width
    let disc_ty =
        place::place_type(discriminant, fc.body, &fc.ctx.module.ty_arena, fc.ctx.module, &fc.ctx.tc);
    let arena = &fc.ctx.module.ty_arena;

    let disc_width = match arena.get(disc_ty) {
        MirTy::Named { entity, type_args } => {
            let entity = *entity;
            let type_args = type_args.clone();
            if let Some(e) = place::find_mono_enum(&entity, &type_args, fc.ctx.module, &fc.ctx.tc) {
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

    // Load discriminant value at offset 0 of the place
    let disc_val = place::place_read_scalar(fc, builder, discriminant, disc_width)?;

    // Find wildcard (default) block
    let wildcard_block = cases.iter().find_map(|(case, block)| {
        if matches!(case, SwitchCase::Wildcard) {
            Some(fc.block_map[block.index()])
        } else {
            None
        }
    });

    // Non-wildcard cases
    let concrete_cases: Vec<_> = cases
        .iter()
        .filter(|(case, _)| !matches!(case, SwitchCase::Wildcard))
        .collect();

    if concrete_cases.is_empty() {
        if let Some(default) = wildcard_block {
            builder.ins().jump(default, &[]);
        } else {
            builder.ins().trap(TrapCode::unwrap_user(2));
        }
        return Ok(());
    }

    // Emit cascading comparisons
    for (i, (case, block_id)) in concrete_cases.iter().enumerate() {
        let target = fc.block_map[block_id.index()];

        match case {
            SwitchCase::Variant(idx) => {
                let disc_val_imm = idx.index() as i64;
                let cmp = builder.ins().icmp_imm(IntCC::Equal, disc_val, disc_val_imm);
                emit_case_branch(fc, builder, cmp, target, &concrete_cases, i, wildcard_block);
            }
            SwitchCase::Bool(b) => {
                let cmp = builder.ins().icmp_imm(IntCC::Equal, disc_val, *b as i64);
                emit_case_branch(fc, builder, cmp, target, &concrete_cases, i, wildcard_block);
            }
            SwitchCase::IntLiteral(v) => {
                let cmp = builder.ins().icmp_imm(IntCC::Equal, disc_val, *v);
                emit_case_branch(fc, builder, cmp, target, &concrete_cases, i, wildcard_block);
            }
            SwitchCase::CharLiteral(c) => {
                let cmp = builder.ins().icmp_imm(IntCC::Equal, disc_val, *c as i64);
                emit_case_branch(fc, builder, cmp, target, &concrete_cases, i, wildcard_block);
            }
            SwitchCase::IntRange { start, end } => {
                // MIR IntRange uses i64 values but discriminants are unsigned indices
                let ge = builder
                    .ins()
                    .icmp_imm(IntCC::SignedGreaterThanOrEqual, disc_val, *start);
                let le = builder
                    .ins()
                    .icmp_imm(IntCC::SignedLessThanOrEqual, disc_val, *end);
                let in_range = builder.ins().band(ge, le);
                emit_case_branch(
                    fc,
                    builder,
                    in_range,
                    target,
                    &concrete_cases,
                    i,
                    wildcard_block,
                );
            }
            SwitchCase::CharRange { start, end } => {
                let ge = builder
                    .ins()
                    .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, disc_val, *start as i64);
                let le = builder
                    .ins()
                    .icmp_imm(IntCC::UnsignedLessThanOrEqual, disc_val, *end as i64);
                let in_range = builder.ins().band(ge, le);
                emit_case_branch(
                    fc,
                    builder,
                    in_range,
                    target,
                    &concrete_cases,
                    i,
                    wildcard_block,
                );
            }
            SwitchCase::Wildcard => unreachable!(),
        }
    }

    Ok(())
}

/// Emit a conditional branch for a switch case.
/// If this is the last non-wildcard case, fall through to the default.
fn emit_case_branch(
    _fc: &FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    cmp: Value,
    target: ir::Block,
    concrete_cases: &[&(SwitchCase, kestrel_mir_2::BlockId)],
    index: usize,
    wildcard_block: Option<ir::Block>,
) {
    let is_last = index == concrete_cases.len() - 1;

    if is_last {
        let fallthrough = wildcard_block.unwrap_or(target);
        builder.ins().brif(cmp, target, &[], fallthrough, &[]);
    } else {
        // Create a continuation block for the next case check
        let next_block = builder.create_block();
        builder.ins().brif(cmp, target, &[], next_block, &[]);
        builder.switch_to_block(next_block);
        builder.seal_block(next_block);
    }
}
