use kestrel_ast_builder::Intrinsic;
use kestrel_hecs::Entity;
use kestrel_hir::body::{HirCallArg, HirExprId};
use kestrel_mir_3::{FloatBits, FloatMathKind, FloatPredicateKind, Immediate, ImmediateKind, IntBits, Op, Signedness, ValueId};
use kestrel_mir_3::inst::InstKind;

use crate::body::OssaBodyCtx;

struct IntrinsicEntry {
    name: &'static str,
    op: Op,
    arity: u8,
}

static TABLE: &[IntrinsicEntry] = &[
    IntrinsicEntry { name: "i1_eq", op: Op::BoolEq, arity: 2 },
    IntrinsicEntry { name: "i1_and", op: Op::BoolAnd, arity: 2 },
    IntrinsicEntry { name: "i1_or", op: Op::BoolOr, arity: 2 },
    IntrinsicEntry { name: "i1_not", op: Op::BoolNot, arity: 1 },
    IntrinsicEntry { name: "i8_add", op: Op::Add(IntBits::I8, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i8_sub", op: Op::Sub(IntBits::I8, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i8_mul", op: Op::Mul(IntBits::I8, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i8_signed_div", op: Op::Div(IntBits::I8, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i8_signed_rem", op: Op::Rem(IntBits::I8, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i8_unsigned_div", op: Op::Div(IntBits::I8, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i8_unsigned_rem", op: Op::Rem(IntBits::I8, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i8_neg", op: Op::Neg(IntBits::I8), arity: 1 },
    IntrinsicEntry { name: "i8_not", op: Op::Not(IntBits::I8), arity: 1 },
    IntrinsicEntry { name: "i8_eq", op: Op::Eq(IntBits::I8), arity: 2 },
    IntrinsicEntry { name: "i8_ne", op: Op::Ne(IntBits::I8), arity: 2 },
    IntrinsicEntry { name: "i8_signed_lt", op: Op::Lt(IntBits::I8, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i8_signed_le", op: Op::Le(IntBits::I8, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i8_signed_gt", op: Op::Gt(IntBits::I8, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i8_signed_ge", op: Op::Ge(IntBits::I8, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i8_unsigned_lt", op: Op::Lt(IntBits::I8, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i8_unsigned_le", op: Op::Le(IntBits::I8, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i8_unsigned_gt", op: Op::Gt(IntBits::I8, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i8_unsigned_ge", op: Op::Ge(IntBits::I8, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i8_and", op: Op::And(IntBits::I8), arity: 2 },
    IntrinsicEntry { name: "i8_or", op: Op::Or(IntBits::I8), arity: 2 },
    IntrinsicEntry { name: "i8_xor", op: Op::Xor(IntBits::I8), arity: 2 },
    IntrinsicEntry { name: "i8_shl", op: Op::Shl(IntBits::I8), arity: 2 },
    IntrinsicEntry { name: "i8_signed_shr", op: Op::Shr(IntBits::I8, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i8_unsigned_shr", op: Op::Shr(IntBits::I8, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i8_popcount", op: Op::Popcount(IntBits::I8), arity: 1 },
    IntrinsicEntry { name: "i8_clz", op: Op::Clz(IntBits::I8), arity: 1 },
    IntrinsicEntry { name: "i8_ctz", op: Op::Ctz(IntBits::I8), arity: 1 },
    IntrinsicEntry { name: "i16_add", op: Op::Add(IntBits::I16, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i16_sub", op: Op::Sub(IntBits::I16, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i16_mul", op: Op::Mul(IntBits::I16, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i16_signed_div", op: Op::Div(IntBits::I16, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i16_signed_rem", op: Op::Rem(IntBits::I16, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i16_unsigned_div", op: Op::Div(IntBits::I16, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i16_unsigned_rem", op: Op::Rem(IntBits::I16, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i16_neg", op: Op::Neg(IntBits::I16), arity: 1 },
    IntrinsicEntry { name: "i16_not", op: Op::Not(IntBits::I16), arity: 1 },
    IntrinsicEntry { name: "i16_eq", op: Op::Eq(IntBits::I16), arity: 2 },
    IntrinsicEntry { name: "i16_ne", op: Op::Ne(IntBits::I16), arity: 2 },
    IntrinsicEntry { name: "i16_signed_lt", op: Op::Lt(IntBits::I16, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i16_signed_le", op: Op::Le(IntBits::I16, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i16_signed_gt", op: Op::Gt(IntBits::I16, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i16_signed_ge", op: Op::Ge(IntBits::I16, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i16_unsigned_lt", op: Op::Lt(IntBits::I16, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i16_unsigned_le", op: Op::Le(IntBits::I16, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i16_unsigned_gt", op: Op::Gt(IntBits::I16, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i16_unsigned_ge", op: Op::Ge(IntBits::I16, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i16_and", op: Op::And(IntBits::I16), arity: 2 },
    IntrinsicEntry { name: "i16_or", op: Op::Or(IntBits::I16), arity: 2 },
    IntrinsicEntry { name: "i16_xor", op: Op::Xor(IntBits::I16), arity: 2 },
    IntrinsicEntry { name: "i16_shl", op: Op::Shl(IntBits::I16), arity: 2 },
    IntrinsicEntry { name: "i16_signed_shr", op: Op::Shr(IntBits::I16, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i16_unsigned_shr", op: Op::Shr(IntBits::I16, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i16_popcount", op: Op::Popcount(IntBits::I16), arity: 1 },
    IntrinsicEntry { name: "i16_clz", op: Op::Clz(IntBits::I16), arity: 1 },
    IntrinsicEntry { name: "i16_ctz", op: Op::Ctz(IntBits::I16), arity: 1 },
    IntrinsicEntry { name: "i16_bswap", op: Op::Bswap(IntBits::I16), arity: 1 },
    IntrinsicEntry { name: "i32_add", op: Op::Add(IntBits::I32, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i32_sub", op: Op::Sub(IntBits::I32, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i32_mul", op: Op::Mul(IntBits::I32, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i32_signed_div", op: Op::Div(IntBits::I32, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i32_signed_rem", op: Op::Rem(IntBits::I32, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i32_unsigned_div", op: Op::Div(IntBits::I32, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i32_unsigned_rem", op: Op::Rem(IntBits::I32, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i32_neg", op: Op::Neg(IntBits::I32), arity: 1 },
    IntrinsicEntry { name: "i32_not", op: Op::Not(IntBits::I32), arity: 1 },
    IntrinsicEntry { name: "i32_eq", op: Op::Eq(IntBits::I32), arity: 2 },
    IntrinsicEntry { name: "i32_ne", op: Op::Ne(IntBits::I32), arity: 2 },
    IntrinsicEntry { name: "i32_signed_lt", op: Op::Lt(IntBits::I32, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i32_signed_le", op: Op::Le(IntBits::I32, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i32_signed_gt", op: Op::Gt(IntBits::I32, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i32_signed_ge", op: Op::Ge(IntBits::I32, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i32_unsigned_lt", op: Op::Lt(IntBits::I32, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i32_unsigned_le", op: Op::Le(IntBits::I32, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i32_unsigned_gt", op: Op::Gt(IntBits::I32, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i32_unsigned_ge", op: Op::Ge(IntBits::I32, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i32_and", op: Op::And(IntBits::I32), arity: 2 },
    IntrinsicEntry { name: "i32_or", op: Op::Or(IntBits::I32), arity: 2 },
    IntrinsicEntry { name: "i32_xor", op: Op::Xor(IntBits::I32), arity: 2 },
    IntrinsicEntry { name: "i32_shl", op: Op::Shl(IntBits::I32), arity: 2 },
    IntrinsicEntry { name: "i32_signed_shr", op: Op::Shr(IntBits::I32, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i32_unsigned_shr", op: Op::Shr(IntBits::I32, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i32_popcount", op: Op::Popcount(IntBits::I32), arity: 1 },
    IntrinsicEntry { name: "i32_clz", op: Op::Clz(IntBits::I32), arity: 1 },
    IntrinsicEntry { name: "i32_ctz", op: Op::Ctz(IntBits::I32), arity: 1 },
    IntrinsicEntry { name: "i32_bswap", op: Op::Bswap(IntBits::I32), arity: 1 },
    IntrinsicEntry { name: "i64_add", op: Op::Add(IntBits::I64, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i64_sub", op: Op::Sub(IntBits::I64, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i64_mul", op: Op::Mul(IntBits::I64, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i64_signed_div", op: Op::Div(IntBits::I64, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i64_signed_rem", op: Op::Rem(IntBits::I64, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i64_unsigned_div", op: Op::Div(IntBits::I64, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i64_unsigned_rem", op: Op::Rem(IntBits::I64, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i64_neg", op: Op::Neg(IntBits::I64), arity: 1 },
    IntrinsicEntry { name: "i64_not", op: Op::Not(IntBits::I64), arity: 1 },
    IntrinsicEntry { name: "i64_eq", op: Op::Eq(IntBits::I64), arity: 2 },
    IntrinsicEntry { name: "i64_ne", op: Op::Ne(IntBits::I64), arity: 2 },
    IntrinsicEntry { name: "i64_signed_lt", op: Op::Lt(IntBits::I64, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i64_signed_le", op: Op::Le(IntBits::I64, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i64_signed_gt", op: Op::Gt(IntBits::I64, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i64_signed_ge", op: Op::Ge(IntBits::I64, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i64_unsigned_lt", op: Op::Lt(IntBits::I64, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i64_unsigned_le", op: Op::Le(IntBits::I64, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i64_unsigned_gt", op: Op::Gt(IntBits::I64, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i64_unsigned_ge", op: Op::Ge(IntBits::I64, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i64_and", op: Op::And(IntBits::I64), arity: 2 },
    IntrinsicEntry { name: "i64_or", op: Op::Or(IntBits::I64), arity: 2 },
    IntrinsicEntry { name: "i64_xor", op: Op::Xor(IntBits::I64), arity: 2 },
    IntrinsicEntry { name: "i64_shl", op: Op::Shl(IntBits::I64), arity: 2 },
    IntrinsicEntry { name: "i64_signed_shr", op: Op::Shr(IntBits::I64, Signedness::Signed), arity: 2 },
    IntrinsicEntry { name: "i64_unsigned_shr", op: Op::Shr(IntBits::I64, Signedness::Unsigned), arity: 2 },
    IntrinsicEntry { name: "i64_popcount", op: Op::Popcount(IntBits::I64), arity: 1 },
    IntrinsicEntry { name: "i64_clz", op: Op::Clz(IntBits::I64), arity: 1 },
    IntrinsicEntry { name: "i64_ctz", op: Op::Ctz(IntBits::I64), arity: 1 },
    IntrinsicEntry { name: "i64_bswap", op: Op::Bswap(IntBits::I64), arity: 1 },
    IntrinsicEntry { name: "cast_i8_i16", op: Op::IntWiden(IntBits::I8, IntBits::I16), arity: 1 },
    IntrinsicEntry { name: "cast_i8_i32", op: Op::IntWiden(IntBits::I8, IntBits::I32), arity: 1 },
    IntrinsicEntry { name: "cast_i8_i64", op: Op::IntWiden(IntBits::I8, IntBits::I64), arity: 1 },
    IntrinsicEntry { name: "cast_i16_i32", op: Op::IntWiden(IntBits::I16, IntBits::I32), arity: 1 },
    IntrinsicEntry { name: "cast_i16_i64", op: Op::IntWiden(IntBits::I16, IntBits::I64), arity: 1 },
    IntrinsicEntry { name: "cast_i32_i64", op: Op::IntWiden(IntBits::I32, IntBits::I64), arity: 1 },
    IntrinsicEntry { name: "cast_i64_i32", op: Op::IntTruncate(IntBits::I64, IntBits::I32), arity: 1 },
    IntrinsicEntry { name: "cast_i64_i16", op: Op::IntTruncate(IntBits::I64, IntBits::I16), arity: 1 },
    IntrinsicEntry { name: "cast_i64_i8", op: Op::IntTruncate(IntBits::I64, IntBits::I8), arity: 1 },
    IntrinsicEntry { name: "cast_i32_i16", op: Op::IntTruncate(IntBits::I32, IntBits::I16), arity: 1 },
    IntrinsicEntry { name: "cast_i32_i8", op: Op::IntTruncate(IntBits::I32, IntBits::I8), arity: 1 },
    IntrinsicEntry { name: "cast_i16_i8", op: Op::IntTruncate(IntBits::I16, IntBits::I8), arity: 1 },
    IntrinsicEntry { name: "cast_u8_i16", op: Op::IntUnsignedWiden(IntBits::I8, IntBits::I16), arity: 1 },
    IntrinsicEntry { name: "cast_u8_i32", op: Op::IntUnsignedWiden(IntBits::I8, IntBits::I32), arity: 1 },
    IntrinsicEntry { name: "cast_u8_i64", op: Op::IntUnsignedWiden(IntBits::I8, IntBits::I64), arity: 1 },
    IntrinsicEntry { name: "cast_u16_i32", op: Op::IntUnsignedWiden(IntBits::I16, IntBits::I32), arity: 1 },
    IntrinsicEntry { name: "cast_u16_i64", op: Op::IntUnsignedWiden(IntBits::I16, IntBits::I64), arity: 1 },
    IntrinsicEntry { name: "cast_u32_i64", op: Op::IntUnsignedWiden(IntBits::I32, IntBits::I64), arity: 1 },
    IntrinsicEntry { name: "cast_u16_i8", op: Op::IntTruncate(IntBits::I16, IntBits::I8), arity: 1 },
    IntrinsicEntry { name: "cast_u32_i8", op: Op::IntTruncate(IntBits::I32, IntBits::I8), arity: 1 },
    IntrinsicEntry { name: "cast_u32_i16", op: Op::IntTruncate(IntBits::I32, IntBits::I16), arity: 1 },
    IntrinsicEntry { name: "cast_u64_i8", op: Op::IntTruncate(IntBits::I64, IntBits::I8), arity: 1 },
    IntrinsicEntry { name: "cast_u64_i16", op: Op::IntTruncate(IntBits::I64, IntBits::I16), arity: 1 },
    IntrinsicEntry { name: "cast_u64_i32", op: Op::IntTruncate(IntBits::I64, IntBits::I32), arity: 1 },
    IntrinsicEntry { name: "f32_add", op: Op::FAdd(FloatBits::F32), arity: 2 },
    IntrinsicEntry { name: "f32_sub", op: Op::FSub(FloatBits::F32), arity: 2 },
    IntrinsicEntry { name: "f32_mul", op: Op::FMul(FloatBits::F32), arity: 2 },
    IntrinsicEntry { name: "f32_div", op: Op::FDiv(FloatBits::F32), arity: 2 },
    IntrinsicEntry { name: "f32_neg", op: Op::FNeg(FloatBits::F32), arity: 1 },
    IntrinsicEntry { name: "f64_add", op: Op::FAdd(FloatBits::F64), arity: 2 },
    IntrinsicEntry { name: "f64_sub", op: Op::FSub(FloatBits::F64), arity: 2 },
    IntrinsicEntry { name: "f64_mul", op: Op::FMul(FloatBits::F64), arity: 2 },
    IntrinsicEntry { name: "f64_div", op: Op::FDiv(FloatBits::F64), arity: 2 },
    IntrinsicEntry { name: "f64_neg", op: Op::FNeg(FloatBits::F64), arity: 1 },
    IntrinsicEntry { name: "f32_eq", op: Op::FEq(FloatBits::F32), arity: 2 },
    IntrinsicEntry { name: "f32_ne", op: Op::FNe(FloatBits::F32), arity: 2 },
    IntrinsicEntry { name: "f32_lt", op: Op::FLt(FloatBits::F32), arity: 2 },
    IntrinsicEntry { name: "f32_le", op: Op::FLe(FloatBits::F32), arity: 2 },
    IntrinsicEntry { name: "f32_gt", op: Op::FGt(FloatBits::F32), arity: 2 },
    IntrinsicEntry { name: "f32_ge", op: Op::FGe(FloatBits::F32), arity: 2 },
    IntrinsicEntry { name: "f64_eq", op: Op::FEq(FloatBits::F64), arity: 2 },
    IntrinsicEntry { name: "f64_ne", op: Op::FNe(FloatBits::F64), arity: 2 },
    IntrinsicEntry { name: "f64_lt", op: Op::FLt(FloatBits::F64), arity: 2 },
    IntrinsicEntry { name: "f64_le", op: Op::FLe(FloatBits::F64), arity: 2 },
    IntrinsicEntry { name: "f64_gt", op: Op::FGt(FloatBits::F64), arity: 2 },
    IntrinsicEntry { name: "f64_ge", op: Op::FGe(FloatBits::F64), arity: 2 },
    IntrinsicEntry { name: "cast_i64_f64", op: Op::IntToFloat(IntBits::I64, FloatBits::F64), arity: 1 },
    IntrinsicEntry { name: "cast_i32_f32", op: Op::IntToFloat(IntBits::I32, FloatBits::F32), arity: 1 },
    IntrinsicEntry { name: "cast_i32_f64", op: Op::IntToFloat(IntBits::I32, FloatBits::F64), arity: 1 },
    IntrinsicEntry { name: "cast_f64_i64", op: Op::FloatToInt(FloatBits::F64, IntBits::I64), arity: 1 },
    IntrinsicEntry { name: "cast_f32_i32", op: Op::FloatToInt(FloatBits::F32, IntBits::I32), arity: 1 },
    IntrinsicEntry { name: "cast_f32_i64", op: Op::FloatToInt(FloatBits::F32, IntBits::I64), arity: 1 },
    IntrinsicEntry { name: "cast_i64_f32", op: Op::IntToFloat(IntBits::I64, FloatBits::F32), arity: 1 },
    IntrinsicEntry { name: "cast_f32_f64", op: Op::FloatWiden(FloatBits::F32, FloatBits::F64), arity: 1 },
    IntrinsicEntry { name: "cast_f64_f32", op: Op::FloatTruncate(FloatBits::F64, FloatBits::F32), arity: 1 },
    IntrinsicEntry { name: "f32_floor", op: Op::FloatMath(FloatBits::F32, FloatMathKind::Floor), arity: 1 },
    IntrinsicEntry { name: "f32_ceil", op: Op::FloatMath(FloatBits::F32, FloatMathKind::Ceil), arity: 1 },
    IntrinsicEntry { name: "f32_round", op: Op::FloatMath(FloatBits::F32, FloatMathKind::Round), arity: 1 },
    IntrinsicEntry { name: "f32_trunc", op: Op::FloatMath(FloatBits::F32, FloatMathKind::Trunc), arity: 1 },
    IntrinsicEntry { name: "f32_sqrt", op: Op::FloatMath(FloatBits::F32, FloatMathKind::Sqrt), arity: 1 },
    IntrinsicEntry { name: "f64_floor", op: Op::FloatMath(FloatBits::F64, FloatMathKind::Floor), arity: 1 },
    IntrinsicEntry { name: "f64_ceil", op: Op::FloatMath(FloatBits::F64, FloatMathKind::Ceil), arity: 1 },
    IntrinsicEntry { name: "f64_round", op: Op::FloatMath(FloatBits::F64, FloatMathKind::Round), arity: 1 },
    IntrinsicEntry { name: "f64_trunc", op: Op::FloatMath(FloatBits::F64, FloatMathKind::Trunc), arity: 1 },
    IntrinsicEntry { name: "f64_sqrt", op: Op::FloatMath(FloatBits::F64, FloatMathKind::Sqrt), arity: 1 },
    IntrinsicEntry { name: "f32_fma", op: Op::FloatFma(FloatBits::F32), arity: 3 },
    IntrinsicEntry { name: "f64_fma", op: Op::FloatFma(FloatBits::F64), arity: 3 },
    IntrinsicEntry { name: "f32_copysign", op: Op::FloatCopysign(FloatBits::F32), arity: 2 },
    IntrinsicEntry { name: "f64_copysign", op: Op::FloatCopysign(FloatBits::F64), arity: 2 },
    IntrinsicEntry { name: "f32_is_nan", op: Op::FloatPred(FloatBits::F32, FloatPredicateKind::IsNan), arity: 1 },
    IntrinsicEntry { name: "f64_is_nan", op: Op::FloatPred(FloatBits::F64, FloatPredicateKind::IsNan), arity: 1 },
    IntrinsicEntry { name: "f32_is_infinite", op: Op::FloatPred(FloatBits::F32, FloatPredicateKind::IsInfinite), arity: 1 },
    IntrinsicEntry { name: "f64_is_infinite", op: Op::FloatPred(FloatBits::F64, FloatPredicateKind::IsInfinite), arity: 1 },
    IntrinsicEntry { name: "ptr_offset", op: Op::PtrOffset, arity: 2 },
    IntrinsicEntry { name: "ptr_to_address", op: Op::PtrToAddress, arity: 1 },
    IntrinsicEntry { name: "ptr_is_null", op: Op::PtrIsNull, arity: 1 },
    IntrinsicEntry { name: "ref_to_ptr", op: Op::RefToPtr, arity: 1 },
    IntrinsicEntry { name: "str_ptr", op: Op::StrPtr, arity: 1 },
    IntrinsicEntry { name: "str_len", op: Op::StrLen, arity: 1 },
    IntrinsicEntry { name: "atomic_add", op: Op::AtomicAdd, arity: 2 },
    IntrinsicEntry { name: "atomic_sub", op: Op::AtomicSub, arity: 2 },
];

pub(crate) fn try_intrinsic(
    bctx: &mut OssaBodyCtx,
    expr_id: HirExprId,
    callee_expr: HirExprId,
    callee_entity: Entity,
    args: &[HirCallArg],
) -> Option<ValueId> {
    bctx.ctx.world.get::<Intrinsic>(callee_entity)?;
    let name = bctx.ctx.world.get::<kestrel_ast_builder::Name>(callee_entity)?.0.clone();

    if name == "panic" || name == "panic_unwind" {
        bctx.emit_panic("panic");
        return Some(bctx.emit_literal(Immediate::unit()));
    }

    match name.as_str() {
        "f32_infinity" => return Some(bctx.emit_literal(Immediate::new(ImmediateKind::FloatInfinity(FloatBits::F32)))),
        "f64_infinity" => return Some(bctx.emit_literal(Immediate::new(ImmediateKind::FloatInfinity(FloatBits::F64)))),
        "f32_nan" => return Some(bctx.emit_literal(Immediate::new(ImmediateKind::FloatNan(FloatBits::F32)))),
        "f64_nan" => return Some(bctx.emit_literal(Immediate::new(ImmediateKind::FloatNan(FloatBits::F64)))),
        _ => {}
    }

    let mut type_args = bctx.resolve_type_args(expr_id);
    if type_args.is_empty() {
        type_args = bctx.resolve_type_args(callee_expr);
    }
    match name.as_str() {
        "sizeof" | "alignof" => {
            let ty_arg = *type_args.first()?;
            let result_ty = bctx.resolve_expr_type(expr_id);
            let op = if name == "sizeof" { Op::SizeOf(ty_arg) } else { Op::AlignOf(ty_arg) };
            let unit = bctx.emit_literal(Immediate::unit());
            return Some(bctx.emit_op1(op, unit, result_ty));
        }
        "ptr_null" => {
            let ty_arg = *type_args.first()?;
            let result_ty = bctx.resolve_expr_type(expr_id);
            let unit = bctx.emit_literal(Immediate::unit());
            return Some(bctx.emit_op1(Op::PtrNull(ty_arg), unit, result_ty));
        }
        "ptr_to" => {
            let ty_arg = *type_args.first()?;
            let result_ty = bctx.resolve_expr_type(expr_id);
            let arg = bctx.lower_expr(args.first()?.value);
            return Some(bctx.emit_op1(Op::PtrTo(ty_arg), arg, result_ty));
        }
        "ptr_from_address" => {
            let ty_arg = *type_args.first()?;
            let result_ty = bctx.resolve_expr_type(expr_id);
            let arg = bctx.lower_expr(args.first()?.value);
            return Some(bctx.emit_op1(Op::PtrFromAddress(ty_arg), arg, result_ty));
        }
        "ptr_read" => {
            let ty_arg = *type_args.first()?;
            let result_ty = bctx.resolve_expr_type(expr_id);
            let arg = bctx.lower_expr(args.first()?.value);
            // PtrRead produces a @guaranteed view into the pointed-to memory,
            // immediately followed by CopyValue → @owned clone + EndBorrow.
            // For Named types, CopyValue expands to clone (e.g. incrementing
            // RcBoxStorage refcount), preventing double-frees.
            let view = bctx.alloc_guaranteed(result_ty, arg);
            bctx.push_inst(InstKind::Op1 { result: view, op: Op::PtrRead(ty_arg), arg });
            let owned = bctx.emit_copy_value(view);
            bctx.emit_end_borrow(view);
            return Some(owned);
        }
        "drop_in_place" => {
            let ty_arg = *type_args.first()?;
            let arg = bctx.lower_expr(args.first()?.value);
            bctx.push_inst(kestrel_mir_3::inst::InstKind::DestroyAddr { address: arg, ty: ty_arg });
            return Some(bctx.emit_literal(Immediate::unit()));
        }
        "ptr_write" => {
            let ty_arg = *type_args.first()?;
            let result_ty = bctx.resolve_expr_type(expr_id);
            let ptr = bctx.lower_expr(args.get(0)?.value);
            // Consume the value — PtrWrite moves it into the destination address.
            let val = bctx.lower_expr_for_consuming(args.get(1)?.value);
            bctx.consume(val);
            return Some(bctx.emit_op2(Op::PtrWrite(ty_arg), ptr, val, result_ty));
        }
        "cast_ptr" => {
            let ty_arg = type_args.get(1).or(type_args.first()).copied()?;
            let result_ty = bctx.resolve_expr_type(expr_id);
            let arg = bctx.lower_expr(args.first()?.value);
            return Some(bctx.emit_op1(Op::PtrCast(ty_arg), arg, result_ty));
        }
        "stack_alloc" => {
            let ty_arg = *type_args.first()?;
            let result_ty = bctx.resolve_expr_type(expr_id);
            let count = bctx.lower_expr(args.first()?.value);
            return Some(bctx.emit_op1(Op::StackAlloc(ty_arg), count, result_ty));
        }
        _ => {}
    }

    let entry = TABLE.iter().find(|e| e.name == name)?;
    let result_ty = bctx.resolve_expr_type(expr_id);

    match entry.arity {
        1 => {
            let arg = bctx.lower_expr(args.first()?.value);
            Some(bctx.emit_op1(entry.op, arg, result_ty))
        }
        2 => {
            let lhs = bctx.lower_expr(args.get(0)?.value);
            let rhs = bctx.lower_expr(args.get(1)?.value);
            Some(bctx.emit_op2(entry.op, lhs, rhs, result_ty))
        }
        3 => {
            let a = bctx.lower_expr(args.get(0)?.value);
            let b = bctx.lower_expr(args.get(1)?.value);
            let c = bctx.lower_expr(args.get(2)?.value);
            Some(bctx.emit_op3(entry.op, a, b, c, result_ty))
        }
        _ => None,
    }
}
