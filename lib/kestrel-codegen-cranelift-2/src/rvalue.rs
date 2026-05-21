use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, Value};
use cranelift_frontend::FunctionBuilder;

use cranelift_module::Module;
use kestrel_mir_2::{
    FieldIdx, FloatMathKind, FloatPredicateKind, ImmediateKind, MirTy, Op, Operand, Rvalue,
    Signedness,
};

use crate::error::CodegenError;
use crate::func::FuncCompiler;
use crate::ty::{float_bits_to_cl, int_bits_to_cl, TypeRepr};
use crate::{imm, mem, place};

pub fn compile_rvalue(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    rvalue: &Rvalue,
) -> Result<Value, CodegenError> {
    match rvalue {
        Rvalue::Use(op, _mode) => compile_operand(fc, builder, op),

        Rvalue::Ref(p) | Rvalue::RefMut(p) => place::place_addr(fc, builder, p),

        Rvalue::Op1 { op, arg } => {
            let a = compile_operand(fc, builder, arg)?;
            compile_op1(fc, builder, *op, a)
        }
        Rvalue::Op2 { op, lhs, rhs } => {
            let l = compile_operand(fc, builder, lhs)?;
            let r = compile_operand(fc, builder, rhs)?;
            compile_op2(fc, builder, *op, l, r)
        }
        Rvalue::Op3 { op, a, b, c } => {
            let va = compile_operand(fc, builder, a)?;
            let vb = compile_operand(fc, builder, b)?;
            let vc = compile_operand(fc, builder, c)?;
            compile_op3(builder, *op, va, vb, vc)
        }

        Rvalue::Construct { ty, fields } => compile_construct(fc, builder, *ty, fields),
        Rvalue::Tuple(elems) => compile_tuple(fc, builder, elems),
        Rvalue::EnumVariant {
            enum_ty,
            variant,
            payload,
        } => compile_enum_variant(fc, builder, *enum_ty, *variant, payload),
        Rvalue::ArrayLiteral {
            element_ty,
            values,
        } => compile_array_literal(fc, builder, *element_ty, values),
        Rvalue::ApplyPartial { func, captures } => {
            compile_apply_partial(fc, builder, *func, captures)
        }
    }
}

pub fn compile_operand(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    op: &Operand,
) -> Result<Value, CodegenError> {
    match op {
        Operand::Place(p) => place::place_read(fc, builder, p),
        Operand::Const(imm_val) => imm::compile_immediate(fc.ctx, builder, &imm_val.kind),
    }
}

// -- Operations --

fn compile_op1(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    op: Op,
    arg: Value,
) -> Result<Value, CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;

    Ok(match op {
        // Arithmetic
        Op::Neg(bits) => {
            let zero = builder.ins().iconst(int_bits_to_cl(bits), 0);
            builder.ins().isub(zero, arg)
        }
        Op::FNeg(_) => builder.ins().fneg(arg),

        // Bitwise
        Op::Not(_) => builder.ins().bnot(arg),
        Op::Popcount(_) => builder.ins().popcnt(arg),
        Op::Clz(_) => builder.ins().clz(arg),
        Op::Ctz(_) => builder.ins().ctz(arg),
        Op::Bswap(_) => builder.ins().bswap(arg),
        Op::BoolNot => {
            let one = builder.ins().iconst(ir::types::I8, 1);
            builder.ins().bxor(arg, one)
        }

        // Casts
        Op::IntWiden(_, to) => builder.ins().sextend(int_bits_to_cl(to), arg),
        Op::IntUnsignedWiden(_, to) => builder.ins().uextend(int_bits_to_cl(to), arg),
        Op::IntTruncate(_, to) => builder.ins().ireduce(int_bits_to_cl(to), arg),
        Op::IntToFloat(_, fb) => builder.ins().fcvt_from_sint(float_bits_to_cl(fb), arg),
        Op::FloatToInt(_, ib) => builder.ins().fcvt_to_sint_sat(int_bits_to_cl(ib), arg),
        Op::FloatWiden(_, to) => builder.ins().fpromote(float_bits_to_cl(to), arg),
        Op::FloatTruncate(_, to) => builder.ins().fdemote(float_bits_to_cl(to), arg),
        Op::RefToImmut => arg,

        // Pointer
        Op::PtrFromAddress(_) => arg,
        Op::PtrToAddress => arg,
        Op::PtrIsNull => builder.ins().icmp_imm(IntCC::Equal, arg, 0),
        Op::PtrCast(_) | Op::PtrBitcast(_) => arg,
        Op::RefToPtr => arg,

        Op::PtrRead(ty) => {
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            mem::load_from_repr(builder, repr, arg, ptr_ty)
        }

        Op::StackAlloc(ty) => {
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            mem::alloc_stack_slot(builder, repr.size(), repr.align(), ptr_ty)
        }

        // String
        Op::StrPtr => builder
            .ins()
            .load(ptr_ty, MemFlags::new(), arg, Offset32::new(0)),
        Op::StrLen => {
            let ptr_size = fc.ctx.ptr_size;
            builder
                .ins()
                .load(ptr_ty, MemFlags::new(), arg, Offset32::new(ptr_size as i32))
        }

        // Float intrinsics
        Op::FloatPred(_, FloatPredicateKind::IsNan) => {
            // NaN != NaN
            let cmp = builder.ins().fcmp(FloatCC::Unordered, arg, arg);
            builder.ins().uextend(ir::types::I8, cmp)
        }
        Op::FloatPred(_, FloatPredicateKind::IsInfinite) => {
            let abs = builder.ins().fabs(arg);
            let inf = builder.ins().f64const(f64::INFINITY);
            let is_inf = builder.ins().fcmp(FloatCC::Equal, abs, inf);
            builder.ins().uextend(ir::types::I8, is_inf)
        }
        Op::FloatMath(_, FloatMathKind::Floor) => builder.ins().floor(arg),
        Op::FloatMath(_, FloatMathKind::Ceil) => builder.ins().ceil(arg),
        Op::FloatMath(_, FloatMathKind::Round) => builder.ins().nearest(arg),
        Op::FloatMath(_, FloatMathKind::Trunc) => builder.ins().trunc(arg),
        Op::FloatMath(_, FloatMathKind::Sqrt) => builder.ins().sqrt(arg),

        _ => {
            return Err(CodegenError::Unsupported(format!(
                "op1 variant: {op:?}"
            )));
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
        // Integer arithmetic
        Op::Add(_, _) => builder.ins().iadd(lhs, rhs),
        Op::Sub(_, _) => builder.ins().isub(lhs, rhs),
        Op::Mul(_, _) => builder.ins().imul(lhs, rhs),
        Op::Div(_, Signedness::Signed) => builder.ins().sdiv(lhs, rhs),
        Op::Div(_, Signedness::Unsigned) => builder.ins().udiv(lhs, rhs),
        Op::Rem(_, Signedness::Signed) => builder.ins().srem(lhs, rhs),
        Op::Rem(_, Signedness::Unsigned) => builder.ins().urem(lhs, rhs),

        // Float arithmetic
        Op::FAdd(_) => builder.ins().fadd(lhs, rhs),
        Op::FSub(_) => builder.ins().fsub(lhs, rhs),
        Op::FMul(_) => builder.ins().fmul(lhs, rhs),
        Op::FDiv(_) => builder.ins().fdiv(lhs, rhs),

        // Bitwise
        Op::And(_) => builder.ins().band(lhs, rhs),
        Op::Or(_) => builder.ins().bor(lhs, rhs),
        Op::Xor(_) => builder.ins().bxor(lhs, rhs),
        Op::Shl(_) => builder.ins().ishl(lhs, rhs),
        Op::Shr(_, Signedness::Signed) => builder.ins().sshr(lhs, rhs),
        Op::Shr(_, Signedness::Unsigned) => builder.ins().ushr(lhs, rhs),

        // Integer comparison
        Op::Eq(_) => {
            let cmp = builder.ins().icmp(IntCC::Equal, lhs, rhs);
            builder.ins().uextend(ir::types::I8, cmp)
        }
        Op::Ne(_) => {
            let cmp = builder.ins().icmp(IntCC::NotEqual, lhs, rhs);
            builder.ins().uextend(ir::types::I8, cmp)
        }
        Op::Lt(_, Signedness::Signed) => {
            let cmp = builder.ins().icmp(IntCC::SignedLessThan, lhs, rhs);
            builder.ins().uextend(ir::types::I8, cmp)
        }
        Op::Lt(_, Signedness::Unsigned) => {
            let cmp = builder.ins().icmp(IntCC::UnsignedLessThan, lhs, rhs);
            builder.ins().uextend(ir::types::I8, cmp)
        }
        Op::Le(_, Signedness::Signed) => {
            let cmp = builder
                .ins()
                .icmp(IntCC::SignedLessThanOrEqual, lhs, rhs);
            builder.ins().uextend(ir::types::I8, cmp)
        }
        Op::Le(_, Signedness::Unsigned) => {
            let cmp = builder
                .ins()
                .icmp(IntCC::UnsignedLessThanOrEqual, lhs, rhs);
            builder.ins().uextend(ir::types::I8, cmp)
        }
        Op::Gt(_, Signedness::Signed) => {
            let cmp = builder.ins().icmp(IntCC::SignedGreaterThan, lhs, rhs);
            builder.ins().uextend(ir::types::I8, cmp)
        }
        Op::Gt(_, Signedness::Unsigned) => {
            let cmp = builder
                .ins()
                .icmp(IntCC::UnsignedGreaterThan, lhs, rhs);
            builder.ins().uextend(ir::types::I8, cmp)
        }
        Op::Ge(_, Signedness::Signed) => {
            let cmp = builder
                .ins()
                .icmp(IntCC::SignedGreaterThanOrEqual, lhs, rhs);
            builder.ins().uextend(ir::types::I8, cmp)
        }
        Op::Ge(_, Signedness::Unsigned) => {
            let cmp = builder
                .ins()
                .icmp(IntCC::UnsignedGreaterThanOrEqual, lhs, rhs);
            builder.ins().uextend(ir::types::I8, cmp)
        }

        // Float comparison
        Op::FEq(_) => {
            let cmp = builder.ins().fcmp(FloatCC::Equal, lhs, rhs);
            builder.ins().uextend(ir::types::I8, cmp)
        }
        Op::FNe(_) => {
            let cmp = builder.ins().fcmp(FloatCC::NotEqual, lhs, rhs);
            builder.ins().uextend(ir::types::I8, cmp)
        }
        Op::FLt(_) => {
            let cmp = builder.ins().fcmp(FloatCC::LessThan, lhs, rhs);
            builder.ins().uextend(ir::types::I8, cmp)
        }
        Op::FLe(_) => {
            let cmp = builder.ins().fcmp(FloatCC::LessThanOrEqual, lhs, rhs);
            builder.ins().uextend(ir::types::I8, cmp)
        }
        Op::FGt(_) => {
            let cmp = builder.ins().fcmp(FloatCC::GreaterThan, lhs, rhs);
            builder.ins().uextend(ir::types::I8, cmp)
        }
        Op::FGe(_) => {
            let cmp = builder
                .ins()
                .fcmp(FloatCC::GreaterThanOrEqual, lhs, rhs);
            builder.ins().uextend(ir::types::I8, cmp)
        }

        // Boolean
        Op::BoolAnd => builder.ins().band(lhs, rhs),
        Op::BoolOr => builder.ins().bor(lhs, rhs),
        Op::BoolEq => {
            let cmp = builder.ins().icmp(IntCC::Equal, lhs, rhs);
            builder.ins().uextend(ir::types::I8, cmp)
        }

        // Pointer
        Op::PtrOffset => builder.ins().iadd(lhs, rhs),
        Op::PtrWrite(ty) => {
            let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            mem::store_to_repr(builder, repr, lhs, rhs);
            builder.ins().iconst(ptr_ty, 0)
        }

        // Atomic
        Op::AtomicAdd => {
            builder
                .ins()
                .atomic_rmw(ir::types::I64, MemFlags::new(), ir::AtomicRmwOp::Add, lhs, rhs)
        }
        Op::AtomicSub => {
            builder
                .ins()
                .atomic_rmw(ir::types::I64, MemFlags::new(), ir::AtomicRmwOp::Sub, lhs, rhs)
        }

        // Float
        Op::FloatCopysign(_) => builder.ins().fcopysign(lhs, rhs),

        _ => {
            return Err(CodegenError::Unsupported(format!(
                "op2 variant: {op:?}"
            )));
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
            return Err(CodegenError::Unsupported(format!(
                "op3 variant: {op:?}"
            )));
        }
    })
}

// -- Composite construction --

fn compile_construct(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    ty: kestrel_mir_2::TyId,
    fields: &[(FieldIdx, Operand, kestrel_mir_2::UseMode)],
) -> Result<Value, CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;
    let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);

    match repr {
        TypeRepr::Zst => return Ok(builder.ins().iconst(ptr_ty, 0)),
        TypeRepr::Scalar(t) => {
            // Single-field scalar struct (like Bool, Char)
            if fields.len() == 1 {
                let val = compile_operand(fc, builder, &fields[0].1)?;
                return Ok(val);
            }
            // Multi-field struct that fits in a register: pack into slot, load as scalar
            let slot = mem::alloc_stack_slot(builder, repr.size(), repr.align(), ptr_ty);
            mem::zero_memory(builder, slot, repr.size());
            for (field_idx, operand, _) in fields {
                let val = compile_operand(fc, builder, operand)?;
                let offset = struct_field_offset_for_ty(fc, ty, *field_idx);
                let field_ty = struct_field_type_for_ty(fc, ty, *field_idx);
                let field_repr = fc.ctx.tc.repr(field_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
                let dest = builder.ins().iadd_imm(slot, offset as i64);
                mem::store_to_repr(builder, field_repr, dest, val);
            }
            return Ok(builder
                .ins()
                .load(t, MemFlags::new(), slot, Offset32::new(0)));
        }
        TypeRepr::Aggregate { size, align } => {
            let slot = mem::alloc_stack_slot(builder, size, align, ptr_ty);
            mem::zero_memory(builder, slot, size);
            for (field_idx, operand, _) in fields {
                let val = compile_operand(fc, builder, operand)?;
                let offset = struct_field_offset_for_ty(fc, ty, *field_idx);
                let field_ty = struct_field_type_for_ty(fc, ty, *field_idx);
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
    elems: &[(Operand, kestrel_mir_2::UseMode)],
) -> Result<Value, CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;

    if elems.is_empty() {
        return Ok(builder.ins().iconst(ptr_ty, 0));
    }

    // Compute tuple layout
    let mut layout = kestrel_mir_2::StructLayout::new();
    let mut elem_tys = Vec::with_capacity(elems.len());
    for (operand, _) in elems {
        let ty = operand_type(fc, operand);
        let repr = fc.ctx.tc.repr(ty, &fc.ctx.module.ty_arena, fc.ctx.module);
        layout.append_field(kestrel_mir_2::StructLayout::scalar(repr.size(), repr.align()));
        elem_tys.push(ty);
    }
    layout.pad_to_align();

    let slot = mem::alloc_stack_slot(builder, layout.size, layout.align, ptr_ty);
    mem::zero_memory(builder, slot, layout.size);

    for (i, (operand, _)) in elems.iter().enumerate() {
        let val = compile_operand(fc, builder, operand)?;
        let offset = layout.field_offsets[i];
        let repr = fc
            .ctx
            .tc
            .repr(elem_tys[i], &fc.ctx.module.ty_arena, fc.ctx.module);
        let dest = if offset != 0 {
            builder.ins().iadd_imm(slot, offset as i64)
        } else {
            slot
        };
        mem::store_to_repr(builder, repr, dest, val);
    }

    Ok(slot)
}

fn compile_enum_variant(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    enum_ty: kestrel_mir_2::TyId,
    variant: kestrel_mir_2::VariantIdx,
    payload: &[(Operand, kestrel_mir_2::UseMode)],
) -> Result<Value, CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;
    let repr = fc
        .ctx
        .tc
        .repr(enum_ty, &fc.ctx.module.ty_arena, fc.ctx.module);

    // Find enum metadata
    let arena = &fc.ctx.module.ty_arena;
    let (disc_width, payload_offset, disc_value) =
        if let MirTy::Named { entity, type_args } = arena.get(enum_ty) {
            let entity = *entity;
            let type_args = type_args.clone();
            if let Some(e) = place::find_mono_enum(&entity, &type_args, fc.ctx.module, &fc.ctx.tc) {
                let disc = e.cases[variant.index()].discriminant;
                (
                    crate::ty::int_bits_to_cl(e.discriminant_width),
                    e.payload_offset as u64,
                    disc as i64,
                )
            } else {
                (ir::types::I8, 0u64, variant.index() as i64)
            }
        } else {
            (ir::types::I8, 0u64, variant.index() as i64)
        };

    match repr {
        TypeRepr::Scalar(t) => {
            // Small enum that fits in a register — just store discriminant (and maybe payload)
            let slot = mem::alloc_stack_slot(builder, repr.size(), repr.align(), ptr_ty);
            mem::zero_memory(builder, slot, repr.size());
            let disc = builder.ins().iconst(disc_width, disc_value);
            builder
                .ins()
                .store(MemFlags::new(), disc, slot, Offset32::new(0));

            // Store payload fields at payload_offset + variant field offsets
            store_variant_payload(fc, builder, enum_ty, variant, payload, slot, payload_offset)?;

            Ok(builder
                .ins()
                .load(t, MemFlags::new(), slot, Offset32::new(0)))
        }
        TypeRepr::Aggregate { size, align } => {
            let slot = mem::alloc_stack_slot(builder, size, align, ptr_ty);
            mem::zero_memory(builder, slot, size);

            let disc = builder.ins().iconst(disc_width, disc_value);
            builder
                .ins()
                .store(MemFlags::new(), disc, slot, Offset32::new(0));

            store_variant_payload(fc, builder, enum_ty, variant, payload, slot, payload_offset)?;

            Ok(slot)
        }
        TypeRepr::Zst => Ok(builder.ins().iconst(ptr_ty, 0)),
    }
}

fn store_variant_payload(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    enum_ty: kestrel_mir_2::TyId,
    variant: kestrel_mir_2::VariantIdx,
    payload: &[(Operand, kestrel_mir_2::UseMode)],
    slot: Value,
    payload_offset: u64,
) -> Result<(), CodegenError> {
    let arena = &fc.ctx.module.ty_arena;

    if let MirTy::Named { entity, type_args } = arena.get(enum_ty) {
        let entity = *entity;
        let type_args = type_args.clone();
        if let Some(e) = place::find_mono_enum(&entity, &type_args, fc.ctx.module, &fc.ctx.tc) {
            if let Some(kestrel_mir_2::Layout::Enum(el)) = &e.type_info.layout {
                if let Some(vl) = el.variant_layouts.get(variant.index()) {
                    for (i, (operand, _)) in payload.iter().enumerate() {
                        let val = compile_operand(fc, builder, operand)?;
                        let field_offset = vl.field_offsets.get(i).copied().unwrap_or(0);
                        let total_offset = payload_offset + field_offset;
                        let field_ty = e.cases[variant.index()].payload_fields[i].ty;
                        let field_repr =
                            fc.ctx
                                .tc
                                .repr(field_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
                        let dest = builder.ins().iadd_imm(slot, total_offset as i64);
                        mem::store_to_repr(builder, field_repr, dest, val);
                    }
                }
            }
        }
    }
    Ok(())
}

fn compile_array_literal(
    fc: &mut FuncCompiler<'_, '_>,
    builder: &mut FunctionBuilder,
    element_ty: kestrel_mir_2::TyId,
    values: &[(Operand, kestrel_mir_2::UseMode)],
) -> Result<Value, CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;
    let elem_repr = fc
        .ctx
        .tc
        .repr(element_ty, &fc.ctx.module.ty_arena, fc.ctx.module);
    let elem_size = elem_repr.size();
    let total_size = elem_size * values.len() as u64;

    if total_size == 0 {
        return Ok(builder.ins().iconst(ptr_ty, 0));
    }

    let slot = mem::alloc_stack_slot(builder, total_size, elem_repr.align(), ptr_ty);
    mem::zero_memory(builder, slot, total_size);

    for (i, (operand, _)) in values.iter().enumerate() {
        let val = compile_operand(fc, builder, operand)?;
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
    func_entity: kestrel_hecs::Entity,
    captures: &[(Operand, kestrel_mir_2::UseMode)],
) -> Result<Value, CodegenError> {
    let ptr_ty = fc.ctx.ptr_ty;
    let ptr_size = fc.ctx.ptr_size;

    // Build thick closure: {func_ptr, env_ptr}
    // For now, find the MonoFuncId that matches this entity
    let func_addr = {
        let mut found = None;
        for (i, f) in fc.ctx.module.functions.iter().enumerate() {
            if f.source == func_entity {
                let func_id = fc.ctx.func_ids[i].ok_or_else(|| {
                    CodegenError::Unsupported("closure target not declared".into())
                })?;
                let func_ref = fc.ctx.cl_module.declare_func_in_func(func_id, builder.func);
                found = Some(builder.ins().func_addr(ptr_ty, func_ref));
                break;
            }
        }
        found.unwrap_or_else(|| builder.ins().iconst(ptr_ty, 0))
    };

    // Allocate env struct on stack (captures laid out sequentially)
    let mut env_size = 0u64;
    let mut env_align = 1u64;
    let mut capture_reprs = Vec::new();
    for (operand, _) in captures {
        let ty = operand_type(fc, operand);
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
        for (i, (operand, _)) in captures.iter().enumerate() {
            let val = compile_operand(fc, builder, operand)?;
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

    // Build thick function {func_ptr, env_ptr}
    let thick = mem::alloc_stack_slot(builder, ptr_size * 2, ptr_size, ptr_ty);
    builder
        .ins()
        .store(MemFlags::new(), func_addr, thick, Offset32::new(0));
    builder
        .ins()
        .store(MemFlags::new(), env_ptr, thick, Offset32::new(ptr_size as i32));

    Ok(thick)
}

// -- Helpers --

fn struct_field_offset_for_ty(
    fc: &FuncCompiler<'_, '_>,
    ty: kestrel_mir_2::TyId,
    field_idx: FieldIdx,
) -> u64 {
    let arena = &fc.ctx.module.ty_arena;
    if let MirTy::Named { entity, type_args } = arena.get(ty) {
        let entity = *entity;
        let type_args = type_args.clone();
        if let Some(s) = place::find_mono_struct(&entity, &type_args, fc.ctx.module, &fc.ctx.tc) {
            if let Some(kestrel_mir_2::Layout::Struct(sl)) = &s.type_info.layout {
                return sl.field_offsets[field_idx.index()];
            }
        }
    }
    0
}

fn struct_field_type_for_ty(
    fc: &FuncCompiler<'_, '_>,
    ty: kestrel_mir_2::TyId,
    field_idx: FieldIdx,
) -> kestrel_mir_2::TyId {
    let arena = &fc.ctx.module.ty_arena;
    if let MirTy::Named { entity, type_args } = arena.get(ty) {
        let entity = *entity;
        let type_args = type_args.clone();
        if let Some(s) = place::find_mono_struct(&entity, &type_args, fc.ctx.module, &fc.ctx.tc) {
            return s.fields[field_idx.index()].ty;
        }
    }
    ty
}

fn operand_type(fc: &FuncCompiler<'_, '_>, op: &Operand) -> kestrel_mir_2::TyId {
    match op {
        Operand::Place(p) => {
            place::place_type(p, fc.body, &fc.ctx.module.ty_arena, fc.ctx.module, &fc.ctx.tc)
        }
        Operand::Const(imm) => {
            // For immediates, infer from the kind
            let arena = &fc.ctx.module.ty_arena;
            match &imm.kind {
                ImmediateKind::IntLiteral { bits, .. } => arena
                    .find(|t| match t {
                        MirTy::I8 if matches!(bits, kestrel_mir_2::IntBits::I8) => true,
                        MirTy::I16 if matches!(bits, kestrel_mir_2::IntBits::I16) => true,
                        MirTy::I32 if matches!(bits, kestrel_mir_2::IntBits::I32) => true,
                        MirTy::I64 if matches!(bits, kestrel_mir_2::IntBits::I64) => true,
                        _ => false,
                    })
                    .unwrap_or(kestrel_mir_2::TyId::new(0)),
                ImmediateKind::BoolLiteral(_) => arena
                    .find(|t| matches!(t, MirTy::Bool))
                    .unwrap_or(kestrel_mir_2::TyId::new(0)),
                _ => kestrel_mir_2::TyId::new(0),
            }
        }
    }
}
