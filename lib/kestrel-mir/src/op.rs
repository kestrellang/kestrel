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

/// Whether a signed `Div`/`Rem` carries its safety guards (divide-by-zero trap
/// and the `Int.MIN / -1` overflow guard). `Unchecked` skips both, emitting the
/// bare hardware divide — undefined behaviour on those edges, like C. Backs the
/// `divideUnchecked`/`moduloUnchecked` stdlib methods for hot loops where the
/// caller guarantees a valid divisor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DivGuard {
    Checked,
    Unchecked,
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
    Div(IntBits, Signedness, DivGuard),
    Rem(IntBits, Signedness, DivGuard),
    Neg(IntBits),

    // Overflow predicates: `true` if the corresponding wrapping op overflows the
    // type. Result is a Bool (like the comparison ops). Back the `*Checked`
    // stdlib helpers, which can't detect overflow reliably from the wrapped
    // result alone (e.g. `minValue * -1`).
    AddOverflows(IntBits, Signedness),
    SubOverflows(IntBits, Signedness),
    MulOverflows(IntBits, Signedness),

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

    /// Project one machine word out of a `FuncThick` value: `0` = code
    /// pointer, `1` = environment handle, `2` = retain shim, `3` = release
    /// shim (the last two exist only at the owning kinds, whose layout is 4
    /// words — see `passes/layout.rs`).
    ///
    /// The ONLY consumer is the post-mono type-erased retain/release dispatch
    /// in `mono/expand.rs`: a value of type `escaping (…) -> …` does not name
    /// its environment type, so copy/destroy load the shim pointer out of the
    /// value and call it indirectly. Codegen is a GEP + load — never pointer
    /// arithmetic on an address (kestrel-codegen-llvm/AGENTS.md).
    ClosureWord(u32),

    StrPtr,
    StrLen,

    AtomicAdd,
    AtomicSub,

    FloatPred(FloatBits, FloatPredicateKind),
    FloatMath(FloatBits, FloatMathKind),
    FloatFma(FloatBits),
    FloatCopysign(FloatBits),
    /// Reinterpret a float's bits as the same-width integer (no value conversion,
    /// pure bitcast): `f64 -> i64`, `f32 -> i32`. Unlike `FloatToInt`, the bit
    /// pattern is preserved exactly — used for exact IEEE-754 decomposition.
    FloatToBits(FloatBits),
    /// Inverse of `FloatToBits`: reinterpret a same-width integer's bits as a
    /// float (`i64 -> f64`, `i32 -> f32`).
    BitsToFloat(FloatBits),
}

/// The single canonical enumeration of the `Op` variants that carry a `TyId`.
///
/// Every consumer that needs "which ops name a type" derives from this macro —
/// `op_type` (read), `op_type_mut` (substitution), the mono collector, and the
/// mono verifier. Writing the list twice by hand is exactly how G2 happened:
/// `mono::substitute_op_type` handled all ten while `mono::collect_named_types`
/// handled zero, so a type reachable only through an op operand was never
/// seeded and silently fell back to a pointer-sized layout.
///
/// When you add an `Op` variant with a `TyId` payload, add it here — nowhere
/// else.
macro_rules! op_ty_variants {
    ($mac:ident) => {
        $mac! {
            PtrFromAddress,
            PtrRead,
            PtrWrite,
            PtrNull,
            PtrTo,
            PtrCast,
            PtrBitcast,
            SizeOf,
            AlignOf,
            StackAlloc,
        }
    };
}

macro_rules! define_op_type_accessors {
    ($($variant:ident),* $(,)?) => {
        /// The type operand an `Op` names, if it has one.
        ///
        /// Only `SizeOf`/`AlignOf` name a type that need not appear anywhere
        /// else in the body; the rest produce a `Pointer[T]` or `T` value whose
        /// own type usually re-seeds `T`. That correlation is incidental, so
        /// callers must consult this rather than relying on it.
        pub fn op_type(op: &Op) -> Option<TyId> {
            match op {
                $(Op::$variant(ty) => Some(*ty),)*
                _ => None,
            }
        }

        /// Mutable twin of [`op_type`], for monomorphization substitution.
        pub fn op_type_mut(op: &mut Op) -> Option<&mut TyId> {
            match op {
                $(Op::$variant(ty) => Some(ty),)*
                _ => None,
            }
        }
    };
}

op_ty_variants!(define_op_type_accessors);
