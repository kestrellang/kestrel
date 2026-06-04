//! Instruction lowering — the core of the backend. Faithful port of the
//! Cranelift backend's `inst.rs`. Addresses are i64 (Option A): byte-offset
//! arithmetic is `build_int_add`, and `inttoptr` happens only inside `mem`.
//!
//! Operand contract legend (same as the Cranelift backend):
//!   VALUE  — the scalar/aggregate data itself (resolve_scalar)
//!   ADDR   — a memory address to load from / store to (get_value)
//!   RAW    — forwarded as-is, custom ownership handling inline

use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, IntValue};
use inkwell::{AtomicOrdering, AtomicRMWBinOp, FloatPredicate, IntPredicate};
use inkwell::intrinsics::Intrinsic;

use kestrel_hecs::Entity;
use kestrel_mir::callee::Callee;
use kestrel_mir::inst::{CallArg, InstKind};
use kestrel_mir::mono::{MonoEnum, MonoModule, MonoStruct};
use kestrel_mir::value::Ownership;
use kestrel_mir::{
    FieldIdx, FloatMathKind, FloatPredicateKind, Layout, MirTy, Op, ParamConvention, Signedness,
    StructLayout, TyArena, TyId, ValueId, VariantIdx,
};

use crate::abi::{self, PassMode, ReturnMode};
use crate::error::CodegenError;
use crate::func::FuncCompiler;
use crate::mem;
use crate::terminator::emit_trap;
use crate::ty::{ScalarTy, TypeRepr, float_bits_to_scalar, int_bits_to_scalar};

/// Returns true if the instruction diverges (call to a !-returning function).
pub fn compile_inst<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    kind: &InstKind,
) -> Result<bool, CodegenError> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;

    match kind {
        // operand: VALUE → result: @owned value
        InstKind::MoveValue { result, operand } => {
            let val = fc.resolve_scalar(builder, *operand);
            fc.map_value(*result, val);
        },

        // operand: RAW (custom @guaranteed handling) → result: @owned value
        InstKind::CopyValue { result, operand } => {
            let val = fc.get_value(*operand);
            let ty = fc.body.values[result.index()].ty;
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            let operand_is_guaranteed =
                fc.body.values[operand.index()].ownership == Ownership::Guaranteed;
            match repr {
                TypeRepr::Aggregate { size, align } => {
                    let slot = fc.alloca(size, align);
                    mem::copy_aggregate(cx, builder, ptr_size, size, slot, val.into_int_value());
                    fc.map_value(*result, slot.into());
                },
                TypeRepr::Scalar(t) if operand_is_guaranteed => {
                    let p = mem::int_to_ptr(cx, builder, val.into_int_value());
                    let loaded = builder.build_load(t.llvm(cx), p, "cv").unwrap();
                    fc.map_value(*result, loaded);
                },
                _ => {
                    fc.map_value(*result, val);
                },
            }
        },

        InstKind::DestroyValue { .. } => {},

        // operand: RAW (custom spill for @owned scalars) → result: ADDR
        InstKind::BeginBorrow { result, operand } | InstKind::BeginMutBorrow { result, operand } => {
            let val = fc.get_value(*operand);
            let operand_ty = fc.body.values[operand.index()].ty;
            let is_guaranteed =
                fc.body.values[operand.index()].ownership == Ownership::Guaranteed;
            let repr = fc.ctx.tc.repr(operand_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            match repr {
                _ if is_guaranteed => fc.map_value(*result, val),
                TypeRepr::Aggregate { .. } | TypeRepr::Zst => fc.map_value(*result, val),
                TypeRepr::Scalar(_) => {
                    let slot = fc.alloca(repr.size(), repr.align());
                    let p = mem::int_to_ptr(cx, builder, slot);
                    builder.build_store(p, val).unwrap();
                    fc.map_value(*result, slot.into());
                },
            }
        },

        InstKind::EndBorrow { .. } | InstKind::EndMutBorrow { .. } => {},

        // address: ADDR → result: ADDR
        InstKind::BeginBorrowAddr { result, address, .. }
        | InstKind::BeginMutBorrowAddr { result, address, .. } => {
            let addr = fc.get_value(*address);
            fc.map_value(*result, addr);
        },

        // address: ADDR → result: VALUE
        InstKind::Load { result, address } => {
            let addr = fc.get_value(*address).into_int_value();
            let ty = fc.body.values[result.index()].ty;
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            let val = mem::load_from_repr(cx, builder, ptr_size, repr, addr);
            fc.map_value(*result, val);
        },

        // address: ADDR → result: VALUE (copy of pointed-to data)
        InstKind::CopyAddr { result, address, ty } => {
            let addr = fc.get_value(*address).into_int_value();
            let repr = fc.ctx.tc.repr(*ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            match repr {
                TypeRepr::Aggregate { size, align } => {
                    let slot = fc.alloca(size, align);
                    mem::copy_aggregate(cx, builder, ptr_size, size, slot, addr);
                    fc.map_value(*result, slot.into());
                },
                _ => {
                    let val = mem::load_from_repr(cx, builder, ptr_size, repr, addr);
                    fc.map_value(*result, val);
                },
            }
        },

        // address: ADDR → result: VALUE (destructive read)
        InstKind::Take { result, address, ty } => {
            let addr = fc.resolve_scalar(builder, *address).into_int_value();
            let repr = fc.ctx.tc.repr(*ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            let val = mem::load_from_repr(cx, builder, ptr_size, repr, addr);
            fc.map_value(*result, val);
        },

        // address: ADDR, value: VALUE → writes value to address
        InstKind::StoreInit { address, value } | InstKind::StoreAssign { address, value } => {
            let addr = fc.get_value(*address).into_int_value();
            let val = fc.resolve_scalar(builder, *value);
            let ty = fc.body.values[value.index()].ty;
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            mem::store_to_repr(cx, builder, ptr_size, repr, addr, val);
        },

        InstKind::DestroyAddr { .. } => {},

        // operand: RAW (custom @guaranteed handling) → result: discriminant int
        InstKind::Discriminant { result, operand } => {
            let base = fc.get_value(*operand);
            let operand_ty = fc.body.values[operand.index()].ty;
            let repr = fc.ctx.tc.repr(operand_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            let is_guaranteed =
                fc.body.values[operand.index()].ownership == Ownership::Guaranteed;
            let disc_scalar =
                discriminant_width(operand_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            let disc_ty = disc_scalar.llvm(cx).into_int_type();
            let val: BasicValueEnum = match repr {
                TypeRepr::Scalar(_) if is_guaranteed => {
                    let p = mem::int_to_ptr(cx, builder, base.into_int_value());
                    builder.build_load(disc_ty, p, "disc").unwrap()
                },
                TypeRepr::Scalar(_) => {
                    let v = base.into_int_value();
                    let actual = v.get_type().get_bit_width();
                    let want = disc_ty.get_bit_width();
                    if actual == want {
                        base
                    } else if actual > want {
                        builder.build_int_truncate(v, disc_ty, "disc").unwrap().into()
                    } else {
                        builder.build_int_z_extend(v, disc_ty, "disc").unwrap().into()
                    }
                },
                _ => {
                    let p = mem::int_to_ptr(cx, builder, base.into_int_value());
                    builder.build_load(disc_ty, p, "disc").unwrap()
                },
            };
            fc.map_value(*result, val);
        },

        // arg: VALUE → result: @owned scalar (or @guaranteed for PtrRead)
        InstKind::Op1 { result, op, arg } => {
            let result_is_guaranteed =
                fc.body.values[result.index()].ownership == Ownership::Guaranteed;
            if result_is_guaranteed {
                let a = fc.resolve_scalar(builder, *arg);
                fc.map_value(*result, a);
            } else if matches!(op, Op::PtrTo(_)) {
                let a = fc.get_value(*arg);
                let arg_is_guaranteed =
                    fc.body.values[arg.index()].ownership == Ownership::Guaranteed;
                if arg_is_guaranteed {
                    fc.map_value(*result, a);
                } else {
                    let val = compile_op1(fc, builder, *op, a)?;
                    fc.map_value(*result, val);
                }
            } else {
                let a = fc.resolve_scalar(builder, *arg);
                let val = compile_op1(fc, builder, *op, a)?;
                fc.map_value(*result, val);
            }
        },

        // lhs, rhs: VALUE → result: @owned scalar
        InstKind::Op2 { result, op, lhs, rhs } => {
            let l = fc.resolve_scalar(builder, *lhs);
            let r = fc.resolve_scalar(builder, *rhs);
            let val = compile_op2(fc, builder, *op, l, r)?;
            fc.map_value(*result, val);
        },

        // a, b, c: VALUE → result: @owned scalar
        InstKind::Op3 { result, op, a, b, c } => {
            let va = fc.resolve_scalar(builder, *a);
            let vb = fc.resolve_scalar(builder, *b);
            let vc = fc.resolve_scalar(builder, *c);
            let val = compile_op3(fc, builder, *op, va, vb, vc)?;
            fc.map_value(*result, val);
        },

        // result: @owned literal
        InstKind::Literal { result, value } => {
            let val = crate::imm::compile_immediate(fc, builder, &value.kind)?;
            fc.map_value(*result, val);
        },

        // result: ADDR (global data pointer)
        InstKind::GlobalRef { result, entity } => {
            let global = *fc.ctx.static_data.get(entity).ok_or_else(|| {
                CodegenError::Unsupported("global entity not found in statics".into())
            })?;
            let addr = mem::ptr_to_int(cx, builder, global.as_pointer_value(), ptr_size);
            fc.map_value(*result, addr.into());
        },

        // fields: VALUE each → result: @owned aggregate/scalar
        InstKind::Struct { result, ty, fields } => {
            let val = compile_struct(fc, builder, *ty, fields)?;
            fc.map_value(*result, val);
        },

        // elements: VALUE each → result: @owned aggregate
        InstKind::Tuple { result, elements } => {
            let val = compile_tuple(fc, builder, elements)?;
            fc.map_value(*result, val);
        },

        // payload: VALUE each → result: @owned enum
        InstKind::Enum { result, enum_ty, variant, payload } => {
            let val = compile_enum(fc, builder, *enum_ty, *variant, payload)?;
            fc.map_value(*result, val);
        },

        // elements: VALUE each → result: @owned aggregate
        InstKind::Array { result, element_ty, elements } => {
            let val = compile_array(fc, builder, *element_ty, elements)?;
            fc.map_value(*result, val);
        },

        // captures: VALUE each → result: @owned closure pair
        InstKind::ApplyPartial { result, callee, captures } => {
            let val = compile_apply_partial(fc, builder, callee, captures)?;
            fc.map_value(*result, val);
        },

        InstKind::StructExtract { result, operand, field } => {
            let val = compile_struct_extract(fc, builder, *operand, *field)?;
            fc.map_value(*result, val);
        },

        InstKind::TupleExtract { result, operand, index } => {
            let val = compile_tuple_extract(fc, builder, *operand, *index)?;
            fc.map_value(*result, val);
        },

        InstKind::EnumPayload { result, operand, variant, field } => {
            let val = compile_enum_payload(fc, builder, *operand, *variant, *field)?;
            fc.map_value(*result, val);
        },

        InstKind::DestructureStruct { results, operand } => {
            for (i, &result_id) in results.iter().enumerate() {
                let val = compile_struct_extract(fc, builder, *operand, FieldIdx::new(i))?;
                fc.map_value(result_id, val);
            }
        },

        InstKind::DestructureTuple { results, operand } => {
            for (i, &result_id) in results.iter().enumerate() {
                let val = compile_tuple_extract(fc, builder, *operand, i as u32)?;
                fc.map_value(result_id, val);
            }
        },

        InstKind::DestructureEnum { results, operand, variant } => {
            for (i, &result_id) in results.iter().enumerate() {
                let val = compile_enum_payload(fc, builder, *operand, *variant, FieldIdx::new(i))?;
                fc.map_value(result_id, val);
            }
        },

        // base: ADDR → result: ADDR (offset into aggregate)
        InstKind::FieldAddr { result, base, ty, field } => {
            let base_val = fc.get_value(*base).into_int_value();
            let offset =
                struct_field_offset(*ty, *field, &fc.ctx.module.ty_arena, fc.ctx.module);
            let addr = offset_addr(cx, builder, ptr_size, base_val, offset);
            fc.map_value(*result, addr.into());
        },

        // result: ADDR (zero-initialized stack slot)
        InstKind::Uninit { result, ty } => {
            let repr = fc.ctx.tc.repr(*ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            match repr {
                TypeRepr::Aggregate { size, align } => {
                    let slot = fc.alloca(size, align);
                    mem::zero_memory(cx, builder, ptr_size, slot, size);
                    fc.map_value(*result, slot.into());
                },
                TypeRepr::Scalar(_) => {
                    let slot = fc.alloca(repr.size(), repr.align());
                    mem::zero_memory(cx, builder, ptr_size, slot, repr.size());
                    fc.map_value(*result, slot.into());
                },
                TypeRepr::Zst => {
                    fc.map_value(*result, mem::ptr_const(cx, ptr_size, 0).into());
                },
            }
        },

        // args: per-convention → result: return value
        InstKind::Call { result, callee, args } => {
            return compile_call(fc, builder, result.as_ref().copied(), callee, args);
        },
    }

    Ok(false)
}

// ======================================================================
// Operations
// ======================================================================

/// Zero-extend an i1 comparison result to the i8 Kestrel `Bool`.
fn cmp_to_bool<'ctx>(
    cx: &'ctx inkwell::context::Context,
    builder: &Builder<'ctx>,
    cmp: IntValue<'ctx>,
) -> BasicValueEnum<'ctx> {
    builder.build_int_z_extend(cmp, cx.i8_type(), "b").unwrap().into()
}

/// Compute `base + offset` (both pointer-width integers). Identity at offset 0.
fn offset_addr<'ctx>(
    cx: &'ctx inkwell::context::Context,
    builder: &Builder<'ctx>,
    ptr_size: u64,
    base: IntValue<'ctx>,
    offset: u64,
) -> IntValue<'ctx> {
    if offset == 0 {
        base
    } else {
        builder
            .build_int_add(base, mem::ptr_const(cx, ptr_size, offset as i64), "fa")
            .unwrap()
    }
}

/// Call an LLVM intrinsic by name with the given overload types and arguments.
fn call_intrinsic<'ctx>(
    llmod: &Module<'ctx>,
    builder: &Builder<'ctx>,
    name: &str,
    overloads: &[BasicTypeEnum<'ctx>],
    args: &[BasicMetadataValueEnum<'ctx>],
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    let intr = Intrinsic::find(name)
        .ok_or_else(|| CodegenError::Unsupported(format!("unknown intrinsic {name}")))?;
    let f = intr
        .get_declaration(llmod, overloads)
        .ok_or_else(|| CodegenError::Unsupported(format!("intrinsic decl {name}")))?;
    builder
        .build_call(f, args, "intr")
        .unwrap()
        .try_as_basic_value().basic()
        .ok_or_else(|| CodegenError::Unsupported(format!("intrinsic {name} returned void")))
}

fn compile_op1<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    op: Op,
    arg: BasicValueEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;

    Ok(match op {
        Op::Neg(_) => builder.build_int_neg(arg.into_int_value(), "neg").unwrap().into(),
        Op::FNeg(_) => builder.build_float_neg(arg.into_float_value(), "fneg").unwrap().into(),
        Op::Not(_) => builder.build_not(arg.into_int_value(), "not").unwrap().into(),
        Op::Popcount(_) => {
            let v = arg.into_int_value();
            let ty: BasicTypeEnum = v.get_type().into();
            call_intrinsic(&fc.ctx.llmod, builder, "llvm.ctpop", &[ty], &[v.into()])?
        },
        Op::Clz(_) => {
            let v = arg.into_int_value();
            let ty: BasicTypeEnum = v.get_type().into();
            let poison = cx.bool_type().const_zero();
            call_intrinsic(&fc.ctx.llmod, builder, "llvm.ctlz", &[ty], &[v.into(), poison.into()])?
        },
        Op::Ctz(_) => {
            let v = arg.into_int_value();
            let ty: BasicTypeEnum = v.get_type().into();
            let poison = cx.bool_type().const_zero();
            call_intrinsic(&fc.ctx.llmod, builder, "llvm.cttz", &[ty], &[v.into(), poison.into()])?
        },
        Op::Bswap(_) => {
            let v = arg.into_int_value();
            let ty: BasicTypeEnum = v.get_type().into();
            call_intrinsic(&fc.ctx.llmod, builder, "llvm.bswap", &[ty], &[v.into()])?
        },
        Op::BoolNot => {
            let one = cx.i8_type().const_int(1, false);
            builder.build_xor(arg.into_int_value(), one, "bnot").unwrap().into()
        },
        Op::IntWiden(_, to) => builder
            .build_int_s_extend(arg.into_int_value(), int_bits_to_scalar(to).llvm(cx).into_int_type(), "sext")
            .unwrap()
            .into(),
        Op::IntUnsignedWiden(_, to) => builder
            .build_int_z_extend(arg.into_int_value(), int_bits_to_scalar(to).llvm(cx).into_int_type(), "zext")
            .unwrap()
            .into(),
        Op::IntTruncate(_, to) => builder
            .build_int_truncate(arg.into_int_value(), int_bits_to_scalar(to).llvm(cx).into_int_type(), "tr")
            .unwrap()
            .into(),
        Op::IntToFloat(_, fb) => builder
            .build_signed_int_to_float(arg.into_int_value(), float_bits_to_scalar(fb).llvm(cx).into_float_type(), "i2f")
            .unwrap()
            .into(),
        // Non-saturating fptosi (cf. Cranelift's saturating variant): out-of-range
        // or NaN inputs are UB rather than clamped. Acceptable for in-range uses.
        Op::FloatToInt(_, ib) => builder
            .build_float_to_signed_int(arg.into_float_value(), int_bits_to_scalar(ib).llvm(cx).into_int_type(), "f2i")
            .unwrap()
            .into(),
        Op::FloatWiden(_, to) => builder
            .build_float_ext(arg.into_float_value(), float_bits_to_scalar(to).llvm(cx).into_float_type(), "fext")
            .unwrap()
            .into(),
        Op::FloatTruncate(_, to) => builder
            .build_float_trunc(arg.into_float_value(), float_bits_to_scalar(to).llvm(cx).into_float_type(), "ftr")
            .unwrap()
            .into(),
        Op::RefToImmut => arg,
        Op::PtrFromAddress(_) => arg,
        Op::PtrToAddress => arg,
        Op::PtrIsNull => {
            let zero = arg.into_int_value().get_type().const_zero();
            let cmp = builder
                .build_int_compare(IntPredicate::EQ, arg.into_int_value(), zero, "isnull")
                .unwrap();
            cmp_to_bool(cx, builder, cmp)
        },
        Op::PtrNull(_) => mem::ptr_const(cx, ptr_size, 0).into(),
        Op::PtrTo(ty) => {
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            let slot = fc.alloca(repr.size(), repr.align());
            mem::store_to_repr(cx, builder, ptr_size, repr, slot, arg);
            slot.into()
        },
        Op::PtrCast(_) | Op::PtrBitcast(_) => arg,
        Op::RefToPtr => arg,
        Op::PtrRead(ty) => {
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            mem::load_from_repr(cx, builder, ptr_size, repr, arg.into_int_value())
        },
        Op::SizeOf(ty) => {
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            mem::ptr_const(cx, ptr_size, repr.size() as i64).into()
        },
        Op::AlignOf(ty) => {
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            mem::ptr_const(cx, ptr_size, repr.align() as i64).into()
        },
        Op::StackAlloc(ty) => {
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            let count = arg
                .into_int_value()
                .get_zero_extended_constant()
                .expect("ICE: StackAlloc element count is not a compile-time constant");
            fc.alloca(repr.size() * count, repr.align()).into()
        },
        Op::StrPtr => {
            let repr = TypeRepr::Scalar(fc.ctx.tc.ptr_scalar);
            mem::load_from_repr(cx, builder, ptr_size, repr, arg.into_int_value())
        },
        Op::StrLen => {
            let ptr_scalar = fc.ctx.tc.ptr_scalar;
            let addr = offset_addr(cx, builder, ptr_size, arg.into_int_value(), ptr_size);
            mem::load_from_repr(cx, builder, ptr_size, TypeRepr::Scalar(ptr_scalar), addr)
        },
        Op::FloatPred(_, FloatPredicateKind::IsNan) => {
            let v = arg.into_float_value();
            let cmp = builder.build_float_compare(FloatPredicate::UNO, v, v, "isnan").unwrap();
            cmp_to_bool(cx, builder, cmp)
        },
        Op::FloatPred(_, FloatPredicateKind::IsInfinite) => {
            let v = arg.into_float_value();
            let fty: BasicTypeEnum = v.get_type().into();
            let absv = call_intrinsic(&fc.ctx.llmod, builder, "llvm.fabs", &[fty], &[v.into()])?
                .into_float_value();
            let inf = v.get_type().const_float(f64::INFINITY);
            let cmp = builder.build_float_compare(FloatPredicate::OEQ, absv, inf, "isinf").unwrap();
            cmp_to_bool(cx, builder, cmp)
        },
        Op::FloatMath(_, kind) => {
            let v = arg.into_float_value();
            let fty: BasicTypeEnum = v.get_type().into();
            let name = match kind {
                FloatMathKind::Floor => "llvm.floor",
                FloatMathKind::Ceil => "llvm.ceil",
                FloatMathKind::Round => "llvm.roundeven",
                FloatMathKind::Trunc => "llvm.trunc",
                FloatMathKind::Sqrt => "llvm.sqrt",
            };
            call_intrinsic(&fc.ctx.llmod, builder, name, &[fty], &[v.into()])?
        },
        _ => {
            return Err(CodegenError::Unsupported(format!("op1 variant: {op:?}")));
        },
    })
}

fn compile_op2<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    op: Op,
    lhs: BasicValueEnum<'ctx>,
    rhs: BasicValueEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;
    let li = || lhs.into_int_value();
    let ri = || rhs.into_int_value();
    let lf = || lhs.into_float_value();
    let rf = || rhs.into_float_value();

    let icmp = |p: IntPredicate, b: &Builder<'ctx>| {
        let c = b.build_int_compare(p, lhs.into_int_value(), rhs.into_int_value(), "icmp").unwrap();
        cmp_to_bool(cx, b, c)
    };
    let fcmp = |p: FloatPredicate, b: &Builder<'ctx>| {
        let c = b.build_float_compare(p, lhs.into_float_value(), rhs.into_float_value(), "fcmp").unwrap();
        cmp_to_bool(cx, b, c)
    };

    Ok(match op {
        Op::Add(_, _) => builder.build_int_add(li(), ri(), "add").unwrap().into(),
        Op::Sub(_, _) => builder.build_int_sub(li(), ri(), "sub").unwrap().into(),
        Op::Mul(_, _) => builder.build_int_mul(li(), ri(), "mul").unwrap().into(),
        Op::Div(_, Signedness::Signed) => builder.build_int_signed_div(li(), ri(), "sdiv").unwrap().into(),
        Op::Div(_, Signedness::Unsigned) => builder.build_int_unsigned_div(li(), ri(), "udiv").unwrap().into(),
        Op::Rem(_, Signedness::Signed) => builder.build_int_signed_rem(li(), ri(), "srem").unwrap().into(),
        Op::Rem(_, Signedness::Unsigned) => builder.build_int_unsigned_rem(li(), ri(), "urem").unwrap().into(),
        Op::FAdd(_) => builder.build_float_add(lf(), rf(), "fadd").unwrap().into(),
        Op::FSub(_) => builder.build_float_sub(lf(), rf(), "fsub").unwrap().into(),
        Op::FMul(_) => builder.build_float_mul(lf(), rf(), "fmul").unwrap().into(),
        Op::FDiv(_) => builder.build_float_div(lf(), rf(), "fdiv").unwrap().into(),
        Op::And(_) => builder.build_and(li(), ri(), "and").unwrap().into(),
        Op::Or(_) => builder.build_or(li(), ri(), "or").unwrap().into(),
        Op::Xor(_) => builder.build_xor(li(), ri(), "xor").unwrap().into(),
        Op::Shl(_) => builder.build_left_shift(li(), ri(), "shl").unwrap().into(),
        Op::Shr(_, Signedness::Signed) => builder.build_right_shift(li(), ri(), true, "ashr").unwrap().into(),
        Op::Shr(_, Signedness::Unsigned) => builder.build_right_shift(li(), ri(), false, "lshr").unwrap().into(),
        Op::Eq(_) => icmp(IntPredicate::EQ, builder),
        Op::Ne(_) => icmp(IntPredicate::NE, builder),
        Op::Lt(_, Signedness::Signed) => icmp(IntPredicate::SLT, builder),
        Op::Lt(_, Signedness::Unsigned) => icmp(IntPredicate::ULT, builder),
        Op::Le(_, Signedness::Signed) => icmp(IntPredicate::SLE, builder),
        Op::Le(_, Signedness::Unsigned) => icmp(IntPredicate::ULE, builder),
        Op::Gt(_, Signedness::Signed) => icmp(IntPredicate::SGT, builder),
        Op::Gt(_, Signedness::Unsigned) => icmp(IntPredicate::UGT, builder),
        Op::Ge(_, Signedness::Signed) => icmp(IntPredicate::SGE, builder),
        Op::Ge(_, Signedness::Unsigned) => icmp(IntPredicate::UGE, builder),
        Op::FEq(_) => fcmp(FloatPredicate::OEQ, builder),
        Op::FNe(_) => fcmp(FloatPredicate::UNE, builder),
        Op::FLt(_) => fcmp(FloatPredicate::OLT, builder),
        Op::FLe(_) => fcmp(FloatPredicate::OLE, builder),
        Op::FGt(_) => fcmp(FloatPredicate::OGT, builder),
        Op::FGe(_) => fcmp(FloatPredicate::OGE, builder),
        Op::BoolAnd => builder.build_and(li(), ri(), "booland").unwrap().into(),
        Op::BoolOr => builder.build_or(li(), ri(), "boolor").unwrap().into(),
        Op::BoolEq => icmp(IntPredicate::EQ, builder),
        Op::PtrOffset => builder.build_int_add(li(), ri(), "ptroff").unwrap().into(),
        Op::PtrWrite(ty) => {
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            mem::store_to_repr(cx, builder, ptr_size, repr, li(), rhs);
            mem::ptr_const(cx, ptr_size, 0).into()
        },
        Op::AtomicAdd => {
            let p = mem::int_to_ptr(cx, builder, li());
            builder
                .build_atomicrmw(AtomicRMWBinOp::Add, p, ri(), AtomicOrdering::SequentiallyConsistent)
                .unwrap()
                .into()
        },
        Op::AtomicSub => {
            let p = mem::int_to_ptr(cx, builder, li());
            builder
                .build_atomicrmw(AtomicRMWBinOp::Sub, p, ri(), AtomicOrdering::SequentiallyConsistent)
                .unwrap()
                .into()
        },
        Op::FloatCopysign(_) => {
            let l = lf();
            let fty: BasicTypeEnum = l.get_type().into();
            call_intrinsic(&fc.ctx.llmod, builder, "llvm.copysign", &[fty], &[l.into(), rf().into()])?
        },
        _ => {
            return Err(CodegenError::Unsupported(format!("op2 variant: {op:?}")));
        },
    })
}

fn compile_op3<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    op: Op,
    a: BasicValueEnum<'ctx>,
    b: BasicValueEnum<'ctx>,
    c: BasicValueEnum<'ctx>,
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    Ok(match op {
        Op::FloatFma(_) => {
            let av = a.into_float_value();
            let fty: BasicTypeEnum = av.get_type().into();
            call_intrinsic(
                &fc.ctx.llmod,
                builder,
                "llvm.fma",
                &[fty],
                &[av.into(), b.into_float_value().into(), c.into_float_value().into()],
            )?
        },
        _ => {
            return Err(CodegenError::Unsupported(format!("op3 variant: {op:?}")));
        },
    })
}

// ======================================================================
// Aggregate construction
// ======================================================================

fn compile_struct<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    ty: TyId,
    fields: &[(FieldIdx, ValueId)],
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;
    let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);

    match repr {
        TypeRepr::Zst => Ok(mem::ptr_const(cx, ptr_size, 0).into()),
        TypeRepr::Scalar(t) => {
            if fields.len() == 1 {
                return Ok(fc.resolve_scalar(builder, fields[0].1));
            }
            let slot = fc.alloca(repr.size(), repr.align());
            mem::zero_memory(cx, builder, ptr_size, slot, repr.size());
            store_struct_fields(fc, builder, ty, fields, slot)?;
            let p = mem::int_to_ptr(cx, builder, slot);
            Ok(builder.build_load(t.llvm(cx), p, "struct").unwrap())
        },
        TypeRepr::Aggregate { size, align } => {
            let slot = fc.alloca(size, align);
            mem::zero_memory(cx, builder, ptr_size, slot, size);
            store_struct_fields(fc, builder, ty, fields, slot)?;
            Ok(slot.into())
        },
    }
}

fn store_struct_fields<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    ty: TyId,
    fields: &[(FieldIdx, ValueId)],
    slot: IntValue<'ctx>,
) -> Result<(), CodegenError> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;
    for &(field_idx, value_id) in fields {
        let val = fc.resolve_scalar(builder, value_id);
        let offset = struct_field_offset(ty, field_idx, &fc.ctx.module.ty_arena, fc.ctx.module);
        let field_ty = struct_field_type(ty, field_idx, &fc.ctx.module.ty_arena, fc.ctx.module);
        let field_repr = fc.ctx.tc.repr(field_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
        let dest = offset_addr(cx, builder, ptr_size, slot, offset);
        mem::store_to_repr(cx, builder, ptr_size, field_repr, dest, val);
    }
    Ok(())
}

fn compile_tuple<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    elements: &[ValueId],
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;

    if elements.is_empty() {
        return Ok(mem::ptr_const(cx, ptr_size, 0).into());
    }

    let mut layout = StructLayout::new();
    let mut elem_tys = Vec::with_capacity(elements.len());
    for &elem_id in elements {
        let ty = fc.body.values[elem_id.index()].ty;
        let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
        layout.append_field(StructLayout::scalar(repr.size(), repr.align()));
        elem_tys.push(ty);
    }
    layout.pad_to_align();

    let slot = fc.alloca(layout.size, layout.align);
    mem::zero_memory(cx, builder, ptr_size, slot, layout.size);

    for (i, &elem_id) in elements.iter().enumerate() {
        let val = fc.resolve_scalar(builder, elem_id);
        let offset = layout.field_offsets[i];
        let repr = fc.ctx.tc.repr(elem_tys[i], &fc.ctx.module.ty_arena, fc.ctx.module);
        let dest = offset_addr(cx, builder, ptr_size, slot, offset);
        mem::store_to_repr(cx, builder, ptr_size, repr, dest, val);
    }

    Ok(slot.into())
}

fn compile_enum<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    enum_ty: TyId,
    variant: VariantIdx,
    payload: &[ValueId],
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;
    let repr = fc.ctx.tc.repr(enum_ty, &fc.ctx.module.ty_arena, fc.ctx.module);

    let (disc_scalar, payload_offset, disc_value) =
        if let MirTy::Named { entity, type_args } = fc.ctx.module.ty_arena.get(enum_ty) {
            let entity = *entity;
            let type_args = type_args.clone();
            if let Some(e) = find_mono_enum(&entity, &type_args, fc.ctx.module) {
                (
                    int_bits_to_scalar(e.discriminant_width),
                    e.payload_offset(),
                    e.cases[variant.index()].discriminant as i64,
                )
            } else {
                (ScalarTy::I8, 0u64, variant.index() as i64)
            }
        } else {
            (ScalarTy::I8, 0u64, variant.index() as i64)
        };

    match repr {
        TypeRepr::Zst => Ok(mem::ptr_const(cx, ptr_size, 0).into()),
        TypeRepr::Scalar(t) => {
            let slot = fc.alloca(repr.size(), repr.align());
            mem::zero_memory(cx, builder, ptr_size, slot, repr.size());
            let disc = disc_scalar.llvm(cx).into_int_type().const_int(disc_value as u64, false);
            let p = mem::int_to_ptr(cx, builder, slot);
            builder.build_store(p, disc).unwrap();
            store_variant_payload(fc, builder, enum_ty, variant, payload, slot, payload_offset)?;
            let p2 = mem::int_to_ptr(cx, builder, slot);
            Ok(builder.build_load(t.llvm(cx), p2, "enum").unwrap())
        },
        TypeRepr::Aggregate { size, align } => {
            let slot = fc.alloca(size, align);
            mem::zero_memory(cx, builder, ptr_size, slot, size);
            let disc = disc_scalar.llvm(cx).into_int_type().const_int(disc_value as u64, false);
            let p = mem::int_to_ptr(cx, builder, slot);
            builder.build_store(p, disc).unwrap();
            store_variant_payload(fc, builder, enum_ty, variant, payload, slot, payload_offset)?;
            Ok(slot.into())
        },
    }
}

fn store_variant_payload<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    enum_ty: TyId,
    variant: VariantIdx,
    payload: &[ValueId],
    slot: IntValue<'ctx>,
    payload_offset: u64,
) -> Result<(), CodegenError> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;
    let MirTy::Named { entity, type_args } = fc.ctx.module.ty_arena.get(enum_ty) else {
        return Ok(());
    };
    let entity = *entity;
    let type_args = type_args.clone();
    let Some(e) = find_mono_enum(&entity, &type_args, fc.ctx.module) else {
        return Ok(());
    };
    let Some(Layout::Enum(el)) = &e.type_info.layout else {
        return Ok(());
    };
    let Some(vl) = el.variant_layouts.get(variant.index()) else {
        return Ok(());
    };
    // Collect (offset, field_ty) before borrowing tc mutably for repr.
    let plan: Vec<(u64, TyId)> = payload
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let field_offset = vl.field_offsets.get(i).copied().unwrap_or_else(|| {
                panic!("ICE: enum variant field offset missing for field {i}")
            });
            let field_ty = e.cases[variant.index()].payload_fields[i].ty;
            (payload_offset + field_offset, field_ty)
        })
        .collect();

    for (i, &value_id) in payload.iter().enumerate() {
        let val = fc.resolve_scalar(builder, value_id);
        let (total_offset, field_ty) = plan[i];
        let field_repr = fc.ctx.tc.repr(field_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
        let dest = offset_addr(cx, builder, ptr_size, slot, total_offset);
        mem::store_to_repr(cx, builder, ptr_size, field_repr, dest, val);
    }
    Ok(())
}

fn compile_array<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    element_ty: TyId,
    elements: &[ValueId],
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;
    let elem_repr = fc.ctx.tc.repr(element_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
    let elem_size = elem_repr.size();
    let total_size = elem_size * elements.len() as u64;

    if total_size == 0 {
        return Ok(mem::ptr_const(cx, ptr_size, 0).into());
    }

    let slot = fc.alloca(total_size, elem_repr.align());
    mem::zero_memory(cx, builder, ptr_size, slot, total_size);

    for (i, &value_id) in elements.iter().enumerate() {
        let val = fc.resolve_scalar(builder, value_id);
        let offset = i as u64 * elem_size;
        let dest = offset_addr(cx, builder, ptr_size, slot, offset);
        mem::store_to_repr(cx, builder, ptr_size, elem_repr, dest, val);
    }

    Ok(slot.into())
}

fn compile_apply_partial<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    callee: &Callee,
    captures: &[ValueId],
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;
    let ptr_scalar = fc.ctx.tc.ptr_scalar;

    let func_addr = {
        let Callee::Resolved(mono_id) = callee else {
            return Err(CodegenError::Unsupported(format!(
                "ApplyPartial callee not resolved to a mono instance: {callee:?}"
            )));
        };
        let func = fc.ctx.func_ids[mono_id.index()]
            .ok_or_else(|| CodegenError::Unsupported("closure target not declared".into()))?;
        mem::ptr_to_int(cx, builder, func.as_global_value().as_pointer_value(), ptr_size)
    };

    let mut env_size = 0u64;
    let mut env_align = 1u64;
    let mut capture_reprs = Vec::new();
    for &cap_id in captures {
        let ty = fc.body.values[cap_id.index()].ty;
        let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
        let size = repr.size();
        let align = repr.align();
        env_align = env_align.max(align);
        if align > 0 {
            let padding = (align - (env_size % align)) % align;
            env_size += padding;
        }
        capture_reprs.push((repr, env_size));
        env_size += size;
    }

    let env_ptr = if env_size > 0 {
        let env_slot = fc.alloca(env_size, env_align);
        for (i, &cap_id) in captures.iter().enumerate() {
            let val = fc.resolve_scalar(builder, cap_id);
            let (repr, offset) = capture_reprs[i];
            let dest = offset_addr(cx, builder, ptr_size, env_slot, offset);
            mem::store_to_repr(cx, builder, ptr_size, repr, dest, val);
        }
        env_slot
    } else {
        mem::ptr_const(cx, ptr_size, 0)
    };

    let thick = fc.alloca(ptr_size * 2, ptr_size);
    mem::store_to_repr(cx, builder, ptr_size, TypeRepr::Scalar(ptr_scalar), thick, func_addr.into());
    let env_dest = offset_addr(cx, builder, ptr_size, thick, ptr_size);
    mem::store_to_repr(cx, builder, ptr_size, TypeRepr::Scalar(ptr_scalar), env_dest, env_ptr.into());

    Ok(thick.into())
}

// ======================================================================
// Aggregate destructuring
// ======================================================================

fn compile_struct_extract<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    operand: ValueId,
    field: FieldIdx,
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;
    let base = fc.get_value(operand).into_int_value();
    let operand_ty = fc.body.values[operand.index()].ty;
    let operand_repr = fc.ctx.tc.repr(operand_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
    let is_borrowed = fc.body.values[operand.index()].ownership == Ownership::Guaranteed;

    let field_ty = struct_field_type(operand_ty, field, &fc.ctx.module.ty_arena, fc.ctx.module);
    let field_repr = fc.ctx.tc.repr(field_ty, &fc.ctx.module.ty_arena, fc.ctx.module);

    // @guaranteed operands are always pointers. Return the field address.
    if is_borrowed {
        let offset = struct_field_offset(operand_ty, field, &fc.ctx.module.ty_arena, fc.ctx.module);
        return Ok(offset_addr(cx, builder, ptr_size, base, offset).into());
    }

    // @owned single-field newtype: value IS the field (classify_named delegates).
    if let (TypeRepr::Scalar(_), TypeRepr::Scalar(_)) = (operand_repr, field_repr) {
        return Ok(base.into());
    }

    // @owned aggregate: compute field offset and load.
    let offset = struct_field_offset(operand_ty, field, &fc.ctx.module.ty_arena, fc.ctx.module);
    let addr = offset_addr(cx, builder, ptr_size, base, offset);
    Ok(mem::load_from_repr(cx, builder, ptr_size, field_repr, addr))
}

fn compile_tuple_extract<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    operand: ValueId,
    index: u32,
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;
    let base = fc.get_value(operand).into_int_value();
    let operand_ty = fc.body.values[operand.index()].ty;
    let is_borrowed = fc.body.values[operand.index()].ownership == Ownership::Guaranteed;

    let MirTy::Tuple(elems) = fc.ctx.module.ty_arena.get(operand_ty) else {
        return Err(CodegenError::Unsupported("TupleExtract on non-tuple".into()));
    };
    let elems = elems.clone();
    let (offset, elem_ty) = tuple_elem_offset(&mut fc.ctx.tc, &fc.ctx.module.ty_arena, fc.ctx.module, &elems, index);
    let addr = offset_addr(cx, builder, ptr_size, base, offset);
    if is_borrowed {
        return Ok(addr.into());
    }
    let elem_repr = fc.ctx.tc.repr(elem_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
    Ok(mem::load_from_repr(cx, builder, ptr_size, elem_repr, addr))
}

fn compile_enum_payload<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    operand: ValueId,
    variant: VariantIdx,
    field: FieldIdx,
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;
    let base = fc.get_value(operand).into_int_value();
    let operand_ty = fc.body.values[operand.index()].ty;
    let is_borrowed = fc.body.values[operand.index()].ownership == Ownership::Guaranteed;

    let MirTy::Named { entity, type_args } = fc.ctx.module.ty_arena.get(operand_ty) else {
        return Err(CodegenError::Unsupported("EnumPayload: enum metadata not found".into()));
    };
    let entity = *entity;
    let type_args = type_args.clone();
    let (total_offset, field_ty) = {
        let Some(e) = find_mono_enum(&entity, &type_args, fc.ctx.module) else {
            return Err(CodegenError::Unsupported("EnumPayload: enum metadata not found".into()));
        };
        let payload_offset = e.payload_offset();
        let Some(Layout::Enum(el)) = &e.type_info.layout else {
            return Err(CodegenError::Unsupported("EnumPayload: enum metadata not found".into()));
        };
        let Some(vl) = el.variant_layouts.get(variant.index()) else {
            return Err(CodegenError::Unsupported("EnumPayload: enum metadata not found".into()));
        };
        let field_offset = vl.field_offsets[field.index()];
        let field_ty = e.cases[variant.index()].payload_fields[field.index()].ty;
        (payload_offset + field_offset, field_ty)
    };

    let addr = offset_addr(cx, builder, ptr_size, base, total_offset);
    if is_borrowed {
        return Ok(addr.into());
    }
    let field_repr = fc.ctx.tc.repr(field_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
    Ok(mem::load_from_repr(cx, builder, ptr_size, field_repr, addr))
}

// ======================================================================
// Calls
// ======================================================================

fn ret_ty_is_never(module: &MonoModule, ret: TyId) -> bool {
    matches!(module.ty_arena.get(ret), MirTy::Never)
}

fn compile_call<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    result: Option<ValueId>,
    callee: &Callee,
    args: &[CallArg],
) -> Result<bool, CodegenError> {
    match callee {
        Callee::Resolved(mono_id) => compile_resolved_call(fc, builder, *mono_id, args, result),
        Callee::Thin(func_val_id) => compile_thin_call(fc, builder, *func_val_id, args, result),
        Callee::Thick(closure_val_id) => compile_thick_call(fc, builder, *closure_val_id, args, result),
        Callee::Direct { func, .. } => {
            let func_name = fc.ctx.module.resolve_name(*func);
            Err(CodegenError::Unsupported(format!(
                "unresolved Direct callee post-mono: {func_name}"
            )))
        },
        Callee::Witness { protocol, method, self_type, .. } => {
            let proto_name = fc.ctx.module.resolve_name(*protocol);
            let self_desc = format!("{:?}", fc.ctx.module.ty_arena.get(*self_type));
            Err(CodegenError::Unsupported(format!(
                "unresolved Witness callee post-mono: {proto_name}.{} on {self_desc}",
                method.name,
            )))
        },
    }
}

/// Coerce a by-value call argument to the expected scalar. A @guaranteed arg (an
/// address) is loaded; a stack-spilled @owned value (a pointer-width address that
/// is not the expected scalar) is loaded too.
fn coerce_byval_arg<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    val: BasicValueEnum<'ctx>,
    arg_value: ValueId,
    expected: ScalarTy,
) -> BasicValueEnum<'ctx> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;
    let arg_is_guaranteed = fc.body.values[arg_value.index()].ownership == Ownership::Guaranteed;
    if arg_is_guaranteed {
        let arg_ty = fc.body.values[arg_value.index()].ty;
        let arg_repr = fc.ctx.tc.repr(arg_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
        if let TypeRepr::Scalar(t) = arg_repr {
            let p = mem::int_to_ptr(cx, builder, val.into_int_value());
            return builder.build_load(t.llvm(cx), p, "argld").unwrap();
        }
        return val;
    }
    let expected_ty = expected.llvm(cx);
    if val.get_type() == expected_ty {
        return val;
    }
    let ptr_bits = (ptr_size * 8) as u32;
    if val.is_int_value() && val.into_int_value().get_type().get_bit_width() == ptr_bits {
        // val is an address; load the expected scalar.
        return mem::load_from_repr(cx, builder, ptr_size, TypeRepr::Scalar(expected), val.into_int_value());
    }
    val
}

fn compile_resolved_call<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    mono_id: kestrel_mir::MonoFuncId,
    args: &[CallArg],
    result: Option<ValueId>,
) -> Result<bool, CodegenError> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;
    let module: &'ctx MonoModule = fc.ctx.module;
    let target_func = &module.functions[mono_id.index()];

    let func_val = fc.ctx.func_ids[mono_id.index()].ok_or_else(|| {
        CodegenError::Unsupported(format!("resolved function {} not declared", target_func.name))
    })?;

    let ret_repr = fc.ctx.tc.repr(target_func.ret, &module.ty_arena, module);
    let is_main = fc.ctx.is_main_function(target_func);
    let ret_mode = abi::return_mode(ret_repr, is_main);

    let mut call_args: Vec<BasicMetadataValueEnum> = Vec::new();

    let sret_slot = if matches!(ret_mode, ReturnMode::Sret) {
        let slot = fc.alloca(ret_repr.size(), ret_repr.align());
        call_args.push(slot.into());
        Some(slot)
    } else {
        None
    };

    for (i, call_arg) in args.iter().enumerate() {
        if i >= target_func.params.len() {
            break;
        }
        let param = &target_func.params[i];
        let convention = param.convention;
        let repr = fc.ctx.tc.repr(param.ty, &module.ty_arena, module);
        let pass = abi::param_pass_mode(convention, repr);
        let val = fc.get_value(call_arg.value);
        let arg_is_guaranteed =
            fc.body.values[call_arg.value.index()].ownership == Ownership::Guaranteed;

        match pass {
            PassMode::ByVal(expected) => {
                let v = coerce_byval_arg(fc, builder, val, call_arg.value, expected);
                call_args.push(v.into());
            },
            PassMode::ByRef => {
                if matches!(call_arg.convention, ParamConvention::Borrow | ParamConvention::MutBorrow) {
                    call_args.push(val.into());
                } else if arg_is_guaranteed {
                    call_args.push(val.into());
                } else {
                    let ptr_bits = (ptr_size * 8) as u32;
                    let is_addr =
                        val.is_int_value() && val.into_int_value().get_type().get_bit_width() == ptr_bits;
                    if repr.is_scalar() || !is_addr {
                        let slot = fc.alloca(repr.size(), repr.align());
                        mem::store_to_repr(cx, builder, ptr_size, repr, slot, val);
                        call_args.push(slot.into());
                    } else {
                        call_args.push(val.into());
                    }
                }
            },
            PassMode::Zst => {},
        }
    }

    let cs = builder.build_call(func_val, &call_args, "call").unwrap();

    if let Some(result_id) = result {
        match ret_mode {
            ReturnMode::Direct(_) => {
                let rv = cs.try_as_basic_value().basic().unwrap();
                fc.map_value(result_id, rv);
            },
            ReturnMode::Sret => {
                fc.map_value(result_id, sret_slot.unwrap().into());
            },
            ReturnMode::Void => {
                fc.map_value(result_id, mem::ptr_const(cx, ptr_size, 0).into());
            },
        }
    }

    if ret_ty_is_never(module, target_func.ret) {
        emit_trap(fc, builder);
        return Ok(true);
    }

    Ok(false)
}

fn compile_thin_call<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    func_val_id: ValueId,
    args: &[CallArg],
    result: Option<ValueId>,
) -> Result<bool, CodegenError> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;
    let ptr_scalar = fc.ctx.tc.ptr_scalar;
    let module: &'ctx MonoModule = fc.ctx.module;

    let func_ptr_int = fc.get_value(func_val_id).into_int_value();

    let func_ty = fc.body.values[func_val_id.index()].ty;
    let inner_ty = match module.ty_arena.get(func_ty) {
        MirTy::Pointer(inner) => *inner,
        _ => func_ty,
    };
    let (param_tys, ret_ty) = if let MirTy::FuncThin { params, ret } = module.ty_arena.get(inner_ty) {
        (params.clone(), *ret)
    } else {
        return Err(CodegenError::Unsupported("thin call on non-FuncThin".into()));
    };

    let ret_repr = fc.ctx.tc.repr(ret_ty, &module.ty_arena, module);
    let ret_mode = abi::return_mode(ret_repr, false);

    // Build the indirect-call function type.
    let mut params: Vec<BasicMetadataTypeEnum> = Vec::new();
    if matches!(ret_mode, ReturnMode::Sret) {
        params.push(ptr_scalar.llvm(cx).into());
    }
    for (ty, convention) in &param_tys {
        let repr = fc.ctx.tc.repr(*ty, &module.ty_arena, module);
        match abi::param_pass_mode(*convention, repr) {
            PassMode::ByVal(t) => params.push(t.llvm(cx).into()),
            PassMode::ByRef => params.push(ptr_scalar.llvm(cx).into()),
            PassMode::Zst => {},
        }
    }
    let fn_type = match ret_mode {
        ReturnMode::Direct(t) => t.llvm(cx).fn_type(&params, false),
        ReturnMode::Sret | ReturnMode::Void => cx.void_type().fn_type(&params, false),
    };

    let mut call_args: Vec<BasicMetadataValueEnum> = Vec::new();
    let sret_slot = if matches!(ret_mode, ReturnMode::Sret) {
        let slot = fc.alloca(ret_repr.size(), ret_repr.align());
        call_args.push(slot.into());
        Some(slot)
    } else {
        None
    };

    for (i, call_arg) in args.iter().enumerate() {
        if i >= param_tys.len() {
            break;
        }
        let (ty, convention) = param_tys[i];
        let repr = fc.ctx.tc.repr(ty, &module.ty_arena, module);
        let pass = abi::param_pass_mode(convention, repr);
        let val = fc.get_value(call_arg.value);
        match pass {
            PassMode::ByVal(expected) => {
                let v = coerce_byval_arg(fc, builder, val, call_arg.value, expected);
                call_args.push(v.into());
            },
            PassMode::ByRef => call_args.push(val.into()),
            PassMode::Zst => {},
        }
    }

    let func_ptr = mem::int_to_ptr(cx, builder, func_ptr_int);
    let cs = builder.build_indirect_call(fn_type, func_ptr, &call_args, "icall").unwrap();

    if let Some(result_id) = result {
        match ret_mode {
            ReturnMode::Direct(_) => {
                let rv = cs.try_as_basic_value().basic().unwrap();
                fc.map_value(result_id, rv);
            },
            ReturnMode::Sret => fc.map_value(result_id, sret_slot.unwrap().into()),
            ReturnMode::Void => fc.map_value(result_id, mem::ptr_const(cx, ptr_size, 0).into()),
        }
    }

    if matches!(module.ty_arena.get(ret_ty), MirTy::Never) {
        emit_trap(fc, builder);
        return Ok(true);
    }

    Ok(false)
}

fn compile_thick_call<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    closure_val_id: ValueId,
    args: &[CallArg],
    result: Option<ValueId>,
) -> Result<bool, CodegenError> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;
    let ptr_scalar = fc.ctx.tc.ptr_scalar;
    let module: &'ctx MonoModule = fc.ctx.module;

    let closure_ptr = fc.get_value(closure_val_id).into_int_value();
    // Load func_ptr and env_ptr from the {fn, env} closure pair.
    let func_ptr_int = mem::load_from_repr(cx, builder, ptr_size, TypeRepr::Scalar(ptr_scalar), closure_ptr)
        .into_int_value();
    let env_addr = offset_addr(cx, builder, ptr_size, closure_ptr, ptr_size);
    let env_ptr_int = mem::load_from_repr(cx, builder, ptr_size, TypeRepr::Scalar(ptr_scalar), env_addr)
        .into_int_value();

    let func_ty = fc.body.values[closure_val_id.index()].ty;
    let inner_ty = match module.ty_arena.get(func_ty) {
        MirTy::Pointer(inner) => *inner,
        _ => func_ty,
    };
    let (param_tys, ret_ty) = if let MirTy::FuncThick { params, ret } = module.ty_arena.get(inner_ty) {
        (params.clone(), *ret)
    } else {
        return Err(CodegenError::Unsupported(format!(
            "thick call on non-FuncThick: got {:?}",
            module.ty_arena.get(func_ty)
        )));
    };

    let ret_repr = fc.ctx.tc.repr(ret_ty, &module.ty_arena, module);
    let ret_mode = abi::return_mode(ret_repr, false);

    let mut params: Vec<BasicMetadataTypeEnum> = Vec::new();
    if matches!(ret_mode, ReturnMode::Sret) {
        params.push(ptr_scalar.llvm(cx).into());
    }
    params.push(ptr_scalar.llvm(cx).into()); // env_ptr
    for (ty, convention) in &param_tys {
        let repr = fc.ctx.tc.repr(*ty, &module.ty_arena, module);
        match abi::param_pass_mode(*convention, repr) {
            PassMode::ByVal(t) => params.push(t.llvm(cx).into()),
            PassMode::ByRef => params.push(ptr_scalar.llvm(cx).into()),
            PassMode::Zst => {},
        }
    }
    let fn_type = match ret_mode {
        ReturnMode::Direct(t) => t.llvm(cx).fn_type(&params, false),
        ReturnMode::Sret | ReturnMode::Void => cx.void_type().fn_type(&params, false),
    };

    let mut call_args: Vec<BasicMetadataValueEnum> = Vec::new();
    let sret_slot = if matches!(ret_mode, ReturnMode::Sret) {
        let slot = fc.alloca(ret_repr.size(), ret_repr.align());
        call_args.push(slot.into());
        Some(slot)
    } else {
        None
    };
    call_args.push(env_ptr_int.into());

    for (i, call_arg) in args.iter().enumerate() {
        if i >= param_tys.len() {
            break;
        }
        let (ty, convention) = param_tys[i];
        let repr = fc.ctx.tc.repr(ty, &module.ty_arena, module);
        let pass = abi::param_pass_mode(convention, repr);
        let val = fc.get_value(call_arg.value);
        let arg_is_guaranteed =
            fc.body.values[call_arg.value.index()].ownership == Ownership::Guaranteed;
        match pass {
            PassMode::ByVal(expected) => {
                if arg_is_guaranteed {
                    let v = coerce_byval_arg(fc, builder, val, call_arg.value, expected);
                    call_args.push(v.into());
                } else if matches!(call_arg.convention, ParamConvention::Borrow | ParamConvention::MutBorrow) {
                    // Borrow arg is an address; load the expected scalar.
                    let loaded = mem::load_from_repr(
                        cx, builder, ptr_size, TypeRepr::Scalar(expected), val.into_int_value(),
                    );
                    call_args.push(loaded.into());
                } else {
                    call_args.push(val.into());
                }
            },
            PassMode::ByRef => call_args.push(val.into()),
            PassMode::Zst => {},
        }
    }

    let func_ptr = mem::int_to_ptr(cx, builder, func_ptr_int);
    let cs = builder.build_indirect_call(fn_type, func_ptr, &call_args, "tcall").unwrap();

    if let Some(result_id) = result {
        match ret_mode {
            ReturnMode::Direct(_) => {
                let rv = cs.try_as_basic_value().basic().unwrap();
                fc.map_value(result_id, rv);
            },
            ReturnMode::Sret => fc.map_value(result_id, sret_slot.unwrap().into()),
            ReturnMode::Void => fc.map_value(result_id, mem::ptr_const(cx, ptr_size, 0).into()),
        }
    }

    if matches!(module.ty_arena.get(ret_ty), MirTy::Never) {
        emit_trap(fc, builder);
        return Ok(true);
    }

    Ok(false)
}

// ======================================================================
// Layout helpers
// ======================================================================

fn discriminant_width(ty: TyId, arena: &TyArena, module: &MonoModule) -> ScalarTy {
    if let MirTy::Named { entity, type_args } = arena.get(ty) {
        let entity = *entity;
        let type_args = type_args.clone();
        if let Some(e) = find_mono_enum(&entity, &type_args, module) {
            return int_bits_to_scalar(e.discriminant_width);
        }
    }
    ScalarTy::I32
}

fn struct_field_offset(
    container_ty: TyId,
    field_idx: FieldIdx,
    arena: &TyArena,
    module: &MonoModule,
) -> u64 {
    if let MirTy::Named { entity, type_args } = arena.get(container_ty) {
        let entity = *entity;
        let type_args = type_args.clone();
        if let Some(s) = find_mono_struct(&entity, &type_args, module) {
            if let Some(Layout::Struct(sl)) = &s.type_info.layout {
                return sl.field_offsets[field_idx.index()];
            }
        }
    }
    0
}

fn struct_field_type(
    container_ty: TyId,
    field_idx: FieldIdx,
    arena: &TyArena,
    module: &MonoModule,
) -> TyId {
    if let MirTy::Named { entity, type_args } = arena.get(container_ty) {
        let entity = *entity;
        let type_args = type_args.clone();
        if let Some(s) = find_mono_struct(&entity, &type_args, module) {
            return s.fields[field_idx.index()].ty;
        }
    }
    container_ty
}

fn tuple_elem_offset(
    tc: &mut crate::ty::TypeCache,
    arena: &TyArena,
    module: &MonoModule,
    elems: &[TyId],
    index: u32,
) -> (u64, TyId) {
    let mut layout = StructLayout::new();
    for (i, &elem) in elems.iter().enumerate() {
        let repr = tc.repr(elem, arena, module);
        layout.append_field(StructLayout::scalar(repr.size(), repr.align()));
        if i == index as usize {
            return (layout.field_offsets[i], elem);
        }
    }
    (0, elems[index as usize])
}

pub fn find_mono_struct<'m>(
    entity: &Entity,
    type_args: &[TyId],
    module: &'m MonoModule,
) -> Option<&'m MonoStruct> {
    module.structs.get(&(*entity, type_args.to_vec()))
}

pub fn find_mono_enum<'m>(
    entity: &Entity,
    type_args: &[TyId],
    module: &'m MonoModule,
) -> Option<&'m MonoEnum> {
    module.enums.get(&(*entity, type_args.to_vec()))
}
