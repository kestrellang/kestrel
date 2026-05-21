# Instruction Set

The core IR: operands, rvalues, statements, terminators, basic blocks.

## Operand

Mode-free. Just "what" is being read — a place or a constant.

```rust
enum Operand {
    Place(Place),
    Const(Immediate),
}
```

Operands carry no ownership or borrow semantics. The mode lives on the
use site (Rvalue::Use, compound rvalue fields, call arguments).

## UseMode

Ownership transfer for value positions: assignments and compound rvalue fields.

```rust
enum UseMode {
    Copy,  // bitwise read, source retained
    Move,  // ownership transfer, source dead after
}
```

UseMode appears on:
- `Rvalue::Use(Operand, UseMode)`
- Compound rvalue operands: Construct, Tuple, EnumVariant, ArrayLiteral, ApplyPartial

UseMode does NOT include Ref/RefMut. Reference creation is a separate Rvalue
variant (Ref/RefMut) that produces a Pointer(T) — a different operation from
copying or moving a value.

## ArgMode

Calling convention at call sites. Superset of UseMode.

```rust
enum ArgMode {
    Copy,    // pass by value, source retained
    Move,    // pass by value, source consumed
    Ref,     // create ephemeral &, pass pointer
    RefMut,  // create ephemeral &var, pass pointer
}
```

ArgMode appears only on call arguments: `Call { args: Vec<(Operand, ArgMode)> }`.

Ref/RefMut here means "take an ephemeral reference for the duration of this
call." This avoids materializing a temp for every borrow-mode argument —
critical because borrow is Kestrel's default calling convention.

**Verifier rule:** `ArgMode::Ref` and `ArgMode::RefMut` require the operand
to be `Operand::Place(_)`. You cannot take a reference of a constant.
`(Operand::Const(_), ArgMode::Ref)` is a verifier error.

### Why two mode types

UseMode and ArgMode encode different semantic categories:

- **UseMode** (Copy|Move): value ownership transfer. Used in assignments
  and composite construction. The question is "does the source survive?"
- **ArgMode** (Copy|Move|Ref|RefMut): calling convention at a call site.
  Ref/RefMut create ephemeral references — a fundamentally different
  operation from value transfer.

Ref on a Construct field is nonsensical (Kestrel has no user-facing reference
types in structs). Two types make this unrepresentable at compile time instead
of requiring verifier enforcement.

For drop elaboration: UseMode::Move is a kill, UseMode::Copy is not.
ArgMode::Move is a kill, everything else is not. Clean match in both cases.

For a future borrow checker: call-scoped borrows (ArgMode::Ref) have trivially
bounded lifetimes. Standalone refs (Rvalue::Ref) flow into struct fields or
closures with complex lifetimes. Keeping them as separate representations
preserves this distinction.

## Rvalue

The right-hand side of an assignment.

```rust
enum Rvalue {
    // === Value transfer ===
    Use(Operand, UseMode),

    // === Reference creation ===
    Ref(Place),          // &place → produces Pointer(T)
    RefMut(Place),       // &var place → produces Pointer(T)

    // === Operations (always read, never consume) ===
    Op1 { op: Op, arg: Operand },
    Op2 { op: Op, lhs: Operand, rhs: Operand },
    Op3 { op: Op, a: Operand, b: Operand, c: Operand },

    // === Composite construction ===
    Construct { ty: TyId, fields: Vec<(FieldIdx, Operand, UseMode)> },
    Tuple(Vec<(Operand, UseMode)>),
    EnumVariant { enum_ty: TyId, variant: VariantIdx, payload: Vec<(Operand, UseMode)> },
    ArrayLiteral { element_ty: TyId, values: Vec<(Operand, UseMode)> },
    ApplyPartial { func: Entity, captures: Vec<(Operand, UseMode)> },
}
```

### Design notes

**Use is the bridge.** `Use(Operand, UseMode)` is the only way to assign a
bare value to a place. It replaces the old duplicated Copy/Move/Ref/RefMut
variants that existed on both Value and Rvalue.

**Ref/RefMut take Place, not Operand.** You can't take a reference of a
constant. Structurally distinct from Use.

**Op operands are bare.** Arithmetic reads without consuming. `Add(x, y)`
doesn't move x or y. No mode needed.

**Compound operands carry UseMode.** Drop elaboration needs to know which
operands are kills without re-deriving from types. Construct/Tuple/etc.
carry UseMode per operand — Move means the source is dead.

**ApplyPartial captures use UseMode.** Borrowed captures are materialized
as ref temps before the ApplyPartial: `_ref = ref x; apply_partial(move _ref)`.
Closures are rare (~1-2 per function), so the extra temp is acceptable.
This keeps all compound rvalue operands on the same UseMode type.

**ApplyPartial inherits type context.** The `func: Entity` on ApplyPartial
identifies the closure's call function. Type args and self_type are NOT
stored on ApplyPartial — they are inherited from the enclosing function's
scope. During monomorphization, the monomorphizer resolves the closure's
type args from the enclosing function's substitution map.

### Operand traversal

Rvalue should expose `operands()` and `operands_mut()` iterators that yield
all contained operands regardless of variant. This replaces the per-variant
pattern matching that currently exists in every pass:

```rust
impl Rvalue {
    fn operands(&self) -> impl Iterator<Item = &Operand>;
    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand>;
    fn operands_with_mode(&self) -> impl Iterator<Item = (&Operand, Option<UseMode>)>;
}
```

Op variants yield operands with `None` mode. Compound variants yield
operands with `Some(mode)`.

## Statement

```rust
enum StatementKind {
    /// dest = rvalue
    Assign { dest: Place, rvalue: Rvalue },

    /// call callee(args...) with optional return destination.
    /// Calls are always statements (not rvalues) because of side effects.
    Call {
        dest: Option<Place>,
        callee: Callee,
        args: Vec<(Operand, ArgMode)>,
    },

    /// Unconditionally call drop shim. Emitted by drop elaboration only.
    Drop { place: Place },

    /// Conditionally call drop shim, guarded by a bool flag.
    /// The flag tracks whether the value is live at this point.
    DropIf { place: Place, flag: LocalId },

    /// Set a drop flag. true = live (needs drop), false = dead (skip).
    SetDropFlag { flag: LocalId, value: bool },

    /// Mark a local as entering scope (loop re-entry).
    /// Drop elaboration reads this as "init-state resets to dead here."
    ScopeLive(LocalId),
}

struct Statement {
    kind: StatementKind,
    span: Option<Span>,
}
```

### Why Call is a statement

Calls have side effects and don't compose. Making Call a statement with
`dest: Option<Place>` handles both void and value-returning calls uniformly.

### Drop flag convention

`true` = live (value exists, needs drop). `false` = dead (moved or not
initialized, skip drop). One convention, no inversion between passes.

## Callee

What's being called. Self-type is always explicit.

```rust
enum Callee {
    /// Direct call to a known generic function (pre-monomorphization).
    Direct {
        func: Entity,
        type_args: Vec<TyId>,
        self_type: Option<TyId>,
    },

    /// Direct call to a resolved monomorphic function (post-monomorphization).
    Resolved(MonoFuncId),

    /// Thin function pointer (no environment).
    Thin(Place),

    /// Thick callable (has environment pointer).
    Thick(Place),

    /// Witness method dispatch — resolved during monomorphization.
    Witness {
        protocol: Entity,
        method: WitnessMethodKey,
        self_type: TyId,
        method_type_args: Vec<TyId>,
    },
}
```

Generic MIR uses `Direct`, `Thin`, `Thick`, `Witness`. The generic
verifier rejects `Resolved`. After monomorphization, `Direct` and
`Witness` are rewritten to `Resolved(MonoFuncId)`. The mono verifier
rejects `Direct` and `Witness`. Codegen sees `Resolved`, `Thin`, `Thick`
— three arms, no type_args, no witness resolution.

## Terminator

How a block exits. Non-optional — every block must have exactly one.

```rust
enum TerminatorKind {
    Return(Operand),          // implicitly moves the operand to the caller
    Jump(BlockId),
    Branch {
        condition: Operand,
        then_block: BlockId,
        else_block: BlockId,
    },
    Switch {
        discriminant: Place,
        cases: Vec<(SwitchCase, BlockId)>,
    },
    Panic(String),
    Unreachable,
}

enum SwitchCase {
    Wildcard,                         // default/else arm
    Variant(VariantIdx),
    Bool(bool),
    IntLiteral(i64),
    IntRange { start: i64, end: i64 },
    CharLiteral(u32),
    CharRange { start: u32, end: u32 },
}
```

`Return(Operand)` — the operand is implicitly moved (ownership transfers
to the caller). Clone elaboration inserts a clone before the return when
a Clone-typed user local is being returned and the local is still live
(has other uses that need the original value to survive). If the local's
only remaining use IS the return, it's moved directly — no clone needed.

`SwitchCase::Variant(VariantIdx)` — index, not string. No display-name
leak bugs, no string comparison at codegen.

**Switch → Downcast invariant:** In the successor block of a
`SwitchCase::Variant(idx)`, the discriminant place may be refined via
`PlaceElem::Downcast(idx)`. The verifier checks that Downcast projections
only appear in blocks dominated by a Switch case's successor where that
successor is reached by exactly one variant. If two variants share a
successor block, no Downcast is valid there — the variant is ambiguous.

## BasicBlock

```rust
struct BasicBlock {
    stmts: Vec<Statement>,
    terminator: Terminator,
}
```

Terminator is never optional. Blocks under construction use
`Terminator::unreachable()` as a placeholder.

## MirBody

```rust
struct MirBody {
    locals: Vec<LocalDef>,
    blocks: Vec<BasicBlock>,
    param_count: usize,
    entry: BlockId,
    local_scopes: HashMap<LocalId, ScopeId>,
    failure_return_blocks: HashSet<BlockId>,
}

struct LocalDef {
    name: String,
    ty: TyId,
}

enum ScopeId {
    Function,
    Loop { header: BlockId, exit: BlockId },
}
```

Parameters are `locals[0..param_count]`. No `borrowed` flag — borrowed
closure captures have type `Pointer(T)`, which is Bitwise, so they are
automatically excluded from drop tracking by the type system.

## Immediate

Constants and references.

```rust
enum ImmediateKind {
    IntLiteral { bits: IntBits, value: i128 },
    FloatLiteral { bits: FloatBits, value: f64 },
    BoolLiteral(bool),
    StringLiteral(String),
    StringPointer(String),
    Unit,
    FunctionRef { func: Entity, type_args: Vec<TyId>, self_type: Option<TyId> },
    NullPtr(TyId),
    SizeOf(TyId),
    AlignOf(TyId),
    FloatInfinity(FloatBits),
    FloatNan(FloatBits),
    Error,
}
```

`FunctionRef` uses `Entity` in generic MIR. During monomorphization, the
monomorphizer resolves function references to a `MonoFunctionRef(MonoFuncId)`
Immediate variant — in MonoModule bodies, function references point into
the module's function table, not the ECS. `PlaceBase::Global(Entity)` for
statics is NOT rewritten — statics are module-global and codegen resolves
them by entity from `MonoModule.statics`.

SizeOf, AlignOf, NullPtr, FloatInfinity, FloatNan are compile-time
constants, not operations. They replace the old Op1-with-dummy-argument
hack. After monomorphization, they can be folded to concrete values.

## Op

All operations — arithmetic, comparisons, casts, pointer ops, intrinsics.
Arity is enforced at the Rvalue level (Op1/Op2/Op3).

```rust
enum Op {
    // === Arithmetic (Op2 unless noted) ===
    Add(IntBits, Signedness), Sub(IntBits, Signedness),
    Mul(IntBits, Signedness), Div(IntBits, Signedness),
    Rem(IntBits, Signedness), Neg(IntBits),          // Op1
    FAdd(FloatBits), FSub(FloatBits), FMul(FloatBits),
    FDiv(FloatBits), FNeg(FloatBits),                // Op1

    // === Bitwise (Op2 unless noted) ===
    And(IntBits), Or(IntBits), Xor(IntBits),
    Shl(IntBits), Shr(IntBits, Signedness),
    Not(IntBits),                                    // Op1
    Popcount(IntBits), Clz(IntBits), Ctz(IntBits),  // Op1
    Bswap(IntBits),                                  // Op1

    // === Integer comparison (Op2) ===
    Eq(IntBits), Ne(IntBits),
    Lt(IntBits, Signedness), Le(IntBits, Signedness),
    Gt(IntBits, Signedness), Ge(IntBits, Signedness),

    // === Float comparison (Op2) ===
    FEq(FloatBits), FNe(FloatBits),
    FLt(FloatBits), FLe(FloatBits),
    FGt(FloatBits), FGe(FloatBits),

    // === Boolean (Op2 unless noted) ===
    BoolAnd, BoolOr, BoolNot,                        // BoolNot is Op1
    BoolEq,

    // === Casts (Op1) ===
    IntToFloat(IntBits, FloatBits),
    FloatToInt(FloatBits, IntBits),
    IntWiden(IntBits, IntBits),                       // signed sign-extend (from, to)
    IntUnsignedWiden(IntBits, IntBits),               // unsigned zero-extend
    IntTruncate(IntBits, IntBits),
    FloatWiden(FloatBits, FloatBits),
    FloatTruncate(FloatBits, FloatBits),
    RefToImmut,                                       // &var T → &T

    // === Pointer ===
    PtrOffset,                                        // Op2: (ptr, byte_offset) → ptr
    PtrFromAddress(TyId),                             // Op1: int → ptr
    PtrToAddress,                                     // Op1: ptr → int
    PtrRead(TyId),                                    // Op1: ptr → value
    PtrWrite(TyId),                                   // Op2: (ptr, value) → ()
    PtrIsNull,                                        // Op1: ptr → bool
    PtrCast(TyId),                                    // Op1: ptr → ptr (different pointee)
    PtrBitcast(TyId),                                 // Op1: ptr → ptr (reinterpret)
    RefToPtr,                                         // Op1: &T → p[T]

    // === Memory ===
    StackAlloc(TyId),                                 // Op1: count → ptr

    // === String ===
    StrPtr,                                           // Op1: str → ptr
    StrLen,                                           // Op1: str → i64

    // === Atomic ===
    AtomicAdd,                                        // Op2: (ptr, delta) → old
    AtomicSub,                                        // Op2: (ptr, delta) → old

    // === Float intrinsics ===
    FloatPred(FloatBits, FloatPredicateKind),          // Op1: is_nan / is_infinite
    FloatMath(FloatBits, FloatMathKind),               // Op1: floor/ceil/round/trunc/sqrt
    FloatFma(FloatBits),                               // Op3: a * b + c
    FloatCopysign(FloatBits),                          // Op2: (magnitude, sign) → value
}

enum IntBits { I8, I16, I32, I64 }
enum FloatBits { F16, F32, F64 }
enum Signedness { Signed, Unsigned }
enum FloatPredicateKind { IsNan, IsInfinite }
enum FloatMathKind { Floor, Ceil, Round, Trunc, Sqrt }
```

Changes from kestrel-mir-1:
- `PtrNull`, `SizeOf`, `AlignOf`, `FloatConst` moved to `ImmediateKind` (constants, not ops)
- `IntToString` removed (handled via witness call to `CustomStringConvertible`)
- Type arguments use `TyId` instead of `MirTy` (interned)

## Body construction

Bodies are built imperatively by the lowering pass using a builder pattern:

```rust
struct BodyBuilder<'a> {
    arena: &'a TyArena,
    body: MirBody,
    current_block: BlockId,
    temp_counter: u32,
}

impl BodyBuilder {
    fn new_block(&mut self) -> BlockId;
    fn switch_to(&mut self, block: BlockId);
    fn emit(&mut self, stmt: StatementKind);
    fn emit_assign(&mut self, dest: Place, rvalue: Rvalue);
    fn emit_call(&mut self, dest: Option<Place>, callee: Callee, args: Vec<(Operand, ArgMode)>);
    fn terminate(&mut self, term: TerminatorKind);
    fn fresh_temp(&mut self, ty: TyId) -> LocalId;
    fn finish(self) -> MirBody;
}
```

The builder owns the `MirBody` under construction. `emit` appends to
the current block. `terminate` seals the current block. `new_block`
creates a block and returns its ID (does not switch to it).

Lowering creates one `BodyBuilder` per function body, emits statements
in HIR traversal order, and calls `finish()` to extract the completed
body. The builder is internal to kestrel-mir-lower — it is not part of
the public MIR API.
