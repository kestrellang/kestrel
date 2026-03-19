//! Arithmetic, bitwise, comparison, and boolean operation compilation.
//!
//! Key simplification over lib1: every Op carries explicit width info
//! (IntBits/FloatBits), so we never need to infer types from context.

use crate::common;
use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::function::FunctionState;
use crate::types::{float_bits_to_type, int_bits_to_type};
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{self, InstBuilder, Value as CrValue};
use cranelift_frontend::FunctionBuilder;
use kestrel_mir::{FloatBits, FloatConstantKind, FloatMathKind, FloatPredicateKind, IntBits, Op, Signedness};

/// Compile a unary arithmetic/bitwise/boolean operation.
pub fn compile_op1(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    op: &Op,
    arg: CrValue,
) -> Result<CrValue, CodegenError> {
    match op {
        Op::Neg(bits) => Ok(builder.ins().ineg(arg)),
        Op::FNeg(_) => Ok(builder.ins().fneg(arg)),

        Op::Not(bits) => Ok(builder.ins().bnot(arg)),

        Op::BoolNot => {
            let one = builder.ins().iconst(ir::types::I8, 1);
            Ok(builder.ins().bxor(arg, one))
        }

        Op::Popcount(_) => Ok(builder.ins().popcnt(arg)),
        Op::Clz(_) => Ok(builder.ins().clz(arg)),
        Op::Ctz(_) => Ok(builder.ins().ctz(arg)),
        Op::Bswap(_) => Ok(builder.ins().bswap(arg)),

        _ => Err(CodegenError::Unsupported(format!("unary op: {op:?}"))),
    }
}

/// Compile a binary arithmetic/bitwise/comparison/boolean operation.
pub fn compile_op2(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    op: &Op,
    lhs: CrValue,
    rhs: CrValue,
) -> Result<CrValue, CodegenError> {
    match op {
        // Integer arithmetic
        Op::Add(_, _) => Ok(builder.ins().iadd(lhs, rhs)),
        Op::Sub(_, _) => Ok(builder.ins().isub(lhs, rhs)),
        Op::Mul(_, _) => Ok(builder.ins().imul(lhs, rhs)),
        Op::Div(_, Signedness::Signed) => Ok(builder.ins().sdiv(lhs, rhs)),
        Op::Div(_, Signedness::Unsigned) => Ok(builder.ins().udiv(lhs, rhs)),
        Op::Rem(_, Signedness::Signed) => Ok(builder.ins().srem(lhs, rhs)),
        Op::Rem(_, Signedness::Unsigned) => Ok(builder.ins().urem(lhs, rhs)),

        // Float arithmetic
        Op::FAdd(_) => Ok(builder.ins().fadd(lhs, rhs)),
        Op::FSub(_) => Ok(builder.ins().fsub(lhs, rhs)),
        Op::FMul(_) => Ok(builder.ins().fmul(lhs, rhs)),
        Op::FDiv(_) => Ok(builder.ins().fdiv(lhs, rhs)),

        // Bitwise
        Op::And(_) => Ok(builder.ins().band(lhs, rhs)),
        Op::Or(_) => Ok(builder.ins().bor(lhs, rhs)),
        Op::Xor(_) => Ok(builder.ins().bxor(lhs, rhs)),
        Op::Shl(_) => Ok(builder.ins().ishl(lhs, rhs)),
        Op::Shr(_, Signedness::Signed) => Ok(builder.ins().sshr(lhs, rhs)),
        Op::Shr(_, Signedness::Unsigned) => Ok(builder.ins().ushr(lhs, rhs)),

        // Integer comparison → i8 bool
        Op::Eq(_) => {
            let cmp = builder.ins().icmp(IntCC::Equal, lhs, rhs);
            Ok(cmp)
        }
        Op::Ne(_) => {
            let cmp = builder.ins().icmp(IntCC::NotEqual, lhs, rhs);
            Ok(cmp)
        }
        Op::Lt(_, Signedness::Signed) => {
            let cmp = builder.ins().icmp(IntCC::SignedLessThan, lhs, rhs);
            Ok(cmp)
        }
        Op::Lt(_, Signedness::Unsigned) => {
            let cmp = builder.ins().icmp(IntCC::UnsignedLessThan, lhs, rhs);
            Ok(cmp)
        }
        Op::Le(_, Signedness::Signed) => {
            let cmp = builder.ins().icmp(IntCC::SignedLessThanOrEqual, lhs, rhs);
            Ok(cmp)
        }
        Op::Le(_, Signedness::Unsigned) => {
            let cmp = builder.ins().icmp(IntCC::UnsignedLessThanOrEqual, lhs, rhs);
            Ok(cmp)
        }
        Op::Gt(_, Signedness::Signed) => {
            let cmp = builder.ins().icmp(IntCC::SignedGreaterThan, lhs, rhs);
            Ok(cmp)
        }
        Op::Gt(_, Signedness::Unsigned) => {
            let cmp = builder.ins().icmp(IntCC::UnsignedGreaterThan, lhs, rhs);
            Ok(cmp)
        }
        Op::Ge(_, Signedness::Signed) => {
            let cmp = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, lhs, rhs);
            Ok(cmp)
        }
        Op::Ge(_, Signedness::Unsigned) => {
            let cmp = builder.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, lhs, rhs);
            Ok(cmp)
        }

        // Float comparison
        Op::FEq(_) => Ok(builder.ins().fcmp(FloatCC::Equal, lhs, rhs)),
        Op::FNe(_) => Ok(builder.ins().fcmp(FloatCC::NotEqual, lhs, rhs)),
        Op::FLt(_) => Ok(builder.ins().fcmp(FloatCC::LessThan, lhs, rhs)),
        Op::FLe(_) => Ok(builder.ins().fcmp(FloatCC::LessThanOrEqual, lhs, rhs)),
        Op::FGt(_) => Ok(builder.ins().fcmp(FloatCC::GreaterThan, lhs, rhs)),
        Op::FGe(_) => Ok(builder.ins().fcmp(FloatCC::GreaterThanOrEqual, lhs, rhs)),

        // Boolean ops (i8)
        Op::BoolAnd => Ok(builder.ins().band(lhs, rhs)),
        Op::BoolOr => Ok(builder.ins().bor(lhs, rhs)),
        Op::BoolEq => {
            let cmp = builder.ins().icmp(IntCC::Equal, lhs, rhs);
            Ok(cmp)
        }

        _ => Err(CodegenError::Unsupported(format!("binary op: {op:?}"))),
    }
}

/// Compile float intrinsic unary ops (FloatConst, FloatPred, FloatMath).
pub fn compile_float_intrinsic_op1(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    op: &Op,
    arg: CrValue,
) -> Result<CrValue, CodegenError> {
    match op {
        Op::FloatConst(bits, kind) => {
            match (bits, kind) {
                (FloatBits::F16, _) => {
                    Err(CodegenError::Unsupported("f16 constants not yet supported".into()))
                }
                (FloatBits::F32, FloatConstantKind::Infinity) => {
                    Ok(builder.ins().f32const(f32::INFINITY))
                }
                (FloatBits::F32, FloatConstantKind::Nan) => {
                    Ok(builder.ins().f32const(f32::NAN))
                }
                (FloatBits::F64, FloatConstantKind::Infinity) => {
                    Ok(builder.ins().f64const(f64::INFINITY))
                }
                (FloatBits::F64, FloatConstantKind::Nan) => {
                    Ok(builder.ins().f64const(f64::NAN))
                }
            }
        }

        Op::FloatPred(bits, pred) => {
            if matches!(bits, FloatBits::F16) {
                return Err(CodegenError::Unsupported("f16 predicates not yet supported".into()));
            }
            match pred {
                FloatPredicateKind::IsNan => {
                    // NaN != NaN
                    Ok(builder.ins().fcmp(FloatCC::Unordered, arg, arg))
                }
                FloatPredicateKind::IsInfinite => {
                    // |arg| == infinity
                    let abs = builder.ins().fabs(arg);
                    let inf = match bits {
                        FloatBits::F32 => builder.ins().f32const(f32::INFINITY),
                        FloatBits::F64 => builder.ins().f64const(f64::INFINITY),
                        FloatBits::F16 => unreachable!(),
                    };
                    Ok(builder.ins().fcmp(FloatCC::Equal, abs, inf))
                }
            }
        }

        Op::FloatMath(_, kind) => {
            let result = match kind {
                FloatMathKind::Floor => builder.ins().floor(arg),
                FloatMathKind::Ceil => builder.ins().ceil(arg),
                FloatMathKind::Round => builder.ins().nearest(arg),
                FloatMathKind::Trunc => builder.ins().trunc(arg),
                FloatMathKind::Sqrt => builder.ins().sqrt(arg),
            };
            Ok(result)
        }

        _ => Err(CodegenError::Unsupported(format!(
            "float intrinsic op1: {op:?}"
        ))),
    }
}

/// Compile float intrinsic binary ops (FloatCopysign).
pub fn compile_float_intrinsic_op2(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    op: &Op,
    lhs: CrValue,
    rhs: CrValue,
) -> Result<CrValue, CodegenError> {
    match op {
        Op::FloatCopysign(_) => Ok(builder.ins().fcopysign(lhs, rhs)),
        _ => Err(CodegenError::Unsupported(format!(
            "float intrinsic op2: {op:?}"
        ))),
    }
}

/// Compile FloatFma (a * b + c).
pub fn compile_float_fma(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    op: &Op,
    a: CrValue,
    b: CrValue,
    c: CrValue,
) -> Result<CrValue, CodegenError> {
    match op {
        Op::FloatFma(_) => Ok(builder.ins().fma(a, b, c)),
        _ => Err(CodegenError::Unsupported(format!("op3: {op:?}"))),
    }
}
