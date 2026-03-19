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

use crate::common::{self, is_aggregate_type};
use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::function::FunctionState;
use crate::place;
use cranelift_codegen::ir::Value as CrValue;
use cranelift_frontend::FunctionBuilder;
use kestrel_mir::{Op, Rvalue, Value};

/// Compile an Rvalue to a Cranelift value.
pub fn compile_rvalue(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    rvalue: &Rvalue,
) -> Result<CrValue, CodegenError> {
    match rvalue {
        // Ownership/reference semantics
        Rvalue::Move(p) | Rvalue::Copy(p) => place::compile_place_read(ctx, state, builder, p),
        Rvalue::Ref(p) | Rvalue::RefMut(p) => place::compile_place_addr(ctx, state, builder, p),

        // Constants
        Rvalue::Const(imm) => immediate::compile_immediate(ctx, state, builder, imm),

        // Operations (dispatch by category via the Op enum)
        Rvalue::Op1 { op, arg } => {
            let a = compile_value(ctx, state, builder, arg)?;
            dispatch_op1(ctx, state, builder, op, a)
        }
        Rvalue::Op2 { op, lhs, rhs } => {
            let l = compile_value(ctx, state, builder, lhs)?;
            let r = compile_value(ctx, state, builder, rhs)?;
            dispatch_op2(ctx, state, builder, op, l, r)
        }
        Rvalue::Op3 { op, a, b, c } => {
            let va = compile_value(ctx, state, builder, a)?;
            let vb = compile_value(ctx, state, builder, b)?;
            let vc = compile_value(ctx, state, builder, c)?;
            dispatch_op3(ctx, state, builder, op, va, vb, vc)
        }

        // Composite construction
        Rvalue::Construct { ty, fields } => {
            construct::compile_construct(ctx, state, builder, ty, fields)
        }
        Rvalue::Tuple(values) => construct::compile_tuple(ctx, state, builder, values),
        Rvalue::EnumVariant {
            enum_ty,
            variant,
            payload,
        } => construct::compile_enum_variant(ctx, state, builder, enum_ty, variant, payload),
        Rvalue::ArrayLiteral {
            element_ty,
            values,
        } => construct::compile_array_literal(ctx, state, builder, element_ty, values),

        // Closures
        Rvalue::ApplyPartial { func, captures } => {
            closure::compile_apply_partial(ctx, state, builder, func, captures)
        }
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
        Value::Place(p) => place::compile_place_read(ctx, state, builder, p),
        Value::Immediate(imm) => immediate::compile_immediate(ctx, state, builder, imm),
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
        Op::PtrNull(_) | Op::PtrFromAddress(_) | Op::PtrToAddress | Op::PtrIsNull
        | Op::PtrCast(_) | Op::PtrBitcast(_) | Op::RefToPtr | Op::PtrRead(_) => {
            pointer::compile_pointer_op1(ctx, state, builder, op, arg)
        }

        // Memory
        Op::SizeOf(_) | Op::AlignOf(_) | Op::StackAlloc(_) => {
            pointer::compile_memory_op1(ctx, state, builder, op, arg)
        }

        // String ops (StrPtr, StrLen only — StrEq/IntToString are not emitted by lib2)
        Op::StrPtr | Op::StrLen => {
            string::compile_string_op1(ctx, state, builder, op, arg)
        }

        // Float intrinsics
        Op::FloatConst(_, _) | Op::FloatPred(_, _) | Op::FloatMath(_, _) => {
            arithmetic::compile_float_intrinsic_op1(ctx, state, builder, op, arg)
        }

        // Casts
        Op::IntWiden(_, _) | Op::IntTruncate(_, _) | Op::FloatWiden(_, _)
        | Op::FloatTruncate(_, _) | Op::IntToFloat(_, _) | Op::FloatToInt(_, _)
        | Op::RefToImmut => {
            cast::compile_cast_op1(ctx, state, builder, op, arg)
        }

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
        Op::PtrOffset | Op::PtrWrite => {
            pointer::compile_pointer_op2(ctx, state, builder, op, lhs, rhs)
        }

        // Atomic
        Op::AtomicAdd | Op::AtomicSub => {
            pointer::compile_atomic_op2(ctx, state, builder, op, lhs, rhs)
        }

        // Float binary
        Op::FloatCopysign(_) => {
            arithmetic::compile_float_intrinsic_op2(ctx, state, builder, op, lhs, rhs)
        }

        // Arithmetic / bitwise / comparison / boolean binary
        _ => arithmetic::compile_op2(ctx, state, builder, op, lhs, rhs),
    }
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
