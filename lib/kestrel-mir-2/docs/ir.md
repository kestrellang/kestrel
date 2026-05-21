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
    /// Direct call to a known function.
    Direct {
        func: Entity,
        type_args: Vec<TyId>,
        self_type: Option<TyId>,
    },

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

In MonoModule, Callee is replaced by MonoCallee:

```rust
enum MonoCallee {
    Direct(MonoFuncId),
    Thin(Place),
    Thick(Place),
}
```

Three variants. No Witness (resolved), no type_args (substituted), no
self_type (baked in). Codegen matches three arms.

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
    borrowed: bool,  // closure-captured borrowed view — don't drop
}

enum ScopeId {
    Function,
    Loop { header: BlockId, exit: BlockId },
}
```

Parameters are `locals[0..param_count]`. The `borrowed` flag marks closure
captures that are borrowed views of the parent scope — drop elaboration
skips these.

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

The Op enum is unchanged from kestrel-mir. See the kestrel-mir op
documentation for the full variant list.
