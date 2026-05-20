//! Rvalue compilation — dispatches to specialized sub-modules.
//!
//! Replaces lib1's 560-line `compile_rvalue` god function with a thin
//! dispatcher that delegates to focused modules.

pub mod arithmetic;
pub mod call;
pub mod cast;
pub mod closure;
pub mod construct;
pub mod immediate;
pub mod pointer;
pub mod string;

use crate::common;
use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::function::FunctionState;
use crate::place;
use cranelift_codegen::ir::immediates::Offset32;
use cranelift_codegen::ir::{InstBuilder, StackSlotData, StackSlotKind, Value as CrValue};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use kestrel_codegen::mangle_function_with_self;
use kestrel_mir::item::CopyBehavior;
use kestrel_mir::passes::place_type;
use kestrel_mir::{MirTy, Op, Place, Rvalue, Value};

/// Compile an Rvalue to a Cranelift value.
pub fn compile_rvalue(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    rvalue: &Rvalue,
) -> Result<CrValue, CodegenError> {
    match rvalue {
        Rvalue::Move(p) | Rvalue::Copy(p) => place::compile_place_read(ctx, state, builder, p),
        Rvalue::Clone(p) => compile_clone(ctx, state, builder, p),
        Rvalue::Ref(p) | Rvalue::RefMut(p) => place::compile_place_addr(ctx, state, builder, p),

        // Constants
        Rvalue::Const(imm) => immediate::compile_immediate(ctx, state, builder, imm),

        // Operations (dispatch by category via the Op enum)
        Rvalue::Op1 { op, arg } => {
            // StackAlloc needs the raw MIR Value to require a compile-time count.
            if let Op::StackAlloc(ty) = op {
                return pointer::compile_stack_alloc(ctx, state, builder, ty, arg);
            }
            let a = compile_value(ctx, state, builder, arg)?;
            dispatch_op1(ctx, state, builder, op, a)
        },
        Rvalue::Op2 { op, lhs, rhs } => {
            let l = compile_value(ctx, state, builder, lhs)?;
            let r = compile_value(ctx, state, builder, rhs)?;
            dispatch_op2(ctx, state, builder, op, l, r)
        },
        Rvalue::Op3 { op, a, b, c } => {
            let va = compile_value(ctx, state, builder, a)?;
            let vb = compile_value(ctx, state, builder, b)?;
            let vc = compile_value(ctx, state, builder, c)?;
            dispatch_op3(ctx, state, builder, op, va, vb, vc)
        },

        // Composite construction
        Rvalue::Construct { ty, fields } => {
            construct::compile_construct(ctx, state, builder, ty, fields)
        },
        Rvalue::Tuple(values) => construct::compile_tuple(ctx, state, builder, values),
        Rvalue::EnumVariant {
            enum_ty,
            variant,
            payload,
        } => construct::compile_enum_variant(ctx, state, builder, enum_ty, variant, payload),
        Rvalue::ArrayLiteral { element_ty, values } => {
            construct::compile_array_literal(ctx, state, builder, element_ty, values)
        },

        // Closures
        Rvalue::ApplyPartial { func, captures } => {
            closure::compile_apply_partial(ctx, state, builder, func, captures)
        },
    }
}

/// Compile a Value (Place or Immediate) to a Cranelift value.
pub fn compile_value(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    value: &Value,
) -> Result<CrValue, CodegenError> {
    match value {
        Value::Copy(p) | Value::Move(p) => place::compile_place_read(ctx, state, builder, p),
        Value::Clone(p) => compile_clone(ctx, state, builder, p),
        Value::Ref(p) | Value::RefMut(p) => place::compile_place_addr(ctx, state, builder, p),
        Value::Const(imm) => immediate::compile_immediate(ctx, state, builder, imm),
    }
}

/// Route Op1 to the appropriate sub-module.
fn dispatch_op1(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    op: &Op,
    arg: CrValue,
) -> Result<CrValue, CodegenError> {
    match op {
        // Pointer ops
        Op::PtrNull(_)
        | Op::PtrFromAddress(_)
        | Op::PtrToAddress
        | Op::PtrIsNull
        | Op::PtrCast(_)
        | Op::PtrBitcast(_)
        | Op::RefToPtr
        | Op::PtrRead(_) => pointer::compile_pointer_op1(ctx, state, builder, op, arg),

        // Memory
        Op::SizeOf(_) | Op::AlignOf(_) | Op::StackAlloc(_) => {
            pointer::compile_memory_op1(ctx, state, builder, op, arg)
        },

        // String ops (StrPtr, StrLen only — IntToString is not emitted by lib)
        Op::StrPtr | Op::StrLen => string::compile_string_op1(ctx, state, builder, op, arg),

        // Float intrinsics
        Op::FloatConst(_, _) | Op::FloatPred(_, _) | Op::FloatMath(_, _) => {
            arithmetic::compile_float_intrinsic_op1(ctx, state, builder, op, arg)
        },

        // Casts
        Op::IntWiden(_, _)
        | Op::IntTruncate(_, _)
        | Op::FloatWiden(_, _)
        | Op::FloatTruncate(_, _)
        | Op::IntToFloat(_, _)
        | Op::FloatToInt(_, _)
        | Op::RefToImmut => cast::compile_cast_op1(ctx, state, builder, op, arg),

        // Arithmetic / bitwise / boolean unary
        _ => arithmetic::compile_op1(ctx, state, builder, op, arg),
    }
}

/// Route Op2 to the appropriate sub-module.
fn dispatch_op2(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    op: &Op,
    lhs: CrValue,
    rhs: CrValue,
) -> Result<CrValue, CodegenError> {
    match op {
        // Pointer ops
        Op::PtrOffset | Op::PtrWrite(_) => {
            pointer::compile_pointer_op2(ctx, state, builder, op, lhs, rhs)
        },

        // Atomic
        Op::AtomicAdd | Op::AtomicSub => {
            pointer::compile_atomic_op2(ctx, state, builder, op, lhs, rhs)
        },

        // Float binary
        Op::FloatCopysign(_) => {
            arithmetic::compile_float_intrinsic_op2(ctx, state, builder, op, lhs, rhs)
        },

        // Arithmetic / bitwise / comparison / boolean binary
        _ => arithmetic::compile_op2(ctx, state, builder, op, lhs, rhs),
    }
}

/// Compile a `Rvalue::Clone` / `Value::Clone` — call the type's clone method
/// to produce a properly refcounted duplicate.
pub fn compile_clone(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    place: &Place,
) -> Result<CrValue, CodegenError> {
    let generic_ty = place_type(ctx.module, state.body, state.func_def, place)
        .ok_or_else(|| CodegenError::Unsupported("clone: cannot resolve place type".into()))?;
    // Substitute TypeParams with concrete types from this instantiation
    let ty = kestrel_codegen::substitute_type_with_self(
        &generic_ty, &state.subst, state.self_type.as_ref(), ctx.module,
    );

    // Look up the clone function. If no clone method exists (e.g., enums
    // with Clone behavior inherited from payloads but no explicit clone
    // impl), fall back to a bitwise read.
    let Some(mangled) = clone_mangled_name(ctx, &ty) else {
        return place::compile_place_read(ctx, state, builder, place);
    };
    let Some(&func_id) = ctx.func_ids_by_name.get(&mangled) else {
        return place::compile_place_read(ctx, state, builder, place);
    };

    let ptr_ty = common::ptr_type(ctx.target);
    let src_addr = place::compile_place_addr(ctx, state, builder, place)?;
    let layout = ctx.layouts.layout_of(&ty);
    let slot = builder.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        layout.size as u32,
        common::align_to_shift(layout.align),
    ));
    let result_addr = builder.ins().stack_addr(ptr_ty, slot, Offset32::new(0));
    common::zero_memory(builder, result_addr, layout.size, ptr_ty);
    let func_ref = ctx.cl_module.declare_func_in_func(func_id, builder.func);
    // clone signature: (sret_ptr, self_ref) -> void
    builder.ins().call(func_ref, &[result_addr, src_addr]);
    Ok(result_addr)
}

/// Find the mangled name of the clone method for a Clone type.
fn clone_mangled_name(ctx: &CodegenContext, ty: &MirTy) -> Option<String> {
    let MirTy::Named { entity, type_args } = ty else {
        return None;
    };
    // Check structs
    let clone_parent = ctx.module.structs.iter()
        .find(|s| s.entity == *entity && matches!(s.copy_behavior, CopyBehavior::Clone(_)))
        .map(|s| s.entity)
        // Check enums
        .or_else(|| ctx.module.enums.iter()
            .find(|e| e.entity == *entity && matches!(e.copy_behavior, CopyBehavior::Clone(_)))
            .map(|e| e.entity));
    let parent = clone_parent?;
    let clone_func = ctx.module.functions.iter().find(|f| {
        matches!(&f.kind, kestrel_mir::FunctionKind::Method { parent: p, .. } if *p == parent)
            && f.name.ends_with(".clone")
    })?;
    Some(kestrel_codegen::mangle_function_with_self(ctx.module, clone_func, type_args, Some(ty)))
}

/// Route Op3 to the appropriate sub-module.
fn dispatch_op3(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    op: &Op,
    a: CrValue,
    b: CrValue,
    c: CrValue,
) -> Result<CrValue, CodegenError> {
    match op {
        Op::FloatFma(_) => arithmetic::compile_float_fma(ctx, state, builder, op, a, b, c),
        _ => Err(CodegenError::Unsupported(format!("unknown Op3: {op:?}"))),
    }
}
