use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{self, InstBuilder, MemFlags, Value};
use cranelift_frontend::FunctionBuilder;

use cranelift_module::Module;
use kestrel_mir::{FloatBits, ImmediateKind, MonoFuncId};

use crate::context::CodegenCtx;
use crate::error::CodegenError;
use crate::mem;
use crate::ty::{float_bits_to_cl, int_bits_to_cl};

pub fn compile_immediate(
    ctx: &mut CodegenCtx<'_>,
    builder: &mut FunctionBuilder,
    kind: &ImmediateKind,
) -> Result<Value, CodegenError> {
    let ptr_ty = ctx.ptr_ty;

    match kind {
        ImmediateKind::IntLiteral { bits, value } => {
            let cl_ty = int_bits_to_cl(*bits);
            Ok(builder.ins().iconst(cl_ty, *value as i64))
        }

        ImmediateKind::FloatLiteral { bits, value } => match bits {
            FloatBits::F16 => {
                let f32_val = builder.ins().f32const(*value as f32);
                Ok(builder.ins().fdemote(ir::types::F16, f32_val))
            }
            FloatBits::F32 => Ok(builder.ins().f32const(*value as f32)),
            FloatBits::F64 => Ok(builder.ins().f64const(*value)),
        },

        ImmediateKind::BoolLiteral(b) => Ok(builder.ins().iconst(ir::types::I8, *b as i64)),

        ImmediateKind::StringLiteral(s) => compile_string_literal(ctx, builder, s),

        ImmediateKind::StringPointer(s) => compile_string_pointer(ctx, builder, s),

        ImmediateKind::Unit => Ok(builder.ins().iconst(ptr_ty, 0)),

        ImmediateKind::MonoFunctionRef(mono_id) => {
            compile_mono_func_ref(ctx, builder, *mono_id)
        }

        ImmediateKind::FunctionRef { .. } => {
            debug_assert!(false, "unresolved FunctionRef in codegen");
            Ok(builder.ins().iconst(ptr_ty, 0))
        }

        ImmediateKind::NullPtr(_) => Ok(builder.ins().iconst(ptr_ty, 0)),

        ImmediateKind::SizeOf(ty) => {
            let repr = ctx.tc.repr(*ty, &ctx.module.ty_arena, ctx.module);
            Ok(builder.ins().iconst(ptr_ty, repr.size() as i64))
        }

        ImmediateKind::AlignOf(ty) => {
            let repr = ctx.tc.repr(*ty, &ctx.module.ty_arena, ctx.module);
            Ok(builder.ins().iconst(ptr_ty, repr.align() as i64))
        }

        ImmediateKind::FloatInfinity(bits) => {
            let cl_ty = float_bits_to_cl(*bits);
            match bits {
                FloatBits::F16 => {
                    let inf = builder.ins().f32const(f32::INFINITY);
                    Ok(builder.ins().fdemote(cl_ty, inf))
                }
                FloatBits::F32 => Ok(builder.ins().f32const(f32::INFINITY)),
                FloatBits::F64 => Ok(builder.ins().f64const(f64::INFINITY)),
            }
        }

        ImmediateKind::FloatNan(bits) => {
            let cl_ty = float_bits_to_cl(*bits);
            match bits {
                FloatBits::F16 => {
                    let nan = builder.ins().f32const(f32::NAN);
                    Ok(builder.ins().fdemote(cl_ty, nan))
                }
                FloatBits::F32 => Ok(builder.ins().f32const(f32::NAN)),
                FloatBits::F64 => Ok(builder.ins().f64const(f64::NAN)),
            }
        }

        ImmediateKind::Error => Ok(builder.ins().iconst(ir::types::I8, 0)),
    }
}

fn compile_string_literal(
    ctx: &mut CodegenCtx<'_>,
    builder: &mut FunctionBuilder,
    s: &str,
) -> Result<Value, CodegenError> {
    let ptr_ty = ctx.ptr_ty;
    let ptr_size = ctx.ptr_size;
    let data_id = ctx.get_or_create_string_data(builder.func, s)?;

    let gv = ctx.cl_module.declare_data_in_func(data_id, builder.func);
    let data_ptr = builder.ins().global_value(ptr_ty, gv);
    let len = builder.ins().iconst(ptr_ty, s.len() as i64);

    let slot = mem::alloc_stack_slot(builder, ptr_size * 2, ptr_size, ptr_ty);
    builder
        .ins()
        .store(MemFlags::new(), data_ptr, slot, Offset32::new(0));
    builder
        .ins()
        .store(MemFlags::new(), len, slot, Offset32::new(ptr_size as i32));

    Ok(slot)
}

fn compile_string_pointer(
    ctx: &mut CodegenCtx<'_>,
    builder: &mut FunctionBuilder,
    s: &str,
) -> Result<Value, CodegenError> {
    let ptr_ty = ctx.ptr_ty;
    let data_id = ctx.get_or_create_string_data(builder.func, s)?;
    let gv = ctx.cl_module.declare_data_in_func(data_id, builder.func);
    Ok(builder.ins().global_value(ptr_ty, gv))
}

fn compile_mono_func_ref(
    ctx: &mut CodegenCtx<'_>,
    builder: &mut FunctionBuilder,
    mono_id: MonoFuncId,
) -> Result<Value, CodegenError> {
    let ptr_ty = ctx.ptr_ty;
    let func_id = ctx.func_ids[mono_id.index()].ok_or_else(|| {
        CodegenError::Unsupported(format!(
            "mono function ref {} not declared",
            mono_id.index()
        ))
    })?;
    let func_ref = ctx.cl_module.declare_func_in_func(func_id, builder.func);
    Ok(builder.ins().func_addr(ptr_ty, func_ref))
}
