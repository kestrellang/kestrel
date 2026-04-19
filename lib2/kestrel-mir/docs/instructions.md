# MIR Instruction Set

## Statements

```rust
pub enum StatementKind {
    /// dest = rvalue
    Assign { dest: Place, rvalue: Rvalue },

    /// call callee(args...) with optional return value
    /// Calls are always statements (not rvalues) because they have side effects.
    Call {
        dest: Option<Place>,
        callee: Callee,
        args: Vec<CallArg>,
    },

    /// Unconditionally run destructor. Used when a value is definitely valid at scope exit.
    Deinit { place: Place },

    /// Conditionally run destructor. Used when a value may have been moved in one
    /// branch but not another. The flag is a Bool local that tracks liveness.
    DeinitIf { place: Place, flag: LocalId },

    /// Set a deinit flag. Used by the deinit pass to track move status across branches.
    SetDeinitFlag { flag: LocalId, value: bool },
}

pub struct Statement {
    pub kind: StatementKind,
    pub span: Option<Span>,
}
```

Statements carry an optional span. No metadata, no priors, no inline names.

## Rvalues

The right-hand side of an assignment.

```rust
pub enum Rvalue {
    // Ownership/reference semantics — meaningful for analysis
    Move(Place),       // take ownership, invalidate source
    Copy(Place),       // copy value, source remains valid
    Ref(Place),        // immutable reference
    RefMut(Place),     // mutable reference

    // Constants
    Const(Immediate),

    // Operations by arity (see Op enum below)
    Op1 { op: Op, arg: Value },
    Op2 { op: Op, lhs: Value, rhs: Value },
    Op3 { op: Op, a: Value, b: Value, c: Value },

    // Composite construction
    Construct { ty: MirTy, fields: Vec<(String, Value)> },
    Tuple(Vec<Value>),
    EnumVariant { enum_ty: MirTy, variant: String, payload: Vec<Value> },
}
```

### Why Move/Copy/Ref/RefMut are separate from Op

These describe **how a value flows** — they have semantic meaning for the deinit pass
and ownership analysis:

- `Move` invalidates the source (deinit pass must not deinit a moved-from local)
- `Copy` preserves the source (both copies may need deinit)
- `Ref`/`RefMut` borrow without transferring ownership

They're not operations in the arithmetic sense. They're ownership annotations.

### Why Call is a statement, not an Rvalue

Calls have side effects and don't compose like pure rvalues. Making them a statement
with `dest: Option<Place>` eliminates the duplication between "void call" and "call
with return value" that existed in lib1 (`StatementKind::Call` + `Rvalue::Call`).

## Op Enum

All operations — arithmetic, comparisons, casts, pointer ops, intrinsics — are variants
of a single `Op` enum. The arity (Op1/Op2/Op3) is enforced at the Rvalue level.

```rust
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

    // === Comparison ===
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

    // === Boolean ===
    BoolAnd,
    BoolOr,
    BoolNot,
    BoolEq,

    // === Casts ===
    IntToFloat(IntBits, FloatBits),
    FloatToInt(FloatBits, IntBits),
    IntWiden(IntBits, IntBits),           // (from, to)
    IntTruncate(IntBits, IntBits),        // (from, to)
    FloatWiden(FloatBits, FloatBits),     // (from, to)
    FloatTruncate(FloatBits, FloatBits),  // (from, to)
    RefToImmut,

    // === Pointer ===
    PtrOffset,                  // Op2: (ptr, byte_offset) -> ptr
    PtrNull(MirTy),             // Op1 (unused arg) or could be Const — see note
    PtrFromAddress(MirTy),      // Op1: int_address -> ptr
    PtrToAddress,               // Op1: ptr -> int_address
    PtrRead(MirTy),             // Op1: ptr -> value
    PtrWrite,                   // Op2: (ptr, value) -> ()
    PtrIsNull,                  // Op1: ptr -> bool
    PtrCast(MirTy),             // Op1: ptr -> ptr (different pointee type)
    PtrBitcast(MirTy),          // Op1: ptr -> ptr (reinterpret)
    RefToPtr,                   // Op1: &T -> p[T]

    // === Memory ===
    SizeOf(MirTy),              // Op1 (unused arg) — size in bytes
    AlignOf(MirTy),             // Op1 (unused arg) — alignment in bytes
    StackAlloc(MirTy),          // Op1: count -> ptr (allocate on stack)

    // === String ===
    StrPtr,                     // Op1: str -> ptr
    StrLen,                     // Op1: str -> i64
    IntToString,                // Op1: int -> str

    // === Atomic ===
    AtomicAdd,                  // Op2: (ptr, delta) -> old_value
    AtomicSub,                  // Op2: (ptr, delta) -> old_value

    // === Float intrinsics ===
    FloatConst(FloatBits, FloatConstantKind),   // Op1 (unused) — infinity/nan
    FloatPred(FloatBits, FloatPredicateKind),   // Op1: is_nan/is_infinite
    FloatMath(FloatBits, FloatMathKind),         // Op1: floor/ceil/round/trunc/sqrt
    FloatFma(FloatBits),                         // Op3: a * b + c
    FloatCopysign(FloatBits),                    // Op2: (magnitude, sign_source)

    // === Callable ===
    ApplyPartial(Entity),       // OpN: captures... -> thick_callable
}

pub enum Signedness { Signed, Unsigned }

pub enum IntBits { I8, I16, I32, I64 }

pub enum FloatBits { F16, F32, F64 }

pub enum FloatConstantKind { Infinity, Nan }

pub enum FloatPredicateKind { IsNan, IsInfinite }

pub enum FloatMathKind { Floor, Ceil, Round, Trunc, Sqrt }
```

### Note on nullary ops

`PtrNull`, `SizeOf`, `AlignOf`, and `FloatConst` don't take meaningful arguments but
are encoded as `Op1` for uniformity. Alternatively, these could be `Const(Immediate)`
variants. The current encoding prioritizes keeping `Immediate` minimal (just literals
and references).

## Callee

What's being called. Self-type is always explicit — no inference at codegen time.

```rust
pub enum Callee {
    /// Direct call to a known function
    Direct {
        func: Entity,
        type_args: Vec<MirTy>,
        self_type: Option<MirTy>,  // explicit for methods
    },

    /// Thin function pointer call (no environment)
    Thin(Place),

    /// Thick callable call (has environment pointer)
    Thick(Place),

    /// Witness method dispatch (resolved at monomorphization)
    Witness {
        protocol: Entity,
        method: String,
        self_type: MirTy,
        method_type_args: Vec<MirTy>,
    },
}
```

## CallArg

How an argument is passed. Based on parameter access mode and type copyability.

```rust
pub struct CallArg {
    pub value: Value,
    pub mode: PassingMode,
}

pub enum PassingMode {
    Ref,     // immutable borrow (default)
    MutRef,  // mutable borrow
    Copy,    // value copied, original retained
    Move,    // value moved, original invalidated
}
```

## Place

A path to a memory location. No metadata — just the path.

```rust
pub enum Place {
    Local(LocalId),
    Global(Entity),
    Field { parent: Box<Place>, name: String },
    Index { parent: Box<Place>, index: usize },
    Downcast { parent: Box<Place>, variant: String },
    Deref(Box<Place>),
}
```

Convenience methods:

```rust
impl Place {
    fn local(id: LocalId) -> Self;
    fn global(entity: Entity) -> Self;
    fn field(self, name: &str) -> Self;   // chainable projection
    fn index(self, i: usize) -> Self;
    fn downcast(self, variant: &str) -> Self;
    fn deref(self) -> Self;
    fn root_local(&self) -> Option<LocalId>;  // returns None for globals
}
```

## Value

Either a place or a constant.

```rust
pub enum Value {
    Place(Place),
    Immediate(Immediate),
}
```

No `Unreachable` variant. Diverging expressions (return/break/continue) are handled
at the statement/terminator level, not embedded in values.

## Immediate

Constants and references.

```rust
pub struct Immediate {
    pub kind: ImmediateKind,
}

pub enum ImmediateKind {
    IntLiteral { bits: IntBits, value: i128 },
    FloatLiteral { bits: FloatBits, value: f64 },
    BoolLiteral(bool),
    StringLiteral(String),
    StringPointer(String),
    Unit,
    FunctionRef { func: Entity, type_args: Vec<MirTy> },
    WitnessMethod { protocol: Entity, method: String, for_type: MirTy },
    NullPtr(MirTy),
    Error,
}
```

No metadata, no inline names. Just the value.

## Terminators

How a block exits. Non-optional — every block must have exactly one.

```rust
pub struct Terminator {
    pub kind: TerminatorKind,
    pub span: Option<Span>,
}

pub enum TerminatorKind {
    Return(Value),
    Jump(BlockId),
    Branch {
        condition: Value,
        then_block: BlockId,
        else_block: BlockId,
    },
    Switch {
        discriminant: Place,
        cases: Vec<(String, BlockId)>,
    },
    Panic(String),
    Unreachable,
}
```

`Unreachable` is a terminator (the block diverged), not a value. This is the only
place divergence is represented.
