//! Type conversion operations — int/float widen/truncate/convert.

use crate::context::CodegenContext;
use crate::error::CodegenError;
use crate::function::FunctionState;
use crate::types::{float_bits_to_type, int_bits_to_type};
use cranelift_codegen::ir::{InstBuilder, Value as CrValue};
use cranelift_frontend::FunctionBuilder;
use kestrel_mir::{IntBits, Op, Signedness};

/// Compile a cast operation (Op1).
pub fn compile_cast_op1(
    ctx: &mut CodegenContext,
    state: &FunctionState,
    builder: &mut FunctionBuilder,
    op: &Op,
    arg: CrValue,
) -> Result<CrValue, CodegenError> {
    match op {
        Op::IntWiden(from, to) => {
            let to_ty = int_bits_to_type(*to);
            // Determine sign extension vs zero extension based on from width
            // Default to sign-extend (most common in Kestrel)
            Ok(builder.ins().sextend(to_ty, arg))
        }

        Op::IntTruncate(from, to) => {
            let to_ty = int_bits_to_type(*to);
            Ok(builder.ins().ireduce(to_ty, arg))
        }

        Op::IntToFloat(int_bits, float_bits) => {
            let to_ty = float_bits_to_type(*float_bits);
            // Signed int → float (most common case)
            Ok(builder.ins().fcvt_from_sint(to_ty, arg))
        }

        Op::FloatToInt(float_bits, int_bits) => {
            let to_ty = int_bits_to_type(*int_bits);
            // Float → signed int with saturation
            Ok(builder.ins().fcvt_to_sint_sat(to_ty, arg))
        }

        Op::FloatWiden(from, to) => {
            let to_ty = float_bits_to_type(*to);
            Ok(builder.ins().fpromote(to_ty, arg))
        }

        Op::FloatTruncate(from, to) => {
            let to_ty = float_bits_to_type(*to);
            Ok(builder.ins().fdemote(to_ty, arg))
        }

        // RefToImmut: &var T → &T — same representation at runtime
        Op::RefToImmut => Ok(arg),

        _ => Err(CodegenError::Unsupported(format!("cast op: {op:?}"))),
    }
}
