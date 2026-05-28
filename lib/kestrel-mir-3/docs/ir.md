# Instruction Set

The core OSSA IR: values, instructions, terminators, basic blocks.

## Value Model

Every instruction that produces a result defines a new **SSA value** with
a fixed type and ownership kind. Values are immutable once defined — you
never "reassign" a value. An `@owned` value is consumed exactly once on
every control-flow path. An `@none` value can be used freely.

```rust
/// Unique SSA value identifier. Allocated sequentially per function body.
pub struct ValueId(u32);
```

Values replace MIR-2's mutable `LocalId` slots. In MIR-2, a local could
be assigned, moved-from, reassigned, and dropped across its lifetime,
requiring dataflow analysis to reconstruct ownership state. In OSSA,
ownership is a property of the value definition — no reconstruction needed.

## Ownership

```rust
pub enum Ownership {
    /// The value is independently owned. It MUST be consumed exactly
    /// once on every control-flow path: by destroy_value, a consuming
    /// call argument, return, store_init/store_assign, or forwarding into
    /// another @owned instruction. Leaking is a verifier error.
    /// Double-consume is a verifier error.
    Owned,

    /// The value is borrowed from an enclosing @owned value. It is
    /// valid between begin_borrow and end_borrow. The holder cannot
    /// consume it (no destroy, no move, no store). Multiple reads
    /// are allowed.
    Guaranteed,

    /// Trivial value. No ownership tracking, no cleanup. Can be used
    /// any number of times, duplicated freely, ignored at scope exit.
    /// All bitwise-copyable types: Int64, Bool, raw pointers, thin
    /// function refs, unit.
    None,
}
```

No `@unowned` kind (unlike Swift SIL). Kestrel is single-threaded and
has no reference types in the language. No scenario where a value could
be invalidated while a non-owning reference exists.

### Assignment Rule

Ownership is determined by type at value creation:

```rust
fn ownership_for_type(ty: TyId, module: &MirModule) -> Ownership {
    match module.copy_behavior(ty) {
        CopyBehavior::Bitwise => Ownership::None,
        CopyBehavior::Clone(_) | CopyBehavior::None => Ownership::Owned,
    }
}
```

Trivial types are `@none`. Everything else (Clone-copyable and affine)
is `@owned`. Borrows (`@guaranteed`) are created explicitly by
`begin_borrow` instructions — they never arise from type information.

## Value Definition

```rust
pub struct ValueDef {
    pub ty: TyId,
    pub ownership: Ownership,
    /// For @guaranteed values: which @owned value is frozen by this
    /// borrow. None for @owned and @none values. When a @guaranteed
    /// value flows through a block arg or forwarding extraction, the
    /// new value inherits the same provenance. The verifier uses this
    /// to enforce "source not consumed while any value with this
    /// provenance is live."
    pub borrow_source: Option<ValueId>,
    /// Source location of the defining expression, when known. Metadata
    /// only — gives verifier ICEs a precise span when the error fires at
    /// a block boundary (no triggering instruction). Synthetic values
    /// (shim/thunk temporaries) carry None.
    pub span: Option<Span>,
}
```

The function body maintains a flat `Vec<ValueDef>` indexed by `ValueId`.
Every instruction that produces a result allocates a new `ValueId`
before emission.

### Spans: instructions carry them, types can't

Source spans live on **instructions** (`Instruction.span`), **terminators**
(`Terminator.span`), and now **value definitions** (`ValueDef.span`). They do
**not** live on types: `TyId`s are interned in a content-addressed `TyArena`
(one `TyId` per structural type, shared module-wide), so a type has no single
source location to attach. Any diagnostic needing a span for a *type* inherits
it from the enclosing instruction, or — for a value — from that value's
defining span.

`ValueDef.span` is deliberately **excluded from `PartialEq`** (hand-written
impl): it is metadata, never identity. Two values that agree on
type/ownership/borrow_source are equal regardless of span, so adding spans
cannot perturb dedup or any equality-keyed pass.

The lowerer stamps the current expr/stmt span into every value it allocates
(`OssaBodyCtx::alloc_value` / `alloc_guaranteed`, mirroring `push_inst`). The
OSSA verifier's `err_val` helper prefers the triggering instruction's span and
falls back to the offending value's defining span — this is what gives
block-exit errors ("@owned value never consumed", "borrow still active at block
exit") a real location instead of collapsing to the function `DeclSpan`.

## Basic Block

```rust
pub struct BasicBlock {
    /// Block parameters — phi-like values received from predecessors.
    /// Each predecessor's jump/branch must pass matching arguments.
    pub params: Vec<BlockParam>,

    /// Sequential instructions.
    pub insts: Vec<Instruction>,

    /// How the block exits. Non-optional.
    pub terminator: Terminator,
}

pub struct BlockParam {
    pub value: ValueId,
    pub ty: TyId,
    pub ownership: Ownership,
}
```

Unlike MIR-2's `BasicBlock { stmts, terminator }`, OSSA blocks have
typed, ownership-annotated parameters. This replaces MIR-2's drop flags
(`DropIf` + `SetDropFlag`) at control-flow merge points.

Block parameters accept all three ownership kinds: `@owned`,
`@guaranteed`, and `@none`. Today the lowerer only emits `@owned` and
`@none` block params (borrows are call-scoped). But the IR and verifier
accept `@guaranteed` block params so that lexical lifetimes can be
added later without touching the block model. A `@guaranteed` block
param carries borrow provenance — the verifier tracks which `@owned`
source is frozen and ensures it isn't consumed while any value with
that provenance is live.

## Instruction

```rust
pub struct Instruction {
    pub kind: InstKind,
    pub span: Option<Span>,
}
```

## InstKind

### Value Lifecycle

```rust
/// Produce a deep copy of an @owned or @guaranteed value.
/// For Clone types: lowers to a clone() witness call at codegen.
/// For Bitwise types: should not appear (verifier rejects).
/// For Affine types: should not appear (verifier rejects).
/// Result is always @owned.
CopyValue {
    result: ValueId,
    operand: ValueId,
}

/// Forward an @owned value with a new name. The operand is consumed.
/// This is a renaming device — no runtime cost. Useful for the
/// verifier (tracks consume points) and for canonicalization.
MoveValue {
    result: ValueId,
    operand: ValueId,
}

/// End the lifetime of an @owned value. If the type is droppable,
/// calls the drop shim (__drop$T). If trivial — should not appear
/// (verifier rejects: use @none values, don't destroy them).
/// This is the ONLY way to end an @owned value's lifetime without
/// transferring it elsewhere.
DestroyValue {
    operand: ValueId,
}
```

### Borrowing

```rust
/// Begin a shared borrow. The operand must be @owned. Result is
/// @guaranteed. The operand remains valid but cannot be consumed
/// until end_borrow.
BeginBorrow {
    result: ValueId,
    operand: ValueId,
}

/// End a shared borrow. The @guaranteed value dies. The borrowed-from
/// @owned value is released (can be consumed again).
EndBorrow {
    operand: ValueId,
}

/// Begin a mutable borrow. The operand must be @owned. Result is
/// @guaranteed but grants mutable access. The operand is frozen:
/// cannot be used (read, moved, destroyed) until end_mut_borrow.
BeginMutBorrow {
    result: ValueId,
    operand: ValueId,
}

/// End a mutable borrow. Unfreezes the borrowed-from value.
EndMutBorrow {
    operand: ValueId,
}
```

Kestrel borrows are always call-scoped — they never escape the function
or flow into struct fields. This means borrow scopes are always local
and the borrow verifier is simple: check nesting and no-consume-during.

### Memory Access

```rust
/// Load a trivial value from a pointer address. Result must be @none.
/// Non-trivial values must use copy_addr, take, or begin_borrow_addr so
/// ownership is explicit.
Load {
    result: ValueId,
    address: ValueId,
}

/// Produce an independent owned copy of a non-trivial value in memory.
/// Valid only for Clone-copyable types. The memory location remains
/// initialized and still owns its value.
CopyAddr {
    result: ValueId,
    address: ValueId,
    ty: TyId,
}

/// Move a non-trivial value out of memory. Result is @owned and the
/// memory location becomes uninitialized until store_init initializes it.
Take {
    result: ValueId,
    address: ValueId,
    ty: TyId,
}

/// Begin a shared borrow of an initialized memory location. Result is
/// @guaranteed and compiles to the address value. The memory location
/// cannot be moved out or overwritten until end_borrow.
BeginBorrowAddr {
    result: ValueId,
    address: ValueId,
    ty: TyId,
}

/// Begin a mutable borrow of an initialized memory location. Result is
/// @guaranteed and compiles to the address value. The memory location
/// cannot be read, moved out, or overwritten until end_mut_borrow.
BeginMutBorrowAddr {
    result: ValueId,
    address: ValueId,
    ty: TyId,
}

/// Initialize an uninitialized memory location. The value is consumed
/// and ownership transfers to the memory location.
StoreInit {
    address: ValueId,
    value: ValueId,
}

/// Assign into an already-initialized memory location. The old contents
/// are destroyed in place, then the new value is consumed into memory.
StoreAssign {
    address: ValueId,
    value: ValueId,
}

/// Destroy the initialized value currently stored at an address, leaving
/// the memory location uninitialized.
DestroyAddr {
    address: ValueId,
    ty: TyId,
}
```

OSSA tracks two ownership domains:
- SSA values, through `@owned` / `@guaranteed` / `@none`.
- Memory locations, through explicit initialized/uninitialized effects on
  `Take`, `StoreInit`, `StoreAssign`, and `DestroyAddr`.

This separation prevents a non-trivial `load` from accidentally creating
two independent owners for the same memory. If the source is memory, the
lowerer must choose whether it is reading a trivial value, copying,
borrowing, moving out, initializing, assigning, or destroying.

### Enum Discriminant

```rust
/// Read the integer discriminant tag from an enum value without
/// consuming it. Operand can be @owned or @guaranteed. Result is
/// always @none (the tag is a trivial integer). This is the only
/// non-consuming read allowed on an @owned enum — it exists
/// specifically to support Switch lowering.
Discriminant {
    result: ValueId,
    operand: ValueId,
}
```

### Computation

```rust
/// Unary operation. Arg must be @none. Result is @none.
Op1 { result: ValueId, op: Op, arg: ValueId }

/// Binary operation. Both args must be @none. Result is @none.
Op2 { result: ValueId, op: Op, lhs: ValueId, rhs: ValueId }

/// Ternary operation (e.g., fma). All args must be @none. Result is @none.
Op3 { result: ValueId, op: Op, a: ValueId, b: ValueId, c: ValueId }
```

All ops produce trivial results. All operands must be @none. To use a
field of an @owned struct in an op (e.g., a raw pointer for pointer
arithmetic), first borrow the struct and extract the trivial field
from the borrow.

### Constants

```rust
/// Immediate constant value. Always @none.
Literal { result: ValueId, value: Immediate }

/// Reference to a global/static. Always @none (pointer).
GlobalRef { result: ValueId, entity: Entity }
```

### Aggregates — Construction (Forwarding)

Forwarding instructions: result ownership is determined by operand
ownership. Consumes @owned operands, reads @none operands.

```rust
/// Construct a struct from fields.
Struct {
    result: ValueId,
    ty: TyId,
    fields: Vec<(FieldIdx, ValueId)>,
}

/// Construct a tuple.
Tuple {
    result: ValueId,
    elements: Vec<ValueId>,
}

/// Construct an enum variant.
Enum {
    result: ValueId,
    enum_ty: TyId,
    variant: VariantIdx,
    payload: Vec<ValueId>,
}

/// Construct an array literal.
Array {
    result: ValueId,
    element_ty: TyId,
    elements: Vec<ValueId>,
}
```

### Aggregates — Destructuring (Forwarding)

```rust
/// Extract a struct field. If the operand is @guaranteed, this is a
/// non-consuming projection and the result is @guaranteed. If the operand
/// is @owned, this consumes the entire aggregate and is only legal when
/// all unextracted fields are @none. Use DestructureStruct when multiple
/// fields are needed or when unextracted fields require explicit destroy.
StructExtract {
    result: ValueId,
    operand: ValueId,
    field: FieldIdx,
}

/// Extract a tuple element. Same ownership rule as StructExtract.
TupleExtract {
    result: ValueId,
    operand: ValueId,
    index: u32,
}

/// Extract a single enum payload field (used after Switch determines
/// the active variant). Same ownership rules as StructExtract: if the
/// enum is @owned, this consumes it and all unextracted payload fields
/// must be @none. Use DestructureEnum when multiple owned payload fields
/// are needed.
EnumPayload {
    result: ValueId,
    operand: ValueId,
    variant: VariantIdx,
    field: FieldIdx,
}

/// Destructure a struct into all its fields at once. Consumes the struct.
/// Each result is @owned (for non-trivial fields) or @none (for trivial).
DestructureStruct {
    results: Vec<ValueId>,
    operand: ValueId,
}

/// Destructure a tuple into all elements.
DestructureTuple {
    results: Vec<ValueId>,
    operand: ValueId,
}

/// Destructure an enum variant's payload into all its fields at once.
/// Consumes the enum. Must only be used in a block dominated by a
/// Switch case that determined the variant. Each result is @owned or
/// @none based on the field's type.
DestructureEnum {
    results: Vec<ValueId>,
    operand: ValueId,
    variant: VariantIdx,
}
```

### Calls

```rust
/// Function call.
Call {
    result: Option<ValueId>,
    callee: Callee,
    args: Vec<CallArg>,
}

pub struct CallArg {
    pub value: ValueId,
    pub convention: ParamConvention,
}
```

Convention on `CallArg` determines ownership at the call site:
- `Consuming`: value is consumed by the call (ownership transfer).
- `Borrow`: caller wraps in begin_borrow/end_borrow around the call, or
  begin_borrow_addr/end_borrow when the argument is an address-backed place.
- `MutBorrow`: caller wraps in begin_mut_borrow/end_mut_borrow, or
  begin_mut_borrow_addr/end_mut_borrow for address-backed places.

### Closures

```rust
/// Create a thick closure (partial application). Captures are
/// consumed (if @owned) or copied trivially (if @none). Borrow
/// captures are not supported directly — the lowerer materializes
/// borrowed captures as ref temps (Pointer(T), which is @none)
/// before ApplyPartial. This is consistent with "borrows are
/// call-scoped only" — closure environments own their captures.
ApplyPartial {
    result: ValueId,
    func: Entity,
    captures: Vec<ValueId>,
}
```

### Address Projection

```rust
/// Compute the address of a struct field from a base struct address.
/// Base must be @none (a pointer from Uninit, a parameter address, or
/// another FieldAddr). Result is @none (a sub-address). The verifier
/// uses FieldAddr to track per-field initialized/uninitialized state
/// for Uninit allocations — it knows which field is being addressed
/// without depending on byte-level layout information.
///
/// At codegen, compiles to base + field_offset(ty, field).
FieldAddr {
    result: ValueId,
    base: ValueId,
    ty: TyId,
    field: FieldIdx,
}
```

### Special

```rust
/// Declare an uninitialized memory location. Result is always @none
/// (a pointer/address). The memory it points to starts uninitialized
/// and must be initialized via StoreInit before use. The verifier
/// tracks per-field initialized/uninitialized state through FieldAddr
/// projections (see "Address State" in passes.md).
Uninit { result: ValueId, ty: TyId }
```

## Callee

Same call-target model as MIR-2, with `Thin` and `Thick` pointing at
`ValueId`s instead of MIR-2 `Place`s:

```rust
pub enum Callee {
    Direct {
        func: Entity,
        type_args: Vec<TyId>,
        self_type: Option<TyId>,
    },
    Resolved(MonoFuncId),
    Thin(ValueId),
    Thick(ValueId),
    Witness {
        protocol: Entity,
        method: WitnessMethodKey,
        self_type: TyId,
        method_type_args: Vec<TyId>,
    },
}
```

## Terminator

```rust
pub enum TerminatorKind {
    /// Return a value to the caller. @owned values transfer ownership.
    /// For void-returning functions, the lowerer emits a unit Literal
    /// and passes it here (unit is @none).
    /// Functions returning `!` (Never) end with Panic or Unreachable,
    /// never Return.
    Return(ValueId),

    /// Unconditional jump with block arguments.
    Jump {
        target: BlockId,
        args: Vec<ValueId>,
    },

    /// Conditional branch. Condition must be @none (Bool).
    Branch {
        condition: ValueId,
        then_block: BlockId,
        then_args: Vec<ValueId>,
        else_block: BlockId,
        else_args: Vec<ValueId>,
    },

    /// Multi-way switch with block arguments per case.
    /// The discriminant must be @none (an integer tag, bool, or char).
    /// For @owned enums, the lowerer uses the Discriminant instruction
    /// to extract the @none tag without consuming the enum, then passes
    /// the @owned enum itself as a block argument in the matching
    /// SwitchArm. The arm block receives the enum via a block param
    /// and uses EnumPayload or DestructureEnum to extract the payload.
    Switch {
        discriminant: ValueId,
        cases: Vec<SwitchArm>,
    },

    Panic(String),
    Unreachable,
}

pub struct SwitchArm {
    pub pattern: SwitchCase,
    pub target: BlockId,
    pub args: Vec<ValueId>,
}
```

`SwitchCase` is unchanged from MIR-2: `Wildcard`, `Variant(VariantIdx)`,
`Bool(bool)`, `IntLiteral(i64)`, `IntRange { start, end }`,
`CharLiteral(u32)`, `CharRange { start, end }`.

## Body

```rust
pub struct OssaBody {
    /// All SSA values in this function, indexed by ValueId.
    pub values: Vec<ValueDef>,
    /// All basic blocks, indexed by BlockId.
    pub blocks: Vec<BasicBlock>,
    /// Entry block.
    pub entry: BlockId,
    /// Number of function parameters (first N values).
    pub param_count: usize,
}
```

## What Changed from MIR-2

| MIR-2 | OSSA | Why |
|-------|------|-----|
| `LocalId` mutable slots | `ValueId` SSA values | Ownership is structural, not inferred |
| `UseMode::Copy \| Move` on use sites | `CopyValue` / `MoveValue` instructions | Copies are explicit in the IR |
| `ArgMode::Copy \| Move \| Ref \| RefMut` | `CallArg { convention }` + explicit borrows | Borrows have scope in the IR |
| `Drop { place }` inserted by drop_elab | `DestroyValue` emitted by lowerer | No analysis pass needed |
| `DropIf { place, flag }` + `SetDropFlag` | Block arguments at merge points | No runtime flags |
| `ScopeLive(LocalId)` | Implicit in block arg threading | Loops thread values via header params |
| No block params | `BasicBlock.params: Vec<BlockParam>` | Ownership at joins is structural |
| `Rvalue::Ref(Place)` / `RefMut(Place)` | `BeginBorrow` / `EndBorrow` instructions | Borrow scopes are explicit |
| Implicit non-trivial place reads/writes | `CopyAddr` / `Take` / `StoreInit` / `StoreAssign` / `DestroyAddr` | Memory ownership is explicit |
| `Op2 PtrOffset` for field access in inits | `FieldAddr { base, ty, field }` | Verifier tracks per-field init state without layout |
