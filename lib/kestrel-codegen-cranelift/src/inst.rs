use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, AbiParam, InstBuilder, MemFlags, TrapCode, Value};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use kestrel_hecs::Entity;
use kestrel_mir::callee::Callee;
use kestrel_mir::inst::{CallArg, InstKind};
use kestrel_mir::mono::{MonoEnum, MonoModule, MonoStruct};
use kestrel_mir::{
    FieldIdx, FloatBits, FloatMathKind, FloatPredicateKind, Layout, MirTy, MonoFuncId, Op,
    ParamConvention, Signedness, StructLayout, TyArena, TyId, ValueId, VariantIdx,
};

use crate::abi::{self, PassMode, ReturnMode};
use crate::error::CodegenError;
use crate::func::FuncCompiler;
use crate::ty::{float_bits_to_cl, int_bits_to_cl, TypeCache, TypeRepr};
use crate::{imm, mem};

/// Returns true if the instruction diverges (call to a !-returning function).
// Operand contract legend (enforced by verify_value_repr):
//   VALUE  — the scalar/aggregate data itself (resolve_scalar)
//   ADDR   — a memory address to load from / store to (get_value)
//   RAW    — forwarded as-is, custom ownership handling inline
pub fn compile_inst(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    kind: &InstKind,
) -> Result<bool, CodegenError> {
    match kind {
        // operand: VALUE → result: @owned value
        InstKind::MoveValue { result, operand } => {
            let val = fc.resolve_scalar(builder, *operand);
            fc.map_value(builder, *result, val);
        }

        // operand: RAW (custom @guaranteed handling) → result: @owned value
        InstKind::CopyValue { result, operand } => {
            let val = fc.get_value(builder, *operand);
            let ty = fc.body.values[result.index()].ty;
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            let operand_is_guaranteed = fc.body.values[operand.index()].ownership
                == kestrel_mir::value::Ownership::Guaranteed;
            match repr {
                TypeRepr::Aggregate { size, align } => {
                    let ptr_ty = fc.ctx.ptr_ty;
                    let slot = mem::alloc_stack_slot(builder, size, align, ptr_ty);
                    mem::copy_aggregate(builder, size, slot, val);
                    fc.map_value(builder, *result, slot);
                }
                TypeRepr::Scalar(t) if operand_is_guaranteed => {
                    let loaded = builder.ins().load(t, MemFlags::new(), val, Offset32::new(0));
                    fc.map_value(builder, *result, loaded);
                }
                _ => {
                    fc.map_value(builder, *result, val);
                }
            }
        }

        InstKind::DestroyValue { .. } => {}

        // operand: RAW (custom spill for @owned scalars) → result: ADDR
        InstKind::BeginBorrow { result, operand }
        | InstKind::BeginMutBorrow { result, operand } => {
            let val = fc.get_value(builder, *operand);
            let operand_ty = fc.body.values[operand.index()].ty;
            let is_guaranteed = fc.body.values[operand.index()].ownership
                == kestrel_mir::value::Ownership::Guaranteed;
            let repr = fc.ctx.tc.repr(operand_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            match repr {
                _ if is_guaranteed => {
                    fc.map_value(builder, *result, val);
                }
                TypeRepr::Aggregate { .. } | TypeRepr::Zst => {
                    fc.map_value(builder, *result, val);
                }
                TypeRepr::Scalar(_) => {
                    let ptr_ty = fc.ctx.ptr_ty;
                    let slot = mem::alloc_stack_slot(builder, repr.size(), repr.align(), ptr_ty);
                    builder.ins().store(MemFlags::new(), val, slot, Offset32::new(0));
                    fc.map_value(builder, *result, slot);
                }
            }
        }

        InstKind::EndBorrow { .. } | InstKind::EndMutBorrow { .. } => {}

        // address: ADDR → result: ADDR
        InstKind::BeginBorrowAddr { result, address, .. }
        | InstKind::BeginMutBorrowAddr { result, address, .. } => {
            let addr = fc.get_value(builder, *address);
            fc.map_value(builder, *result, addr);
        }

        // address: ADDR → result: VALUE
        InstKind::Load { result, address } => {
            let addr = fc.get_value(builder, *address);
            let ty = fc.body.values[result.index()].ty;
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            let val = mem::load_from_repr(builder, repr, addr, fc.ctx.ptr_ty);
            fc.map_value(builder, *result, val);
        }

        // address: ADDR → result: VALUE (copy of pointed-to data)
        InstKind::CopyAddr { result, address, ty } => {
            let addr = fc.get_value(builder, *address);
            let repr = fc.ctx.tc.repr(*ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            match repr {
                TypeRepr::Aggregate { size, align } => {
                    let ptr_ty = fc.ctx.ptr_ty;
                    let slot = mem::alloc_stack_slot(builder, size, align, ptr_ty);
                    mem::copy_aggregate(builder, size, slot, addr);
                    fc.map_value(builder, *result, slot);
                }
                _ => {
                    let val = mem::load_from_repr(builder, repr, addr, fc.ctx.ptr_ty);
                    fc.map_value(builder, *result, val);
                }
            }
        }

        // address: ADDR → result: VALUE (destructive read)
        // Resolve the address as a VALUE: a @guaranteed pointer operand (e.g. a
        // struct_extract field projection feeding a DestroyAddr-expanded drop, as in
        // `drop_in_place(self._raw)`) is represented as a pointer-TO-the-address and
        // must be loaded to recover the actual address. `resolve_scalar` is identity
        // for an @owned pointer (the common stack-slot case) and loads for @guaranteed.
        InstKind::Take { result, address, ty } => {
            let addr = fc.resolve_scalar(builder, *address);
            let repr = fc.ctx.tc.repr(*ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            let val = mem::load_from_repr(builder, repr, addr, fc.ctx.ptr_ty);
            fc.map_value(builder, *result, val);
        }

        // address: ADDR, value: VALUE → writes value to address
        InstKind::StoreInit { address, value } | InstKind::StoreAssign { address, value } => {
            let addr = fc.get_value(builder, *address);
            let val = fc.resolve_scalar(builder, *value);
            let ty = fc.body.values[value.index()].ty;
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            mem::store_to_repr(builder, repr, addr, val);
        }

        InstKind::DestroyAddr { .. } => {}

        // operand: RAW (custom @guaranteed handling) → result: discriminant int
        InstKind::Discriminant { result, operand } => {
            let base = fc.get_value(builder, *operand);
            let operand_ty = fc.body.values[operand.index()].ty;
            let repr = fc.ctx.tc.repr(operand_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            let is_guaranteed = fc.body.values[operand.index()].ownership
                == kestrel_mir::value::Ownership::Guaranteed;
            let disc_width = discriminant_width(operand_ty, &fc.ctx.module.ty_arena, fc.ctx.module, &fc.ctx.tc);
            let val = match repr {
                TypeRepr::Scalar(_) if is_guaranteed => {
                    builder.ins().load(disc_width, MemFlags::new(), base, Offset32::new(0))
                }
                TypeRepr::Scalar(_) => {
                    let actual = builder.func.dfg.value_type(base);
                    if actual == disc_width {
                        base
                    } else if actual.bytes() > disc_width.bytes() {
                        builder.ins().ireduce(disc_width, base)
                    } else {
                        builder.ins().uextend(disc_width, base)
                    }
                }
                _ => {
                    builder.ins().load(disc_width, MemFlags::new(), base, Offset32::new(0))
                }
            };
            fc.map_value(builder, *result, val);
        }

        // arg: VALUE → result: @owned scalar (or @guaranteed for PtrRead)
        InstKind::Op1 { result, op, arg } => {
            let result_is_guaranteed = fc.body.values[result.index()].ownership
                == kestrel_mir::value::Ownership::Guaranteed;
            if result_is_guaranteed {
                // @guaranteed PtrRead: resolve_scalar loads through @guaranteed
                // indirection, giving the actual pointer value (heap address).
                // The result represents "data lives at this address" — downstream
                // struct_extract computes field offsets, CopyValue loads from it.
                let a = fc.resolve_scalar(builder, *arg);
                fc.map_value(builder, *result, a);
            } else if matches!(op, Op::PtrTo(_)) {
                // PtrTo needs the ADDRESS of the arg, not the loaded value.
                // The arg is @guaranteed (Borrow convention) — its codegen
                // value is already the address we want.
                let a = fc.get_value(builder, *arg);
                let arg_is_guaranteed = fc.body.values[arg.index()].ownership
                    == kestrel_mir::value::Ownership::Guaranteed;
                if arg_is_guaranteed {
                    fc.map_value(builder, *result, a);
                } else {
                    // @owned: spill to stack so the pointer is stable
                    let val = compile_op1(fc, builder, *op, a)?;
                    fc.map_value(builder, *result, val);
                }
            } else {
                let a = fc.resolve_scalar(builder, *arg);
                let val = compile_op1(fc, builder, *op, a)?;
                fc.map_value(builder, *result, val);
            }
        }

        // lhs, rhs: VALUE → result: @owned scalar
        InstKind::Op2 { result, op, lhs, rhs } => {
            let l = fc.resolve_scalar(builder, *lhs);
            let r = fc.resolve_scalar(builder, *rhs);
            let val = compile_op2(fc, builder, *op, l, r)?;
            fc.map_value(builder, *result, val);
        }

        // a, b, c: VALUE → result: @owned scalar
        InstKind::Op3 { result, op, a, b, c } => {
            let va = fc.resolve_scalar(builder, *a);
            let vb = fc.resolve_scalar(builder, *b);
            let vc = fc.resolve_scalar(builder, *c);
            let val = compile_op3(builder, *op, va, vb, vc)?;
            fc.map_value(builder, *result, val);
        }

        // result: @owned literal
        InstKind::Literal { result, value } => {
            let val = imm::compile_immediate(fc.ctx, builder, &value.kind)?;
            fc.map_value(builder, *result, val);
        }

        // result: ADDR (global data pointer)
        InstKind::GlobalRef { result, entity } => {
            let ptr_ty = fc.ctx.ptr_ty;
            let data_id = fc.ctx.static_data.get(entity).ok_or_else(|| {
                CodegenError::Unsupported("global entity not found in statics".into())
            })?;
            let gv = fc.ctx.cl_module.declare_data_in_func(*data_id, builder.func);
            let addr = builder.ins().global_value(ptr_ty, gv);
            fc.map_value(builder, *result, addr);
        }

        // fields: VALUE each → result: @owned aggregate/scalar
        InstKind::Struct { result, ty, fields } => {
            let val = compile_struct(fc, builder, *ty, fields)?;
            fc.map_value(builder, *result, val);
        }

        // elements: VALUE each → result: @owned aggregate
        InstKind::Tuple { result, elements } => {
            let val = compile_tuple(fc, builder, elements)?;
            fc.map_value(builder, *result, val);
        }

        // payload: VALUE each → result: @owned enum
        InstKind::Enum { result, enum_ty, variant, payload } => {
            let val = compile_enum(fc, builder, *enum_ty, *variant, payload)?;
            fc.map_value(builder, *result, val);
        }

        // elements: VALUE each → result: @owned aggregate
        InstKind::Array { result, element_ty, elements } => {
            let val = compile_array(fc, builder, *element_ty, elements)?;
            fc.map_value(builder, *result, val);
        }

        // captures: VALUE each → result: @owned closure pair
        InstKind::ApplyPartial { result, callee, captures } => {
            let val = compile_apply_partial(fc, builder, callee, captures)?;
            fc.map_value(builder, *result, val);
        }

        // operand: RAW (@guaranteed→ADDR, @owned→VALUE) → result: field value/addr
        InstKind::StructExtract { result, operand, field } => {
            let val = compile_struct_extract(fc, builder, *operand, *field)?;
            fc.map_value(builder, *result, val);
        }

        // operand: RAW (@guaranteed→ADDR, @owned→VALUE) → result: element value/addr
        InstKind::TupleExtract { result, operand, index } => {
            let val = compile_tuple_extract(fc, builder, *operand, *index)?;
            fc.map_value(builder, *result, val);
        }

        // operand: RAW (@guaranteed→ADDR, @owned→VALUE) → result: payload value/addr
        InstKind::EnumPayload { result, operand, variant, field } => {
            let val = compile_enum_payload(fc, builder, *operand, *variant, *field)?;
            fc.map_value(builder, *result, val);
        }

        InstKind::DestructureStruct { results, operand } => {
            compile_destructure_struct(fc, builder, results, *operand)?;
        }

        InstKind::DestructureTuple { results, operand } => {
            compile_destructure_tuple(fc, builder, results, *operand)?;
        }

        InstKind::DestructureEnum { results, operand, variant } => {
            compile_destructure_enum(fc, builder, results, *operand, *variant)?;
        }

        // base: ADDR → result: ADDR (offset into aggregate)
        InstKind::FieldAddr { result, base, ty, field } => {
            let base_val = fc.get_value(builder, *base);
            let offset = struct_field_offset(*ty, *field, &fc.ctx.module.ty_arena, fc.ctx.module, &fc.ctx.tc);
            let addr = if offset != 0 {
                builder.ins().iadd_imm(base_val, offset as i64)
            } else {
                base_val
            };
            fc.map_value(builder, *result, addr);
        }

        // result: ADDR (zero-initialized stack slot)
        InstKind::Uninit { result, ty } => {
            let repr = fc.ctx.tc.repr(*ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            let ptr_ty = fc.ctx.ptr_ty;
            match repr {
                TypeRepr::Aggregate { size, align } => {
                    let slot = mem::alloc_stack_slot(builder, size, align, ptr_ty);
                    mem::zero_memory(builder, slot, size);
                    fc.map_value(builder, *result, slot);
                }
                TypeRepr::Scalar(_) => {
                    let slot = mem::alloc_stack_slot(builder, repr.size(), repr.align(), ptr_ty);
                    mem::zero_memory(builder, slot, repr.size());
                    fc.map_value(builder, *result, slot);
                }
                TypeRepr::Zst => {
                    let zero = builder.ins().iconst(ptr_ty, 0);
                    fc.map_value(builder, *result, zero);
                }
            }
        }

        // args: per-convention (see compile_resolved_call) → result: return value
        InstKind::Call { result, callee, args } => {
            return compile_call(fc, builder, result.as_ref().copied(), callee, args);
        }
    }

    Ok(false)
}

// ======================================================================
// Operations
// ======================================================================

fn cmp_to_bool(builder: &mut FunctionBuilder, cmp: Value) -> Value {
    let ty = builder.func.dfg.value_type(cmp);
    if ty == ir::types::I8 {
        cmp
    } else {
        builder.ins().uextend(ir::types::I8, cmp)
    }
}

fn resolve_iconst(builder: &FunctionBuilder, val: Value) -> Option<i64> {
    use cranelift_codegen::ir::InstructionData;
    let dfg = &builder.func.dfg;
    let val = dfg.resolve_aliases(val);
    if let ir::ValueDef::Result(inst, 0) = dfg.value_def(val) {
        if let InstructionData::UnaryImm { imm, .. } = dfg.insts[inst] {
            return Some(imm.bits());
        }
    }
    None
}

fn compile_op1(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    op: Op,
    arg: Value,
) -> Result<Value, CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;

    Ok(match op {
        Op::Neg(bits) => {
            let zero = builder.ins().iconst(int_bits_to_cl(bits), 0);
            builder.ins().isub(zero, arg)
        }
        Op::FNeg(_) => builder.ins().fneg(arg),
        Op::Not(_) => builder.ins().bnot(arg),
        Op::Popcount(_) => builder.ins().popcnt(arg),
        Op::Clz(_) => builder.ins().clz(arg),
        Op::Ctz(_) => builder.ins().ctz(arg),
        Op::Bswap(_) => builder.ins().bswap(arg),
        Op::BoolNot => {
            let one = builder.ins().iconst(ir::types::I8, 1);
            builder.ins().bxor(arg, one)
        }
        Op::IntWiden(_, to) => builder.ins().sextend(int_bits_to_cl(to), arg),
        Op::IntUnsignedWiden(_, to) => builder.ins().uextend(int_bits_to_cl(to), arg),
        Op::IntTruncate(_, to) => builder.ins().ireduce(int_bits_to_cl(to), arg),
        Op::IntToFloat(_, fb) => builder.ins().fcvt_from_sint(float_bits_to_cl(fb), arg),
        Op::FloatToInt(_, ib) => builder.ins().fcvt_to_sint_sat(int_bits_to_cl(ib), arg),
        Op::FloatWiden(_, to) => builder.ins().fpromote(float_bits_to_cl(to), arg),
        Op::FloatTruncate(_, to) => builder.ins().fdemote(float_bits_to_cl(to), arg),
        Op::RefToImmut => arg,
        Op::PtrFromAddress(_) => arg,
        Op::PtrToAddress => arg,
        Op::PtrIsNull => builder.ins().icmp_imm(IntCC::Equal, arg, 0),
        Op::PtrNull(_) => builder.ins().iconst(ptr_ty, 0),
        Op::PtrTo(ty) => {
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            let slot = mem::alloc_stack_slot(builder, repr.size(), repr.align(), ptr_ty);
            mem::store_to_repr(builder, repr, slot, arg);
            slot
        }
        Op::PtrCast(_) | Op::PtrBitcast(_) => arg,
        Op::RefToPtr => arg,
        Op::PtrRead(ty) => {
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            mem::load_from_repr(builder, repr, arg, ptr_ty)
        }
        Op::SizeOf(ty) => {
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            builder.ins().iconst(ptr_ty, repr.size() as i64)
        }
        Op::AlignOf(ty) => {
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            builder.ins().iconst(ptr_ty, repr.align() as i64)
        }
        Op::StackAlloc(ty) => {
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            let count = resolve_iconst(builder, arg)
                .expect("ICE: StackAlloc element count is not a compile-time constant") as u64;
            mem::alloc_stack_slot(builder, repr.size() * count, repr.align(), ptr_ty)
        }
        Op::StrPtr => builder
            .ins()
            .load(ptr_ty, MemFlags::new(), arg, Offset32::new(0)),
        Op::StrLen => {
            let ptr_size = fc.ctx.ptr_size;
            builder
                .ins()
                .load(ptr_ty, MemFlags::new(), arg, Offset32::new(ptr_size as i32))
        }
        Op::FloatPred(_, FloatPredicateKind::IsNan) => {
            let cmp = builder.ins().fcmp(FloatCC::Unordered, arg, arg);
            cmp_to_bool(builder, cmp)
        }
        Op::FloatPred(fb, FloatPredicateKind::IsInfinite) => {
            let abs = builder.ins().fabs(arg);
            let inf = match fb {
                FloatBits::F16 | FloatBits::F32 => builder.ins().f32const(f32::INFINITY),
                FloatBits::F64 => builder.ins().f64const(f64::INFINITY),
            };
            let is_inf = builder.ins().fcmp(FloatCC::Equal, abs, inf);
            cmp_to_bool(builder, is_inf)
        }
        Op::FloatMath(_, FloatMathKind::Floor) => builder.ins().floor(arg),
        Op::FloatMath(_, FloatMathKind::Ceil) => builder.ins().ceil(arg),
        Op::FloatMath(_, FloatMathKind::Round) => builder.ins().nearest(arg),
        Op::FloatMath(_, FloatMathKind::Trunc) => builder.ins().trunc(arg),
        Op::FloatMath(_, FloatMathKind::Sqrt) => builder.ins().sqrt(arg),
        _ => {
            return Err(CodegenError::Unsupported(format!("op1 variant: {op:?}")));
        }
    })
}

fn compile_op2(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    op: Op,
    lhs: Value,
    rhs: Value,
) -> Result<Value, CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;

    Ok(match op {
        Op::Add(_, _) => builder.ins().iadd(lhs, rhs),
        Op::Sub(_, _) => builder.ins().isub(lhs, rhs),
        Op::Mul(_, _) => builder.ins().imul(lhs, rhs),
        Op::Div(_, Signedness::Signed) => builder.ins().sdiv(lhs, rhs),
        Op::Div(_, Signedness::Unsigned) => builder.ins().udiv(lhs, rhs),
        Op::Rem(_, Signedness::Signed) => builder.ins().srem(lhs, rhs),
        Op::Rem(_, Signedness::Unsigned) => builder.ins().urem(lhs, rhs),
        Op::FAdd(_) => builder.ins().fadd(lhs, rhs),
        Op::FSub(_) => builder.ins().fsub(lhs, rhs),
        Op::FMul(_) => builder.ins().fmul(lhs, rhs),
        Op::FDiv(_) => builder.ins().fdiv(lhs, rhs),
        Op::And(_) => builder.ins().band(lhs, rhs),
        Op::Or(_) => builder.ins().bor(lhs, rhs),
        Op::Xor(_) => builder.ins().bxor(lhs, rhs),
        Op::Shl(_) => builder.ins().ishl(lhs, rhs),
        Op::Shr(_, Signedness::Signed) => builder.ins().sshr(lhs, rhs),
        Op::Shr(_, Signedness::Unsigned) => builder.ins().ushr(lhs, rhs),
        Op::Eq(_) => { let c = builder.ins().icmp(IntCC::Equal, lhs, rhs); cmp_to_bool(builder, c) }
        Op::Ne(_) => { let c = builder.ins().icmp(IntCC::NotEqual, lhs, rhs); cmp_to_bool(builder, c) }
        Op::Lt(_, Signedness::Signed) => { let c = builder.ins().icmp(IntCC::SignedLessThan, lhs, rhs); cmp_to_bool(builder, c) }
        Op::Lt(_, Signedness::Unsigned) => { let c = builder.ins().icmp(IntCC::UnsignedLessThan, lhs, rhs); cmp_to_bool(builder, c) }
        Op::Le(_, Signedness::Signed) => { let c = builder.ins().icmp(IntCC::SignedLessThanOrEqual, lhs, rhs); cmp_to_bool(builder, c) }
        Op::Le(_, Signedness::Unsigned) => { let c = builder.ins().icmp(IntCC::UnsignedLessThanOrEqual, lhs, rhs); cmp_to_bool(builder, c) }
        Op::Gt(_, Signedness::Signed) => { let c = builder.ins().icmp(IntCC::SignedGreaterThan, lhs, rhs); cmp_to_bool(builder, c) }
        Op::Gt(_, Signedness::Unsigned) => { let c = builder.ins().icmp(IntCC::UnsignedGreaterThan, lhs, rhs); cmp_to_bool(builder, c) }
        Op::Ge(_, Signedness::Signed) => { let c = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, lhs, rhs); cmp_to_bool(builder, c) }
        Op::Ge(_, Signedness::Unsigned) => { let c = builder.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, lhs, rhs); cmp_to_bool(builder, c) }
        Op::FEq(_) => { let c = builder.ins().fcmp(FloatCC::Equal, lhs, rhs); cmp_to_bool(builder, c) }
        Op::FNe(_) => { let c = builder.ins().fcmp(FloatCC::NotEqual, lhs, rhs); cmp_to_bool(builder, c) }
        Op::FLt(_) => { let c = builder.ins().fcmp(FloatCC::LessThan, lhs, rhs); cmp_to_bool(builder, c) }
        Op::FLe(_) => { let c = builder.ins().fcmp(FloatCC::LessThanOrEqual, lhs, rhs); cmp_to_bool(builder, c) }
        Op::FGt(_) => { let c = builder.ins().fcmp(FloatCC::GreaterThan, lhs, rhs); cmp_to_bool(builder, c) }
        Op::FGe(_) => { let c = builder.ins().fcmp(FloatCC::GreaterThanOrEqual, lhs, rhs); cmp_to_bool(builder, c) }
        Op::BoolAnd => builder.ins().band(lhs, rhs),
        Op::BoolOr => builder.ins().bor(lhs, rhs),
        Op::BoolEq => { let c = builder.ins().icmp(IntCC::Equal, lhs, rhs); cmp_to_bool(builder, c) }
        Op::PtrOffset => builder.ins().iadd(lhs, rhs),
        Op::PtrWrite(ty) => {
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            mem::store_to_repr(builder, repr, lhs, rhs);
            builder.ins().iconst(ptr_ty, 0)
        }
        Op::AtomicAdd => {
            builder.ins().atomic_rmw(ir::types::I64, MemFlags::new(), ir::AtomicRmwOp::Add, lhs, rhs)
        }
        Op::AtomicSub => {
            builder.ins().atomic_rmw(ir::types::I64, MemFlags::new(), ir::AtomicRmwOp::Sub, lhs, rhs)
        }
        Op::FloatCopysign(_) => builder.ins().fcopysign(lhs, rhs),
        _ => {
            return Err(CodegenError::Unsupported(format!("op2 variant: {op:?}")));
        }
    })
}

fn compile_op3(
    builder: &mut FunctionBuilder,
    op: Op,
    a: Value,
    b: Value,
    c: Value,
) -> Result<Value, CodegenError> {
    Ok(match op {
        Op::FloatFma(_) => builder.ins().fma(a, b, c),
        _ => {
            return Err(CodegenError::Unsupported(format!("op3 variant: {op:?}")));
        }
    })
}

// ======================================================================
// Aggregate construction
// ======================================================================

fn compile_struct(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    ty: TyId,
    fields: &[(FieldIdx, ValueId)],
) -> Result<Value, CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;
    let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);

    match repr {
        TypeRepr::Zst => Ok(builder.ins().iconst(ptr_ty, 0)),
        TypeRepr::Scalar(t) => {
            if fields.len() == 1 {
                return Ok(fc.resolve_scalar(builder, fields[0].1));
            }
            let slot = mem::alloc_stack_slot(builder, repr.size(), repr.align(), ptr_ty);
            mem::zero_memory(builder, slot, repr.size());
            for &(field_idx, value_id) in fields {
                let val = fc.resolve_scalar(builder, value_id);
                let offset = struct_field_offset(ty, field_idx, &fc.ctx.module.ty_arena, fc.ctx.module, &fc.ctx.tc);
                let field_ty = struct_field_type(ty, field_idx, &fc.ctx.module.ty_arena, fc.ctx.module, &fc.ctx.tc);
                let field_repr = fc.ctx.tc.repr(field_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
                let dest = builder.ins().iadd_imm(slot, offset as i64);
                mem::store_to_repr(builder, field_repr, dest, val);
            }
            Ok(builder.ins().load(t, MemFlags::new(), slot, Offset32::new(0)))
        }
        TypeRepr::Aggregate { size, align } => {
            let slot = mem::alloc_stack_slot(builder, size, align, ptr_ty);
            mem::zero_memory(builder, slot, size);
            for &(field_idx, value_id) in fields {
                let val = fc.resolve_scalar(builder, value_id);
                let offset = struct_field_offset(ty, field_idx, &fc.ctx.module.ty_arena, fc.ctx.module, &fc.ctx.tc);
                let field_ty = struct_field_type(ty, field_idx, &fc.ctx.module.ty_arena, fc.ctx.module, &fc.ctx.tc);
                let field_repr = fc.ctx.tc.repr(field_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
                let dest = if offset != 0 {
                    builder.ins().iadd_imm(slot, offset as i64)
                } else {
                    slot
                };
                mem::store_to_repr(builder, field_repr, dest, val);
            }
            Ok(slot)
        }
    }
}

fn compile_tuple(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    elements: &[ValueId],
) -> Result<Value, CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;

    if elements.is_empty() {
        return Ok(builder.ins().iconst(ptr_ty, 0));
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

    let slot = mem::alloc_stack_slot(builder, layout.size, layout.align, ptr_ty);
    mem::zero_memory(builder, slot, layout.size);

    for (i, &elem_id) in elements.iter().enumerate() {
        let val = fc.resolve_scalar(builder, elem_id);
        let offset = layout.field_offsets[i];
        let repr = fc.ctx.tc.repr(elem_tys[i], &fc.ctx.module.ty_arena, fc.ctx.module);
        let dest = if offset != 0 {
            builder.ins().iadd_imm(slot, offset as i64)
        } else {
            slot
        };
        mem::store_to_repr(builder, repr, dest, val);
    }

    Ok(slot)
}

fn compile_enum(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    enum_ty: TyId,
    variant: VariantIdx,
    payload: &[ValueId],
) -> Result<Value, CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;
    let repr = fc.ctx.tc.repr(enum_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
    let arena = &fc.ctx.module.ty_arena;

    let (disc_width, payload_offset, disc_value) =
        if let MirTy::Named { entity, type_args } = arena.get(enum_ty) {
            let entity = *entity;
            let type_args = type_args.clone();
            if let Some(e) = find_mono_enum(&entity, &type_args, fc.ctx.module, &fc.ctx.tc) {
                let disc = e.cases[variant.index()].discriminant;
                (int_bits_to_cl(e.discriminant_width), e.payload_offset(), disc as i64)
            } else {
                (ir::types::I8, 0u64, variant.index() as i64)
            }
        } else {
            (ir::types::I8, 0u64, variant.index() as i64)
        };

    match repr {
        TypeRepr::Scalar(t) => {
            let slot = mem::alloc_stack_slot(builder, repr.size(), repr.align(), ptr_ty);
            mem::zero_memory(builder, slot, repr.size());
            let disc = builder.ins().iconst(disc_width, disc_value);
            builder.ins().store(MemFlags::new(), disc, slot, Offset32::new(0));
            store_variant_payload(fc, builder, enum_ty, variant, payload, slot, payload_offset)?;
            Ok(builder.ins().load(t, MemFlags::new(), slot, Offset32::new(0)))
        }
        TypeRepr::Aggregate { size, align } => {
            let slot = mem::alloc_stack_slot(builder, size, align, ptr_ty);
            mem::zero_memory(builder, slot, size);
            let disc = builder.ins().iconst(disc_width, disc_value);
            builder.ins().store(MemFlags::new(), disc, slot, Offset32::new(0));
            store_variant_payload(fc, builder, enum_ty, variant, payload, slot, payload_offset)?;
            Ok(slot)
        }
        TypeRepr::Zst => Ok(builder.ins().iconst(ptr_ty, 0)),
    }
}

fn store_variant_payload(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    enum_ty: TyId,
    variant: VariantIdx,
    payload: &[ValueId],
    slot: Value,
    payload_offset: u64,
) -> Result<(), CodegenError> {
    let arena = &fc.ctx.module.ty_arena;
    if let MirTy::Named { entity, type_args } = arena.get(enum_ty) {
        let entity = *entity;
        let type_args = type_args.clone();
        if let Some(e) = find_mono_enum(&entity, &type_args, fc.ctx.module, &fc.ctx.tc) {
            if let Some(Layout::Enum(el)) = &e.type_info.layout {
                if let Some(vl) = el.variant_layouts.get(variant.index()) {
                    for (i, &value_id) in payload.iter().enumerate() {
                        let val = fc.resolve_scalar(builder, value_id);
                        let field_offset = vl.field_offsets.get(i).copied().unwrap_or_else(|| {
                            panic!("ICE: enum variant field offset missing for field {i}")
                        });
                        let total_offset = payload_offset + field_offset;
                        let field_ty = e.cases[variant.index()].payload_fields[i].ty;
                        let field_repr = fc.ctx.tc.repr(field_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
                        let dest = builder.ins().iadd_imm(slot, total_offset as i64);
                        mem::store_to_repr(builder, field_repr, dest, val);
                    }
                }
            }
        }
    }
    Ok(())
}

fn compile_array(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    element_ty: TyId,
    elements: &[ValueId],
) -> Result<Value, CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;
    let elem_repr = fc.ctx.tc.repr(element_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
    let elem_size = elem_repr.size();
    let total_size = elem_size * elements.len() as u64;

    if total_size == 0 {
        return Ok(builder.ins().iconst(ptr_ty, 0));
    }

    let slot = mem::alloc_stack_slot(builder, total_size, elem_repr.align(), ptr_ty);
    mem::zero_memory(builder, slot, total_size);

    for (i, &value_id) in elements.iter().enumerate() {
        let val = fc.resolve_scalar(builder, value_id);
        let offset = i as u64 * elem_size;
        let dest = if offset != 0 {
            builder.ins().iadd_imm(slot, offset as i64)
        } else {
            slot
        };
        mem::store_to_repr(builder, elem_repr, dest, val);
    }

    Ok(slot)
}

fn compile_apply_partial(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    callee: &Callee,
    captures: &[ValueId],
) -> Result<Value, CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;
    let ptr_size = fc.ctx.ptr_size;

    // Monomorphization rewrites the partial-application target to a concrete
    // instance, exactly like a Call. Resolving by `MonoFuncId` (not by scanning
    // for the first function whose `source` matches the generic entity) is what
    // keeps each instantiation bound to *its own* thunk.
    let func_addr = {
        let Callee::Resolved(mono_id) = callee else {
            return Err(CodegenError::Unsupported(format!(
                "ApplyPartial callee not resolved to a mono instance: {callee:?}"
            )));
        };
        let func_id = fc.ctx.func_ids[mono_id.index()].ok_or_else(|| {
            CodegenError::Unsupported("closure target not declared".into())
        })?;
        let func_ref = fc.ctx.cl_module.declare_func_in_func(func_id, builder.func);
        builder.ins().func_addr(ptr_ty, func_ref)
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
        let env_slot = mem::alloc_stack_slot(builder, env_size, env_align, ptr_ty);
        for (i, &cap_id) in captures.iter().enumerate() {
            let val = fc.resolve_scalar(builder, cap_id);
            let (repr, offset) = capture_reprs[i];
            let dest = if offset != 0 {
                builder.ins().iadd_imm(env_slot, offset as i64)
            } else {
                env_slot
            };
            mem::store_to_repr(builder, repr, dest, val);
        }
        env_slot
    } else {
        builder.ins().iconst(ptr_ty, 0)
    };

    let thick = mem::alloc_stack_slot(builder, ptr_size * 2, ptr_size, ptr_ty);
    builder.ins().store(MemFlags::new(), func_addr, thick, Offset32::new(0));
    builder.ins().store(MemFlags::new(), env_ptr, thick, Offset32::new(ptr_size as i32));

    Ok(thick)
}

// ======================================================================
// Aggregate destructuring
// ======================================================================

fn compile_struct_extract(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    operand: ValueId,
    field: FieldIdx,
) -> Result<Value, CodegenError> {
    let base = fc.get_value(builder, operand);
    let operand_ty = fc.body.values[operand.index()].ty;
    let operand_repr = fc.ctx.tc.repr(operand_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
    let is_borrowed = fc.body.values[operand.index()].ownership == kestrel_mir::value::Ownership::Guaranteed;

    let field_ty = struct_field_type(operand_ty, field, &fc.ctx.module.ty_arena, fc.ctx.module, &fc.ctx.tc);
    let field_repr = fc.ctx.tc.repr(field_ty, &fc.ctx.module.ty_arena, fc.ctx.module);

    // @guaranteed operands are always pointers (Option B invariant).
    // Return the field address — CopyValue handles the load.
    if is_borrowed {
        let offset = struct_field_offset(operand_ty, field, &fc.ctx.module.ty_arena, fc.ctx.module, &fc.ctx.tc);
        let addr = if offset != 0 {
            builder.ins().iadd_imm(base, offset as i64)
        } else {
            base
        };
        return Ok(addr);
    }

    // @owned: single-field scalar newtype — value IS the field. A newtype's
    // representation IS its field's (classify_named delegates to it), so the @owned
    // value already carries the field's scalar; no load or bitcast coercion is needed.
    // The assert pins that single-source-of-truth invariant — if it ever fires, a
    // layout authority has diverged from classify_named again.
    if let (TypeRepr::Scalar(base_cl), TypeRepr::Scalar(field_cl)) = (operand_repr, field_repr) {
        debug_assert_eq!(
            base_cl, field_cl,
            "single-field newtype repr must equal its field repr (classify_named delegates)"
        );
        return Ok(base);
    }

    // @owned aggregate: compute field offset and load.
    let offset = struct_field_offset(operand_ty, field, &fc.ctx.module.ty_arena, fc.ctx.module, &fc.ctx.tc);
    let addr = if offset != 0 {
        builder.ins().iadd_imm(base, offset as i64)
    } else {
        base
    };
    Ok(mem::load_from_repr(builder, field_repr, addr, fc.ctx.ptr_ty))
}

fn compile_tuple_extract(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    operand: ValueId,
    index: u32,
) -> Result<Value, CodegenError> {
    let base = fc.get_value(builder, operand);
    let operand_ty = fc.body.values[operand.index()].ty;
    let is_borrowed = fc.body.values[operand.index()].ownership == kestrel_mir::value::Ownership::Guaranteed;
    let arena = &fc.ctx.module.ty_arena;

    if let MirTy::Tuple(elems) = arena.get(operand_ty) {
        let elems = elems.clone();
        let (offset, elem_ty) = tuple_elem_offset(&mut fc.ctx.tc, arena, fc.ctx.module, &elems, index);
        let addr = if offset != 0 {
            builder.ins().iadd_imm(base, offset as i64)
        } else {
            base
        };
        if is_borrowed {
            return Ok(addr);
        }
        let elem_repr = fc.ctx.tc.repr(elem_ty, arena, fc.ctx.module);
        Ok(mem::load_from_repr(builder, elem_repr, addr, fc.ctx.ptr_ty))
    } else {
        Err(CodegenError::Unsupported("TupleExtract on non-tuple".into()))
    }
}

fn compile_enum_payload(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    operand: ValueId,
    variant: VariantIdx,
    field: FieldIdx,
) -> Result<Value, CodegenError> {
    let base = fc.get_value(builder, operand);
    let operand_ty = fc.body.values[operand.index()].ty;
    let is_borrowed = fc.body.values[operand.index()].ownership == kestrel_mir::value::Ownership::Guaranteed;
    let arena = &fc.ctx.module.ty_arena;

    if let MirTy::Named { entity, type_args } = arena.get(operand_ty) {
        let entity = *entity;
        let type_args = type_args.clone();
        if let Some(e) = find_mono_enum(&entity, &type_args, fc.ctx.module, &fc.ctx.tc) {
            let payload_offset = e.payload_offset();
            if let Some(Layout::Enum(el)) = &e.type_info.layout {
                if let Some(vl) = el.variant_layouts.get(variant.index()) {
                    let field_offset = vl.field_offsets[field.index()];
                    let total_offset = payload_offset + field_offset;
                    let addr = builder.ins().iadd_imm(base, total_offset as i64);
                    if is_borrowed {
                        return Ok(addr);
                    }
                    let field_ty = e.cases[variant.index()].payload_fields[field.index()].ty;
                    let field_repr = fc.ctx.tc.repr(field_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
                    return Ok(mem::load_from_repr(builder, field_repr, addr, fc.ctx.ptr_ty));
                }
            }
        }
    }

    Err(CodegenError::Unsupported("EnumPayload: enum metadata not found".into()))
}

fn compile_destructure_struct(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    results: &[ValueId],
    operand: ValueId,
) -> Result<(), CodegenError> {
    for (i, &result_id) in results.iter().enumerate() {
        let val = compile_struct_extract(fc, builder, operand, FieldIdx::new(i))?;
        fc.map_value(builder, result_id, val);
    }
    Ok(())
}

fn compile_destructure_tuple(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    results: &[ValueId],
    operand: ValueId,
) -> Result<(), CodegenError> {
    for (i, &result_id) in results.iter().enumerate() {
        let val = compile_tuple_extract(fc, builder, operand, i as u32)?;
        fc.map_value(builder, result_id, val);
    }
    Ok(())
}

fn compile_destructure_enum(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    results: &[ValueId],
    operand: ValueId,
    variant: VariantIdx,
) -> Result<(), CodegenError> {
    for (i, &result_id) in results.iter().enumerate() {
        let val = compile_enum_payload(fc, builder, operand, variant, FieldIdx::new(i))?;
        fc.map_value(builder, result_id, val);
    }
    Ok(())
}

// ======================================================================
// Calls
// ======================================================================

fn compile_call(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
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
        }
        Callee::Witness { protocol, method, self_type, .. } => {
            let proto_name = fc.ctx.module.resolve_name(*protocol);
            let self_desc = format!("{:?}", fc.ctx.module.ty_arena.get(*self_type));
            Err(CodegenError::Unsupported(format!(
                "unresolved Witness callee post-mono: {proto_name}.{} on {self_desc}",
                method.name,
            )))
        }
    }
}

fn ret_ty_is_never(module: &MonoModule, ret: TyId) -> bool {
    matches!(module.ty_arena.get(ret), MirTy::Never)
}

fn compile_resolved_call(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    mono_id: MonoFuncId,
    args: &[CallArg],
    result: Option<ValueId>,
) -> Result<bool, CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;
    let target_func = &fc.ctx.module.functions[mono_id.index()];

    let func_id = fc.ctx.func_ids[mono_id.index()].ok_or_else(|| {
        CodegenError::Unsupported(format!("resolved function {} not declared", target_func.name))
    })?;
    let func_ref = fc.ctx.cl_module.declare_func_in_func(func_id, builder.func);

    let ret_repr = fc.ctx.tc.repr(target_func.ret, &fc.ctx.module.ty_arena, fc.ctx.module);
    let is_main = fc.ctx.is_main_function(target_func);
    let ret_mode = abi::return_mode(ret_repr, is_main);

    let mut call_args: Vec<Value> = Vec::new();

    let sret_slot = if matches!(ret_mode, ReturnMode::Sret) {
        let slot = mem::alloc_stack_slot(builder, ret_repr.size(), ret_repr.align(), ptr_ty);
        call_args.push(slot);
        Some(slot)
    } else {
        None
    };

    for (i, call_arg) in args.iter().enumerate() {
        if i >= target_func.params.len() {
            break;
        }
        let param = &target_func.params[i];
        let repr = fc.ctx.tc.repr(param.ty, &fc.ctx.module.ty_arena, fc.ctx.module);
        let pass = abi::param_pass_mode(param.convention, repr, ptr_ty);
        let val = fc.get_value(builder, call_arg.value);
        let arg_is_guaranteed = fc.body.values[call_arg.value.index()].ownership
            == kestrel_mir::value::Ownership::Guaranteed;

        match pass {
            PassMode::ByVal(expected_ty) => {
                // @guaranteed scalars are pointers; load the value for by-val passing.
                let val = if arg_is_guaranteed {
                    let arg_ty = fc.body.values[call_arg.value.index()].ty;
                    let arg_repr = fc.ctx.tc.repr(arg_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
                    if let TypeRepr::Scalar(t) = arg_repr {
                        builder.ins().load(t, MemFlags::new(), val, Offset32::new(0))
                    } else {
                        val
                    }
                } else {
                    let actual_ty = builder.func.dfg.value_type(val);
                    if actual_ty != expected_ty && actual_ty == ptr_ty {
                        builder.ins().load(expected_ty, MemFlags::new(), val, Offset32::new(0))
                    } else {
                        val
                    }
                };
                call_args.push(val);
            }
            PassMode::ByRef => {
                if matches!(call_arg.convention, ParamConvention::Borrow | ParamConvention::MutBorrow) {
                    // Borrow/MutBorrow args always go through BeginBorrow
                    // in the lowerer, so the value is already an address.
                    call_args.push(val);
                } else if arg_is_guaranteed {
                    // @guaranteed pointer is already an address — pass directly
                    call_args.push(val);
                } else {
                    // Consuming arg passed by-ref: spill scalar to stack.
                    let actual_ty = builder.func.dfg.value_type(val);
                    if repr.is_scalar() || actual_ty != ptr_ty {
                        let slot = mem::alloc_stack_slot(builder, repr.size(), repr.align(), ptr_ty);
                        mem::store_to_repr(builder, repr, slot, val);
                        call_args.push(slot);
                    } else {
                        call_args.push(val);
                    }
                }
            }
            PassMode::Zst => {}
        }
    }

    let inst = builder.ins().call(func_ref, &call_args);

    if let Some(result_id) = result {
        match ret_mode {
            ReturnMode::Direct(_) => {
                let result_val = builder.inst_results(inst)[0];
                fc.map_value(builder, result_id, result_val);
            }
            ReturnMode::Sret => {
                let slot = sret_slot.expect("sret slot must exist");
                fc.map_value(builder, result_id, slot);
            }
            ReturnMode::Void => {
                let zero = builder.ins().iconst(ptr_ty, 0);
                fc.map_value(builder, result_id, zero);
            }
        }
    }

    if ret_ty_is_never(fc.ctx.module, target_func.ret) {
        builder.ins().trap(TrapCode::unwrap_user(2));
        return Ok(true);
    }

    Ok(false)
}

fn compile_thin_call(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    func_val_id: ValueId,
    args: &[CallArg],
    result: Option<ValueId>,
) -> Result<bool, CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;
    let func_ptr = fc.get_value(builder, func_val_id);

    // Infer types from the ValueId's type (FuncThin or Pointer(FuncThin))
    let func_ty = fc.body.values[func_val_id.index()].ty;
    let arena = &fc.ctx.module.ty_arena;
    let inner_ty = match arena.get(func_ty) {
        MirTy::Pointer(inner) => *inner,
        _ => func_ty,
    };

    let (param_tys, ret_ty) = if let MirTy::FuncThin { params, ret } = arena.get(inner_ty) {
        (params.clone(), *ret)
    } else {
        return Err(CodegenError::Unsupported("thin call on non-FuncThin".into()));
    };

    let ret_repr = fc.ctx.tc.repr(ret_ty, arena, fc.ctx.module);
    let ret_mode = abi::return_mode(ret_repr, false);

    let call_conv = fc.ctx.isa.default_call_conv();
    let mut sig = ir::Signature::new(call_conv);

    if matches!(ret_mode, ReturnMode::Sret) {
        sig.params.push(AbiParam::new(ptr_ty));
    }
    for (ty, convention) in &param_tys {
        let repr = fc.ctx.tc.repr(*ty, arena, fc.ctx.module);
        match abi::param_pass_mode(*convention, repr, ptr_ty) {
            PassMode::ByVal(t) => sig.params.push(AbiParam::new(t)),
            PassMode::ByRef => sig.params.push(AbiParam::new(ptr_ty)),
            PassMode::Zst => {}
        }
    }
    if let ReturnMode::Direct(t) = ret_mode {
        sig.returns.push(AbiParam::new(t));
    }

    let sig_ref = builder.import_signature(sig);

    let mut call_args: Vec<Value> = Vec::new();
    let sret_slot = if matches!(ret_mode, ReturnMode::Sret) {
        let slot = mem::alloc_stack_slot(builder, ret_repr.size(), ret_repr.align(), ptr_ty);
        call_args.push(slot);
        Some(slot)
    } else {
        None
    };

    for (i, call_arg) in args.iter().enumerate() {
        if i >= param_tys.len() {
            break;
        }
        let (ty, convention) = &param_tys[i];
        let repr = fc.ctx.tc.repr(*ty, arena, fc.ctx.module);
        let pass = abi::param_pass_mode(*convention, repr, ptr_ty);
        let val = fc.get_value(builder, call_arg.value);
        let arg_is_guaranteed = fc.body.values[call_arg.value.index()].ownership
            == kestrel_mir::value::Ownership::Guaranteed;
        match pass {
            PassMode::ByVal(expected_ty) => {
                let val = if arg_is_guaranteed {
                    let arg_ty = fc.body.values[call_arg.value.index()].ty;
                    let arg_repr = fc.ctx.tc.repr(arg_ty, arena, fc.ctx.module);
                    if let TypeRepr::Scalar(t) = arg_repr {
                        builder.ins().load(t, MemFlags::new(), val, Offset32::new(0))
                    } else {
                        val
                    }
                } else {
                    val
                };
                call_args.push(val);
            }
            PassMode::ByRef => call_args.push(val),
            PassMode::Zst => {}
        }
    }

    let inst = builder.ins().call_indirect(sig_ref, func_ptr, &call_args);

    if let Some(result_id) = result {
        match ret_mode {
            ReturnMode::Direct(_) => {
                let result_val = builder.inst_results(inst)[0];
                fc.map_value(builder, result_id, result_val);
            }
            ReturnMode::Sret => {
                fc.map_value(builder, result_id, sret_slot.unwrap());
            }
            ReturnMode::Void => {
                let zero = builder.ins().iconst(ptr_ty, 0);
                fc.map_value(builder, result_id, zero);
            }
        }
    }

    if matches!(fc.ctx.module.ty_arena.get(ret_ty), MirTy::Never) {
        builder.ins().trap(TrapCode::unwrap_user(2));
        return Ok(true);
    }

    Ok(false)
}

fn compile_thick_call(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    closure_val_id: ValueId,
    args: &[CallArg],
    result: Option<ValueId>,
) -> Result<bool, CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;
    let ptr_size = fc.ctx.ptr_size;

    let closure_ptr = fc.get_value(builder, closure_val_id);

    let func_ptr = builder.ins().load(ptr_ty, MemFlags::new(), closure_ptr, Offset32::new(0));
    let env_ptr = builder.ins().load(ptr_ty, MemFlags::new(), closure_ptr, Offset32::new(ptr_size as i32));

    let func_ty = fc.body.values[closure_val_id.index()].ty;
    let arena = &fc.ctx.module.ty_arena;
    let inner_ty = match arena.get(func_ty) {
        MirTy::Pointer(inner) => *inner,
        _ => func_ty,
    };

    let (param_tys, ret_ty) = if let MirTy::FuncThick { params, ret } = arena.get(inner_ty) {
        (params.clone(), *ret)
    } else {
        return Err(CodegenError::Unsupported(
            format!("thick call on non-FuncThick: got {:?}", arena.get(func_ty)),
        ));
    };

    let ret_repr = fc.ctx.tc.repr(ret_ty, arena, fc.ctx.module);
    let ret_mode = abi::return_mode(ret_repr, false);

    let call_conv = fc.ctx.isa.default_call_conv();
    let mut sig = ir::Signature::new(call_conv);

    if matches!(ret_mode, ReturnMode::Sret) {
        sig.params.push(AbiParam::new(ptr_ty));
    }
    sig.params.push(AbiParam::new(ptr_ty)); // env_ptr
    for (ty, convention) in &param_tys {
        let repr = fc.ctx.tc.repr(*ty, arena, fc.ctx.module);
        match abi::param_pass_mode(*convention, repr, ptr_ty) {
            PassMode::ByVal(t) => sig.params.push(AbiParam::new(t)),
            PassMode::ByRef => sig.params.push(AbiParam::new(ptr_ty)),
            PassMode::Zst => {}
        }
    }
    if let ReturnMode::Direct(t) = ret_mode {
        sig.returns.push(AbiParam::new(t));
    }

    let sig_ref = builder.import_signature(sig);

    let mut call_args: Vec<Value> = Vec::new();
    let sret_slot = if matches!(ret_mode, ReturnMode::Sret) {
        let slot = mem::alloc_stack_slot(builder, ret_repr.size(), ret_repr.align(), ptr_ty);
        call_args.push(slot);
        Some(slot)
    } else {
        None
    };

    call_args.push(env_ptr);
    for (i, call_arg) in args.iter().enumerate() {
        if i >= param_tys.len() {
            break;
        }
        let (ty, convention) = &param_tys[i];
        let repr = fc.ctx.tc.repr(*ty, arena, fc.ctx.module);
        let pass = abi::param_pass_mode(*convention, repr, ptr_ty);
        let val = fc.get_value(builder, call_arg.value);
        let arg_is_guaranteed = fc.body.values[call_arg.value.index()].ownership
            == kestrel_mir::value::Ownership::Guaranteed;
        match pass {
            PassMode::ByVal(expected_ty) => {
                if arg_is_guaranteed {
                    let arg_ty = fc.body.values[call_arg.value.index()].ty;
                    let arg_repr = fc.ctx.tc.repr(arg_ty, arena, fc.ctx.module);
                    let loaded = if let TypeRepr::Scalar(t) = arg_repr {
                        builder.ins().load(t, MemFlags::new(), val, Offset32::new(0))
                    } else {
                        val
                    };
                    call_args.push(loaded);
                } else if matches!(call_arg.convention, ParamConvention::Borrow | ParamConvention::MutBorrow) {
                    let loaded = builder.ins().load(expected_ty, MemFlags::new(), val, Offset32::new(0));
                    call_args.push(loaded);
                } else {
                    call_args.push(val);
                }
            }
            PassMode::ByRef => call_args.push(val),
            PassMode::Zst => {}
        }
    }

    let inst = builder.ins().call_indirect(sig_ref, func_ptr, &call_args);

    if let Some(result_id) = result {
        match ret_mode {
            ReturnMode::Direct(_) => {
                let result_val = builder.inst_results(inst)[0];
                fc.map_value(builder, result_id, result_val);
            }
            ReturnMode::Sret => {
                fc.map_value(builder, result_id, sret_slot.unwrap());
            }
            ReturnMode::Void => {
                let zero = builder.ins().iconst(ptr_ty, 0);
                fc.map_value(builder, result_id, zero);
            }
        }
    }

    if matches!(fc.ctx.module.ty_arena.get(ret_ty), MirTy::Never) {
        builder.ins().trap(TrapCode::unwrap_user(2));
        return Ok(true);
    }

    Ok(false)
}

// ======================================================================
// Layout helpers
// ======================================================================

fn discriminant_width(
    ty: TyId,
    arena: &TyArena,
    module: &MonoModule,
    tc: &TypeCache,
) -> ir::Type {
    if let MirTy::Named { entity, type_args } = arena.get(ty) {
        let entity = *entity;
        let type_args = type_args.clone();
        if let Some(e) = find_mono_enum(&entity, &type_args, module, tc) {
            return int_bits_to_cl(e.discriminant_width);
        }
    }
    ir::types::I32
}

fn struct_field_offset(
    container_ty: TyId,
    field_idx: FieldIdx,
    arena: &TyArena,
    module: &MonoModule,
    tc: &TypeCache,
) -> u64 {
    if let MirTy::Named { entity, type_args } = arena.get(container_ty) {
        let entity = *entity;
        let type_args = type_args.clone();
        if let Some(s) = find_mono_struct(&entity, &type_args, module, tc) {
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
    tc: &TypeCache,
) -> TyId {
    if let MirTy::Named { entity, type_args } = arena.get(container_ty) {
        let entity = *entity;
        let type_args = type_args.clone();
        if let Some(s) = find_mono_struct(&entity, &type_args, module, tc) {
            return s.fields[field_idx.index()].ty;
        }
    }
    container_ty
}

fn tuple_elem_offset(
    tc: &mut TypeCache,
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
    _tc: &TypeCache,
) -> Option<&'m MonoStruct> {
    module.structs.get(&(*entity, type_args.to_vec()))
}

pub fn find_mono_enum<'m>(
    entity: &Entity,
    type_args: &[TyId],
    module: &'m MonoModule,
    _tc: &TypeCache,
) -> Option<&'m MonoEnum> {
    module.enums.get(&(*entity, type_args.to_vec()))
}
