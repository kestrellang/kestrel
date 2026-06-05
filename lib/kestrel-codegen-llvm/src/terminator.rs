//! Terminator lowering. MIR block args become phi incomings (added from the
//! current insert block); `Switch` is a comparison chain (not a jump table);
//! `Panic`/`Unreachable` lower to `llvm.trap` + `unreachable`.

use inkwell::IntPredicate;
use inkwell::builder::Builder;
use inkwell::intrinsics::Intrinsic;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValue, BasicValueEnum, IntValue};

use kestrel_mir::terminator::{SwitchCase, Terminator, TerminatorKind};
use kestrel_mir::{BlockId, MirTy, ValueId};

use crate::abi::{self, ReturnMode};
use crate::error::CodegenError;
use crate::func::FuncCompiler;
use crate::inst::find_mono_enum;
use crate::mem;
use crate::ty::{ScalarTy, int_bits_to_scalar};

/// Emit `llvm.trap` followed by `unreachable` (a real trapping terminator).
pub fn emit_trap<'ctx>(fc: &FuncCompiler<'_, 'ctx>, builder: &Builder<'ctx>) {
    if let Some(intr) = Intrinsic::find("llvm.trap") {
        if let Some(f) = intr.get_declaration(&fc.ctx.llmod, &[]) {
            let _ = builder.build_call(f, &[], "");
        }
    }
    let _ = builder.build_unreachable();
}

/// Coerce a block-arg value to the phi's expected type. Integers are zero-
/// extended/truncated to the expected width; the `int<->ptr` branches are
/// defensive repairs (should be unreachable if `value_scalar` is consistent —
/// they keep one classification slip from sinking the whole function to a trap
/// stub via a phi-type verify failure).
fn coerce<'ctx>(
    builder: &Builder<'ctx>,
    expected: BasicTypeEnum<'ctx>,
    val: BasicValueEnum<'ctx>,
) -> BasicValueEnum<'ctx> {
    if val.get_type() == expected {
        return val;
    }
    if expected.is_int_type() && val.is_int_value() {
        let e = expected.into_int_type();
        let v = val.into_int_value();
        let ew = e.get_bit_width();
        let vw = v.get_type().get_bit_width();
        if vw < ew {
            return builder.build_int_z_extend(v, e, "ext").unwrap().into();
        }
        if vw > ew {
            return builder.build_int_truncate(v, e, "tr").unwrap().into();
        }
    }
    if expected.is_pointer_type() && val.is_int_value() {
        return builder
            .build_int_to_ptr(val.into_int_value(), expected.into_pointer_type(), "i2p")
            .unwrap()
            .into();
    }
    if expected.is_int_type() && val.is_pointer_value() {
        return builder
            .build_ptr_to_int(val.into_pointer_value(), expected.into_int_type(), "p2i")
            .unwrap()
            .into();
    }
    val
}

/// Add phi incomings to `target`'s block params from the current insert block.
fn add_block_args<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    target: usize,
    args: &[ValueId],
) {
    let pred = builder.get_insert_block().unwrap();
    for (i, &arg) in args.iter().enumerate() {
        let raw = fc.resolve_scalar(builder, arg);
        let phi = fc.block_phis[target][i];
        let expected = phi.as_basic_value().get_type();
        let coerced = coerce(builder, expected, raw);
        phi.add_incoming(&[(&coerced as &dyn BasicValue, pred)]);
    }
}

pub fn compile_terminator<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    term: &Terminator,
) -> Result<(), CodegenError> {
    match &term.kind {
        TerminatorKind::Return(value_id) => compile_return(fc, builder, *value_id),

        TerminatorKind::Jump { target, args } => {
            add_block_args(fc, builder, target.index(), args);
            builder
                .build_unconditional_branch(fc.block_map[target.index()])
                .unwrap();
            Ok(())
        },

        TerminatorKind::Branch {
            condition,
            then_block,
            then_args,
            else_block,
            else_args,
        } => compile_branch(
            fc,
            builder,
            *condition,
            *then_block,
            then_args,
            *else_block,
            else_args,
        ),

        TerminatorKind::Switch {
            discriminant,
            cases,
        } => compile_switch(fc, builder, *discriminant, cases),

        TerminatorKind::Panic(_msg) => {
            emit_trap(fc, builder);
            Ok(())
        },
        TerminatorKind::Unreachable => {
            emit_trap(fc, builder);
            Ok(())
        },
    }
}

fn compile_return<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    value_id: ValueId,
) -> Result<(), CodegenError> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;
    let ret_repr = fc
        .ctx
        .tc
        .repr(fc.func.ret, &fc.ctx.module.ty_arena, fc.ctx.module);
    let ret_mode = abi::return_mode(ret_repr);

    match ret_mode {
        ReturnMode::Direct(scalar) => {
            let val = fc.resolve_scalar(builder, value_id);
            // Coerce to the declared scalar: handles a dead/unreachable block
            // whose Never/ZST return placeholder (`ptr null`) doesn't match the
            // function's scalar return type. The `@main` entry point is an
            // ordinary `i64`-returning function (the MIR-synthesized wrapper),
            // so it needs no special-casing here.
            let final_val = coerce(builder, scalar.llvm(cx), val);
            builder.build_return(Some(&final_val)).unwrap();
        },
        ReturnMode::Sret => {
            let sret_ptr = fc
                .sret_ptr
                .expect("sret_ptr must be set for Sret return mode");
            let val = fc.get_value(value_id).into_pointer_value();
            mem::copy_aggregate(cx, builder, ptr_size, ret_repr.size(), sret_ptr, val);
            builder.build_return(None).unwrap();
        },
        ReturnMode::Void => {
            builder.build_return(None).unwrap();
        },
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn compile_branch<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    condition: ValueId,
    then_block: BlockId,
    then_args: &[ValueId],
    else_block: BlockId,
    else_args: &[ValueId],
) -> Result<(), CodegenError> {
    // Degenerate both-edges-to-same-block: emit an unconditional branch and add
    // phi args once (a duplicate phi entry for one predecessor is invalid).
    if then_block == else_block {
        add_block_args(fc, builder, then_block.index(), then_args);
        builder
            .build_unconditional_branch(fc.block_map[then_block.index()])
            .unwrap();
        return Ok(());
    }

    let cond = fc.resolve_scalar(builder, condition).into_int_value();
    let zero = cond.get_type().const_zero();
    let cmp = builder
        .build_int_compare(IntPredicate::NE, cond, zero, "cond")
        .unwrap();

    add_block_args(fc, builder, then_block.index(), then_args);
    add_block_args(fc, builder, else_block.index(), else_args);

    builder
        .build_conditional_branch(
            cmp,
            fc.block_map[then_block.index()],
            fc.block_map[else_block.index()],
        )
        .unwrap();

    Ok(())
}

fn compile_switch<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    discriminant: ValueId,
    cases: &[kestrel_mir::terminator::SwitchArm],
) -> Result<(), CodegenError> {
    let cx = fc.ctx.cx;
    let raw_disc = fc.resolve_scalar(builder, discriminant).into_int_value();

    // Determine discriminant width from the value's type.
    let disc_ty = fc.body.values[discriminant.index()].ty;
    let disc_scalar = match fc.ctx.module.ty_arena.get(disc_ty) {
        MirTy::Named { entity, type_args } => {
            let entity = *entity;
            let type_args = type_args.clone();
            match find_mono_enum(&entity, &type_args, fc.ctx.module) {
                Some(e) => int_bits_to_scalar(e.discriminant_width),
                None => ScalarTy::I64,
            }
        },
        MirTy::Bool | MirTy::I8 => ScalarTy::I8,
        MirTy::I16 => ScalarTy::I16,
        MirTy::I32 => ScalarTy::I32,
        MirTy::I64 => ScalarTy::I64,
        _ => ScalarTy::I64,
    };
    let want_width = disc_scalar.bytes() as u32 * 8;

    // Narrow an over-wide discriminant (e.g. the enum carried as a scalar).
    let actual_width = raw_disc.get_type().get_bit_width();
    let disc_val = if actual_width > want_width {
        builder
            .build_int_truncate(raw_disc, disc_scalar.llvm(cx).into_int_type(), "disc")
            .unwrap()
    } else {
        raw_disc
    };
    let cmp_ty = disc_val.get_type();

    let wildcard = cases
        .iter()
        .find(|arm| matches!(arm.pattern, SwitchCase::Wildcard));
    let concrete: Vec<_> = cases
        .iter()
        .filter(|arm| !matches!(arm.pattern, SwitchCase::Wildcard))
        .collect();

    if concrete.is_empty() {
        if let Some(w) = wildcard {
            add_block_args(fc, builder, w.target.index(), &w.args);
            builder
                .build_unconditional_branch(fc.block_map[w.target.index()])
                .unwrap();
        } else {
            emit_trap(fc, builder);
        }
        return Ok(());
    }

    let n = concrete.len();
    for (i, arm) in concrete.iter().enumerate() {
        let is_last = i == n - 1;
        let cmp: IntValue = match &arm.pattern {
            SwitchCase::Variant(idx) => builder
                .build_int_compare(
                    IntPredicate::EQ,
                    disc_val,
                    cmp_ty.const_int(idx.index() as u64, false),
                    "case",
                )
                .unwrap(),
            SwitchCase::Bool(b) => builder
                .build_int_compare(
                    IntPredicate::EQ,
                    disc_val,
                    cmp_ty.const_int(*b as u64, false),
                    "case",
                )
                .unwrap(),
            SwitchCase::IntLiteral(v) => builder
                .build_int_compare(
                    IntPredicate::EQ,
                    disc_val,
                    cmp_ty.const_int(*v as u64, true),
                    "case",
                )
                .unwrap(),
            SwitchCase::CharLiteral(c) => builder
                .build_int_compare(
                    IntPredicate::EQ,
                    disc_val,
                    cmp_ty.const_int(*c as u64, false),
                    "case",
                )
                .unwrap(),
            SwitchCase::IntRange { start, end } => {
                let ge = builder
                    .build_int_compare(
                        IntPredicate::SGE,
                        disc_val,
                        cmp_ty.const_int(*start as u64, true),
                        "ge",
                    )
                    .unwrap();
                let le = builder
                    .build_int_compare(
                        IntPredicate::SLE,
                        disc_val,
                        cmp_ty.const_int(*end as u64, true),
                        "le",
                    )
                    .unwrap();
                builder.build_and(ge, le, "range").unwrap()
            },
            SwitchCase::CharRange { start, end } => {
                let ge = builder
                    .build_int_compare(
                        IntPredicate::UGE,
                        disc_val,
                        cmp_ty.const_int(*start as u64, false),
                        "ge",
                    )
                    .unwrap();
                let le = builder
                    .build_int_compare(
                        IntPredicate::ULE,
                        disc_val,
                        cmp_ty.const_int(*end as u64, false),
                        "le",
                    )
                    .unwrap();
                builder.build_and(ge, le, "range").unwrap()
            },
            SwitchCase::Wildcard => unreachable!(),
        };

        let target = arm.target.index();
        if is_last {
            // Fall through to the wildcard. With no wildcard (or one targeting the
            // same block), the final arm of an exhaustive match is always taken,
            // so the branch is unconditional — and crucially we must NOT add a phi
            // entry twice for the same predecessor block (LLVM rejects duplicate
            // entries with different SSA values).
            match wildcard {
                Some(w) if w.target.index() != target => {
                    add_block_args(fc, builder, target, &arm.args);
                    add_block_args(fc, builder, w.target.index(), &w.args);
                    builder
                        .build_conditional_branch(
                            cmp,
                            fc.block_map[target],
                            fc.block_map[w.target.index()],
                        )
                        .unwrap();
                },
                _ => {
                    let _ = cmp; // exhaustive: last arm unconditionally taken
                    add_block_args(fc, builder, target, &arm.args);
                    builder
                        .build_unconditional_branch(fc.block_map[target])
                        .unwrap();
                },
            }
        } else {
            let next = cx.append_basic_block(fc.fn_value, "switch_next");
            add_block_args(fc, builder, target, &arm.args);
            builder
                .build_conditional_branch(cmp, fc.block_map[target], next)
                .unwrap();
            builder.position_at_end(next);
        }
    }

    Ok(())
}
