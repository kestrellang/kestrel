use crate::TyId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntBits {
    I8,
    I16,
    I32,
    I64,
}

impl IntBits {
    pub fn byte_width(self) -> u64 {
        match self {
            IntBits::I8 => 1,
            IntBits::I16 => 2,
            IntBits::I32 => 4,
            IntBits::I64 => 8,
        }
    }

    pub fn bit_width(self) -> u32 {
        match self {
            IntBits::I8 => 8,
            IntBits::I16 => 16,
            IntBits::I32 => 32,
            IntBits::I64 => 64,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatBits {
    F16,
    F32,
    F64,
}

impl FloatBits {
    pub fn byte_width(self) -> u64 {
        match self {
            FloatBits::F16 => 2,
            FloatBits::F32 => 4,
            FloatBits::F64 => 8,
        }
    }

    pub fn bit_width(self) -> u32 {
        match self {
            FloatBits::F16 => 16,
            FloatBits::F32 => 32,
            FloatBits::F64 => 64,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Signedness {
    Signed,
    Unsigned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatPredicateKind {
    IsNan,
    IsInfinite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatMathKind {
    Floor,
    Ceil,
    Round,
    Trunc,
    Sqrt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Op {
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

    Eq(IntBits),
    Ne(IntBits),
    Lt(IntBits, Signedness),
    Le(IntBits, Signedness),
    Gt(IntBits, Signedness),
    Ge(IntBits, Signedness),

    FEq(FloatBits),
    FNe(FloatBits),
    FLt(FloatBits),
    FLe(FloatBits),
    FGt(FloatBits),
    FGe(FloatBits),

    BoolAnd,
    BoolOr,
    BoolNot,
    BoolEq,

    IntToFloat(IntBits, FloatBits),
    FloatToInt(FloatBits, IntBits),
    IntWiden(IntBits, IntBits),
    IntUnsignedWiden(IntBits, IntBits),
    IntTruncate(IntBits, IntBits),
    FloatWiden(FloatBits, FloatBits),
    FloatTruncate(FloatBits, FloatBits),
    RefToImmut,

    PtrOffset,
    PtrFromAddress(TyId),
    PtrToAddress,
    PtrRead(TyId),
    PtrWrite(TyId),
    PtrIsNull,
    PtrNull(TyId),
    PtrTo(TyId),
    PtrCast(TyId),
    PtrBitcast(TyId),
    RefToPtr,

    SizeOf(TyId),
    AlignOf(TyId),
    StackAlloc(TyId),

    StrPtr,
    StrLen,

    AtomicAdd,
    AtomicSub,

    FloatPred(FloatBits, FloatPredicateKind),
    FloatMath(FloatBits, FloatMathKind),
    FloatFma(FloatBits),
    FloatCopysign(FloatBits),
}
