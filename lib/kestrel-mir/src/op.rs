//! Operations — arithmetic, comparisons, casts, pointer ops, intrinsics.
//!
//! All operations are variants of a single `Op` enum. Arity is enforced at the
//! `Rvalue` level via `Op1`/`Op2`/`Op3`.

use crate::ty::MirTy;
use std::fmt;

/// Integer bit widths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntBits {
    I8,
    I16,
    I32,
    I64,
}

impl IntBits {
    pub fn as_str(self) -> &'static str {
        match self {
            IntBits::I8 => "i8",
            IntBits::I16 => "i16",
            IntBits::I32 => "i32",
            IntBits::I64 => "i64",
        }
    }
}

impl fmt::Display for IntBits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Float bit widths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatBits {
    F16,
    F32,
    F64,
}

impl FloatBits {
    pub fn as_str(self) -> &'static str {
        match self {
            FloatBits::F16 => "f16",
            FloatBits::F32 => "f32",
            FloatBits::F64 => "f64",
        }
    }
}

impl fmt::Display for FloatBits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Integer signedness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Signedness {
    Signed,
    Unsigned,
}

impl Signedness {
    pub fn as_str(self) -> &'static str {
        match self {
            Signedness::Signed => "signed",
            Signedness::Unsigned => "unsigned",
        }
    }
}

impl fmt::Display for Signedness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Float constant kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatConstantKind {
    Infinity,
    Nan,
}

/// Float predicate kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatPredicateKind {
    IsNan,
    IsInfinite,
}

/// Float math operations (unary, natively supported by Cranelift).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatMathKind {
    Floor,
    Ceil,
    Round,
    Trunc,
    Sqrt,
}

/// Unified operation enum. Covers arithmetic, comparisons, casts, pointer ops,
/// string ops, atomics, and float intrinsics.
///
/// Arity is enforced at the Rvalue level (Op1/Op2/Op3), not here.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Op {
    // === Arithmetic ===
    Add(IntBits, Signedness),
    Sub(IntBits, Signedness),
    Mul(IntBits, Signedness),
    Div(IntBits, Signedness),
    Rem(IntBits, Signedness),
    Neg(IntBits),
    FAdd(FloatBits),
    FSub(FloatBits),
    FMul(FloatBits),
    FDiv(FloatBits),
    FNeg(FloatBits),

    // === Bitwise ===
    And(IntBits),
    Or(IntBits),
    Xor(IntBits),
    Shl(IntBits),
    Shr(IntBits, Signedness),
    Not(IntBits),
    Popcount(IntBits),
    Clz(IntBits),
    Ctz(IntBits),
    Bswap(IntBits),

    // === Integer comparison ===
    Eq(IntBits),
    Ne(IntBits),
    Lt(IntBits, Signedness),
    Le(IntBits, Signedness),
    Gt(IntBits, Signedness),
    Ge(IntBits, Signedness),

    // === Float comparison ===
    FEq(FloatBits),
    FNe(FloatBits),
    FLt(FloatBits),
    FLe(FloatBits),
    FGt(FloatBits),
    FGe(FloatBits),

    // === Boolean ===
    BoolAnd,
    BoolOr,
    BoolNot,
    BoolEq,

    // === Casts ===
    IntToFloat(IntBits, FloatBits),
    FloatToInt(FloatBits, IntBits),
    /// Signed sign-extend: (from, to)
    IntWiden(IntBits, IntBits),
    /// Unsigned zero-extend: (from, to)
    IntUnsignedWiden(IntBits, IntBits),
    /// (from, to)
    IntTruncate(IntBits, IntBits),
    /// (from, to)
    FloatWiden(FloatBits, FloatBits),
    /// (from, to)
    FloatTruncate(FloatBits, FloatBits),
    RefToImmut,

    // === Pointer ===
    /// Op2: (ptr, byte_offset) -> ptr
    PtrOffset,
    /// Op1: () -> null pointer of given type
    PtrNull(MirTy),
    /// Op1: int_address -> ptr
    PtrFromAddress(MirTy),
    /// Op1: ptr -> int_address
    PtrToAddress,
    /// Op1: ptr -> value (load through pointer)
    PtrRead(MirTy),
    /// Op2: (ptr, value) -> () (store through pointer)
    /// Carries the pointee type for aggregate copy.
    PtrWrite(MirTy),
    /// Op1: ptr -> bool
    PtrIsNull,
    /// Op1: ptr -> ptr (different pointee type)
    PtrCast(MirTy),
    /// Op1: ptr -> ptr (reinterpret bits)
    PtrBitcast(MirTy),
    /// Op1: &T -> p[T]
    RefToPtr,

    // === Memory ===
    /// Op1: () -> size in bytes
    SizeOf(MirTy),
    /// Op1: () -> alignment in bytes
    AlignOf(MirTy),
    /// Op1: count -> ptr (allocate on stack)
    StackAlloc(MirTy),

    // === String ===
    /// Op1: str -> ptr
    StrPtr,
    /// Op1: str -> i64
    StrLen,
    /// Op1: int -> str
    IntToString,

    // === Atomic ===
    /// Op2: (ptr, delta) -> old_value
    AtomicAdd,
    /// Op2: (ptr, delta) -> old_value
    AtomicSub,

    // === Float intrinsics ===
    /// Op1: () -> infinity or nan constant
    FloatConst(FloatBits, FloatConstantKind),
    /// Op1: value -> bool (is_nan / is_infinite)
    FloatPred(FloatBits, FloatPredicateKind),
    /// Op1: value -> value (floor/ceil/round/trunc/sqrt)
    FloatMath(FloatBits, FloatMathKind),
    /// Op3: a * b + c
    FloatFma(FloatBits),
    /// Op2: (magnitude, sign_source) -> value
    FloatCopysign(FloatBits),
}
