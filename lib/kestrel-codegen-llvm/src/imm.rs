//! Immediate / constant lowering. String literals build a `{ ptr, i64 len }`
//! aggregate in a stack slot and return its address (so `Str` is carried by
//! pointer, like every aggregate). Addresses are real LLVM `ptr` values.

use inkwell::builder::Builder;
use inkwell::values::BasicValueEnum;

use kestrel_mir::{ImmediateKind, MonoFuncId};

use crate::error::CodegenError;
use crate::func::FuncCompiler;
use crate::mem;
use crate::ty::{ScalarTy, TypeRepr, float_bits_to_scalar, int_bits_to_scalar};

pub fn compile_immediate<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    kind: &ImmediateKind,
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;

    match kind {
        ImmediateKind::IntLiteral { bits, value } => {
            let int_ty = int_bits_to_scalar(*bits).llvm(cx).into_int_type();
            Ok(int_ty.const_int(*value as i64 as u64, false).into())
        },

        ImmediateKind::FloatLiteral { bits, value } => {
            let float_ty = float_bits_to_scalar(*bits).llvm(cx).into_float_type();
            Ok(float_ty.const_float(*value).into())
        },

        ImmediateKind::BoolLiteral(b) => Ok(cx.i8_type().const_int(*b as u64, false).into()),

        ImmediateKind::StringLiteral(s) => compile_string_literal(fc, builder, s),

        ImmediateKind::StringPointer(s) => {
            let global = fc.ctx.get_or_create_string_data(s)?;
            Ok(global.as_pointer_value().into())
        },

        ImmediateKind::Unit => Ok(mem::null_ptr(cx).into()),

        ImmediateKind::MonoFunctionRef(mono_id) => compile_mono_func_ref(fc, builder, *mono_id),

        ImmediateKind::FunctionRef { .. } => {
            debug_assert!(false, "unresolved FunctionRef in codegen");
            Ok(mem::null_ptr(cx).into())
        },

        ImmediateKind::NullPtr(_) => Ok(mem::null_ptr(cx).into()),

        ImmediateKind::SizeOf(ty) => {
            let repr = fc.ctx.tc.repr(*ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            Ok(mem::usize_const(cx, ptr_size, repr.size() as i64).into())
        },

        ImmediateKind::AlignOf(ty) => {
            let repr = fc.ctx.tc.repr(*ty, &fc.ctx.module.ty_arena, fc.ctx.module);
            Ok(mem::usize_const(cx, ptr_size, repr.align() as i64).into())
        },

        ImmediateKind::FloatInfinity(bits) => {
            let float_ty = float_bits_to_scalar(*bits).llvm(cx).into_float_type();
            Ok(float_ty.const_float(f64::INFINITY).into())
        },

        ImmediateKind::FloatNan(bits) => {
            let float_ty = float_bits_to_scalar(*bits).llvm(cx).into_float_type();
            Ok(float_ty.const_float(f64::NAN).into())
        },

        ImmediateKind::Error => Ok(cx.i8_type().const_int(0, false).into()),
    }
}

fn compile_string_literal<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    builder: &Builder<'ctx>,
    s: &str,
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    let cx = fc.ctx.cx;
    let ptr_size = fc.ctx.ptr_size;
    let ptr_scalar = fc.ctx.tc.ptr_scalar;

    let global = fc.ctx.get_or_create_string_data(s)?;
    let data_ptr = global.as_pointer_value();
    let len = mem::usize_const(cx, ptr_size, s.len() as i64);

    // `{ ptr@0, i64 len@ptr_size }`: data is a real `ptr`, the length an integer.
    let slot = fc.alloca(ptr_size * 2, ptr_size);
    mem::store_to_repr(
        cx,
        builder,
        ptr_size,
        TypeRepr::Scalar(ptr_scalar),
        slot,
        data_ptr.into(),
    );
    let len_dest = mem::field_gep(cx, builder, slot, ptr_size);
    mem::store_to_repr(
        cx,
        builder,
        ptr_size,
        TypeRepr::Scalar(ScalarTy::I64),
        len_dest,
        len.into(),
    );

    Ok(slot.into())
}

fn compile_mono_func_ref<'ctx>(
    fc: &mut FuncCompiler<'_, 'ctx>,
    _builder: &Builder<'ctx>,
    mono_id: MonoFuncId,
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    let func = fc.ctx.func_ids[mono_id.index()].ok_or_else(|| {
        CodegenError::Unsupported(format!(
            "mono function ref {} not declared",
            mono_id.index()
        ))
    })?;
    Ok(func.as_global_value().as_pointer_value().into())
}
