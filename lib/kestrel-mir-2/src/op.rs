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
    // Arithmetic (Op2 unless noted)
    Add(IntBits, Signedness),
    Sub(IntBits, Signedness),
    Mul(IntBits, Signedness),
    Div(IntBits, Signedness),
    Rem(IntBits, Signedness),
    Neg(IntBits), // Op1

    FAdd(FloatBits),
    FSub(FloatBits),
    FMul(FloatBits),
    FDiv(FloatBits),
    FNeg(FloatBits), // Op1

    // Bitwise (Op2 unless noted)
    And(IntBits),
    Or(IntBits),
    Xor(IntBits),
    Shl(IntBits),
    Shr(IntBits, Signedness),
    Not(IntBits),                // Op1
    Popcount(IntBits),           // Op1
    Clz(IntBits),                // Op1
    Ctz(IntBits),                // Op1
    Bswap(IntBits),              // Op1

    // Integer comparison (Op2)
    Eq(IntBits),
    Ne(IntBits),
    Lt(IntBits, Signedness),
    Le(IntBits, Signedness),
    Gt(IntBits, Signedness),
    Ge(IntBits, Signedness),

    // Float comparison (Op2)
    FEq(FloatBits),
    FNe(FloatBits),
    FLt(FloatBits),
    FLe(FloatBits),
    FGt(FloatBits),
    FGe(FloatBits),

    // Boolean (Op2 unless noted)
    BoolAnd,
    BoolOr,
    BoolNot, // Op1
    BoolEq,

    // Casts (Op1)
    IntToFloat(IntBits, FloatBits),
    FloatToInt(FloatBits, IntBits),
    IntWiden(IntBits, IntBits),
    IntUnsignedWiden(IntBits, IntBits),
    IntTruncate(IntBits, IntBits),
    FloatWiden(FloatBits, FloatBits),
    FloatTruncate(FloatBits, FloatBits),
    RefToImmut, // Op1: &var T -> &T

    // Pointer
    PtrOffset,            // Op2: (ptr, byte_offset) -> ptr
    PtrFromAddress(TyId), // Op1: int -> ptr
    PtrToAddress,         // Op1: ptr -> int
    PtrRead(TyId),        // Op1: ptr -> value
    PtrWrite(TyId),       // Op2: (ptr, value) -> ()
    PtrIsNull,            // Op1: ptr -> bool
    PtrCast(TyId),        // Op1: ptr -> ptr (different pointee)
    PtrBitcast(TyId),     // Op1: ptr -> ptr (reinterpret)
    RefToPtr,             // Op1: &T -> p[T]

    // Memory
    StackAlloc(TyId), // Op1: count -> ptr

    // String
    StrPtr, // Op1: str -> ptr
    StrLen, // Op1: str -> i64

    // Atomic
    AtomicAdd, // Op2: (ptr, delta) -> old
    AtomicSub, // Op2: (ptr, delta) -> old

    // Float intrinsics
    FloatPred(FloatBits, FloatPredicateKind), // Op1
    FloatMath(FloatBits, FloatMathKind),       // Op1
    FloatFma(FloatBits),                       // Op3: a * b + c
    FloatCopysign(FloatBits),                  // Op2: (magnitude, sign) -> value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_bits_byte_width() {
        assert_eq!(IntBits::I8.byte_width(), 1);
        assert_eq!(IntBits::I16.byte_width(), 2);
        assert_eq!(IntBits::I32.byte_width(), 4);
        assert_eq!(IntBits::I64.byte_width(), 8);
    }

    #[test]
    fn int_bits_bit_width() {
        assert_eq!(IntBits::I8.bit_width(), 8);
        assert_eq!(IntBits::I16.bit_width(), 16);
        assert_eq!(IntBits::I32.bit_width(), 32);
        assert_eq!(IntBits::I64.bit_width(), 64);
    }

    #[test]
    fn float_bits_byte_width() {
        assert_eq!(FloatBits::F16.byte_width(), 2);
        assert_eq!(FloatBits::F32.byte_width(), 4);
        assert_eq!(FloatBits::F64.byte_width(), 8);
    }

    #[test]
    fn float_bits_bit_width() {
        assert_eq!(FloatBits::F16.bit_width(), 16);
        assert_eq!(FloatBits::F32.bit_width(), 32);
        assert_eq!(FloatBits::F64.bit_width(), 64);
    }

    #[test]
    fn op_equality() {
        let a = Op::Add(IntBits::I64, Signedness::Signed);
        let b = Op::Add(IntBits::I64, Signedness::Signed);
        let c = Op::Add(IntBits::I32, Signedness::Signed);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn op_copy_semantics() {
        let a = Op::FAdd(FloatBits::F64);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn op_with_tyid() {
        use crate::TyId;
        let ty = TyId::new(5);
        let op = Op::PtrRead(ty);
        assert_eq!(op, Op::PtrRead(TyId::new(5)));
        assert_ne!(op, Op::PtrRead(TyId::new(6)));
    }

    #[test]
    fn signedness_equality() {
        assert_eq!(Signedness::Signed, Signedness::Signed);
        assert_ne!(Signedness::Signed, Signedness::Unsigned);
    }

    #[test]
    fn float_pred_kind() {
        assert_ne!(FloatPredicateKind::IsNan, FloatPredicateKind::IsInfinite);
    }

    #[test]
    fn float_math_kind_all_distinct() {
        let kinds = [
            FloatMathKind::Floor,
            FloatMathKind::Ceil,
            FloatMathKind::Round,
            FloatMathKind::Trunc,
            FloatMathKind::Sqrt,
        ];
        for i in 0..kinds.len() {
            for j in (i + 1)..kinds.len() {
                assert_ne!(kinds[i], kinds[j]);
            }
        }
    }
}
