# Pass Pipeline

Passes, ordering constraints, and what each pass reads, writes, and assumes.

## Pipeline overview

```
MirModule (generic)
 │
 ├─ 1. clone elaboration
 ├─ 2. drop elaboration
 ├─ 3. layout (non-generic types)
 ├─ 4. verify (generic)
 │
 ▼  5. monomorphization
MonoModule (concrete)
 │
 ├─ 6. verify (mono)
 │
 ▼  codegen
native code
```

## 1. Clone elaboration

**Purpose:** Rewrite Copy of Clone-typed values into explicit witness clone
calls followed by Move of the clone temp.

**Reads:** MirBody statements/terminators, copy behavior via
`copy_behavior(arena, module, ty)` which consults TypeInfo on struct/enum
defs AND where-clause constraints on the enclosing function (for generic
type params like `T: Cloneable`), backward liveness (computed internally).

**Writes:** Inserts clone call statements, rewrites Use(op, Copy) → Use(clone_temp, Move).

**Invariant produced:** After this pass, no Copy of a Clone-typed value
exists in the IR. All Clone copies are explicit witness calls producing
a fresh owned temp that is then moved.

**Must run before:** Drop elaboration. Drop elab assumes Clone copies have
been rewritten — it needs to see the Call + Move pattern to track the clone
temp as a droppable owned value.

### Decision rule

One rule, all positions, based on backward liveness:

> For any Copy of a Clone-typed place — `UseMode::Copy` on assignments
> and compound rvalue operands, `ArgMode::Copy` on call args, or the
> operand of a `Return` — if the source is **live** after this point,
> insert a clone before the statement and rewrite to Move. If **dead**,
> rewrite to Move directly (last use, no clone needed).

This applies uniformly to:
- **Assignments** (`Rvalue::Use(Place(p), Copy)`)
- **Compound rvalue operands** (Construct, Tuple, EnumVariant, etc.)
- **Closure captures** (ApplyPartial operands)
- **Call args** (`(Place(p), ArgMode::Copy)` for consuming params)
- **Return** (`Return(Place(p))`)

No position-specific special cases. No temp detection heuristics. Liveness
determines everything:
- A single-use temp is dead after its use → rewritten to Move, no clone.
- A user local used later → live → clone inserted.
- A user local at its last use → dead → rewritten to Move, no clone.

The lowering does not need Clone awareness. For consuming call args, it
emits `ArgMode::Copy` when the source needs to survive and `ArgMode::Move`
when it doesn't. Clone elaboration handles the rest.

Bitwise-Copy operands are unaffected (not Clone-typed). Affine types are
already Move by construction. `ArgMode::Ref`/`ArgMode::RefMut` are borrows,
not copies — clone elaboration ignores them.

### Backward liveness analysis

Clone elaboration computes backward liveness per function body.

**Lattice:** A bitset over `LocalId`. Bit set = local is live (has a
future read on some path).

**Transfer functions (backward, per statement):**
- **Read of `Place(p)`** (any Operand position): gen `p.root_local()`
- **Assign/Call to dest `Place(p)`**: kill `p.root_local()` (redefined)
- **Terminator `Return(Place(p))`**: gen `p.root_local()`

**At CFG join points:** union (live on any predecessor = live).

**At CFG split points** (block has multiple successors): a local is live
at a statement if it's live on any successor path from that point. This
is standard backward analysis — liveness at a statement is the union of
its successors' entry states, minus kills, plus gens.

The liveness result is queried per-statement: "is local X live immediately
after statement S?" Clone elaboration checks this for every Copy operand
whose type is Clone.

**Implementation:** Worklist-based backward fixpoint using the shared
`CfgInfo` and `backward_fixpoint` infrastructure (see "Shared dataflow
infrastructure" below). Precomputed once per function, queried per
statement via `Liveness::is_live_after(block, stmt_index, local)`.

For batch processing (clone elaboration processes every statement in a
block), `Liveness::block_liveness_after(body, block)` precomputes a
`Vec<BitVec>` where entry `i` is the liveness state immediately after
statement `i`. This avoids re-walking the block per query. Must be
called before the block is modified (inserted clone calls would corrupt
the index mapping).

## 1b. Thunk synthesis

**Purpose:** Generate thunk wrapper functions for `ApplyPartial` targets.

When a function is used as a thick callable (via `Rvalue::ApplyPartial`),
codegen needs a wrapper that conforms to the thick-callable ABI: an
ignored env pointer parameter followed by forwarded arguments.

**Runs after:** Clone elaboration (thunks forward arguments, don't clone).

**Process:**
1. Scan all bodies for `Rvalue::ApplyPartial { func }` references
2. For each unique target function entity, check if a thunk already exists
3. If not, generate a `FunctionDef` with `FunctionKind::Thunk { original }`
   that takes `(env_ptr: Pointer(Unit), ...forwarded_params)` and emits
   a direct `Call` to the original function, forwarding all args
4. The thunk's entity is allocated from the module's entity counter

Thunks are generic — they inherit the original function's type params
and are monomorphized alongside it. In MonoModule, each thunk instantiation
becomes a separate `MonoFunction`.

## 2. Drop elaboration

**Purpose:** Insert destructor calls at all exit points for owned values
that are still live.

**Reads:** MirBody CFG, TypeInfo.drop on types, local_scopes for loop
boundaries, failure_return_blocks for effectful init cleanup.

**Writes:** Inserts Drop/DropIf/SetDropFlag statements, allocates drop
flag locals, may insert ScopeLive markers.

**Invariant produced:** Every droppable local is dead or conditionally
dropped on every path to a Return terminator.

### Drop shims

The core design change from kestrel-mir-1. Instead of Deinit/DeinitIf
markers that are iteratively expanded into CFG surgery, drop elaboration
inserts calls to drop shims:

```
// Unconditional drop
call __drop$String(move place)

// Conditional drop (at CFG join points)
branch drop_flag → drop_block, skip_block
drop_block:
    call __drop$String(move place)
    jump continue_block
skip_block:
    jump continue_block
```

### Drop shim synthesis

One `__drop$T` function per type that needs cleanup, synthesized as a
fixed-point over the type graph:

```rust
fn __drop$MyStruct(consuming self: MyStruct) {
    // 1. Call user-defined deinit (if any)
    call MyStruct.deinit(&var self)
    // 2. Recursively drop fields that need cleanup
    call __drop$String(move self.name)
    call __drop$Array$Int64$(move self.items)
}
```

For enum types, the shim contains a Switch on the discriminant with
per-variant cleanup:

```rust
fn __drop$Result$String_Error$(consuming self: Result) {
    switch self {
        .Ok → call __drop$String(move self.Ok.value)
        .Err → call __drop$Error(move self.Err.value)
    }
}
```

Shim synthesis is recursive (fields may themselves need shims) and uses
a visited set to break cycles. The fixed-point terminates when no new
shims are needed.

### Droppable local identification

One canonical function replaces the three independent implementations
in kestrel-mir-1:

```rust
fn droppable_locals(body: &MirBody, module: &MirModule) -> Vec<DroppableLocal>
```

A local is droppable if:
- Its type has `TypeInfo.drop != DropBehavior::None` (or the type contains
  an unresolved TypeParam, which conservatively needs tracking).
  Borrowed closure captures have type `Pointer(T)` (Bitwise) and are
  automatically excluded — no special flag needed.
- It owns a value — i.e. it is assigned via Construct, EnumVariant, Call
  result, copy-from-construct chain, or is a consuming parameter. Locals
  that only hold moved-from temps (single-use intermediaries that are always
  moved out before any exit) are excluded by construction: their sole use
  is a Move, so they're always Dead at exits.
- It's not a drop flag local (allocated by drop elaboration itself)

This function is called by drop elaboration, move checking, and the verifier.

### Dataflow

Forward dataflow with InitState lattice: `{ Dead, Live, Maybe }`.

- **Dead:** not initialized, or moved out. No drop needed.
- **Live:** definitely initialized on all predecessor paths. Unconditional drop.
- **Maybe:** initialized on some paths, dead on others. Conditional drop
  (DropIf with flag).

Transfer functions:
- Assign/Call to a droppable local dest → gen (Live). If the dest was already
  Live, insert a Drop before the assignment (overwrite-drop). If Maybe,
  insert DropIf.
- Assign/Call to a droppable projected field dest is allowed only when the
  root local is definitely Live. Drop elaboration inserts a Drop of the old
  field value before the overwrite when that field type needs cleanup.
- Use(place, Move) / ArgMode::Move of a droppable root local → kill (Dead)
- Return(Operand::Place(p)) of a droppable root local → kill (Dead).
  The return operand is implicitly moved to the caller.
- ScopeLive(local) → kill (Dead) — loop re-entry resets to uninitialized

### Partial moves are deferred

MIR-2 v1 deliberately tracks ownership at root-local granularity. Partial
moves are not part of the supported IR semantics yet.

The verifier rejects moves out of projected fields of owned droppable
aggregates:

- `Use(Place(s.f), Move)`
- `Call { args: [(Place(s.f), ArgMode::Move)] }`
- `Return(Place(s.f))`

For now, lowering must either move the whole aggregate (`move s`) or copy/clone
the field value into a new owner before using it. This keeps root-local drop
dataflow sound: a droppable local is either Live, Dead, or Maybe as a whole.

Projection-aware tracking (see place.md's move path tree) is the planned
refinement. When implemented, moving `s.f` will mark `s` as partially moved
while `s.g` remains Live, and the droppable-local identification and transfer
functions will operate on MovePathIds instead of LocalIds.

At CFG join points, the meet operator:
- Live ∧ Live = Live
- Dead ∧ Dead = Dead
- everything else = Maybe

### Drop flag convention

`true` = live (value exists, needs drop). `false` = dead (skip drop).

Flags are allocated as `LocalDef { ty: Bool, ... }` with a SetDropFlag(false)
at function entry. Gen sites set the flag to true. Kill sites set it to false.

### Phase order within drop elaboration

1. Identify droppable locals
2. Forward dataflow to fixed point
3. At each Return/loop-exit/back-edge: insert Drop (for Live) or DropIf (for Maybe).
   Consuming parameters (including consuming `self` receivers) are handled
   by the normal droppable-local identification — no special phase needed.
4. For effectful inits: insert DropIf for partially-initialized fields on
   failure paths. An effectful init (`init?` or `init throws`) can fail
   after some fields are initialized. The `failure_return_blocks` on MirBody
   mark the blocks where the init returns failure. Drop elaboration inserts
   DropIf for each field that was Live before the failure point, using
   per-field drop flags set during the init sequence.

## 3. Layout (non-generic)

**Purpose:** Compute struct and enum sizes, offsets, and alignment for
types with no unresolved type params.

**Reads:** StructDef/EnumDef field types. Takes `&TargetConfig` as a
parameter (not stored on the module — MIR is target-agnostic).

**Writes:** TypeInfo.layout on each non-generic StructDef/EnumDef.

Uses a multi-pass fixed-point algorithm: each pass tries to compute
layouts for types whose dependencies are already laid out. Terminates
when no new progress is made. Generic types (with TypeParam in fields)
get `layout: None` — their layouts are computed during monomorphization
Phase 4 when all type args are concrete.

Layout computation uses the shared `StructLayout` arithmetic helpers
(see types.md). These were previously in the `kestrel-codegen` crate's
`layout.rs` (990 lines) and are now part of kestrel-mir-2.

Struct layout: fields laid out sequentially, each aligned to its natural
alignment, total size padded to struct alignment.

Enum layout: discriminant tag (I32 at offset 0) + payload region (max
size across all variant payloads, at a computed payload offset). The
`EnumLayout` stores per-variant `StructLayout` for payload fields.

## 4. Verify (generic)

**Purpose:** Check structural and ownership invariants on generic MIR.

**Structural checks:**
- Every block has a valid terminator
- Local references are in bounds
- Param count matches locals; for borrow/mutborrow params,
  `LocalDef.ty == Pointer(ParamDef.ty)`; for consuming, `LocalDef.ty == ParamDef.ty`
- Drop/DropIf statements only exist after drop elaboration has run
  (the lowering must never emit them; they are drop-elab-only)
- FieldIdx/VariantIdx in bounds for their type
- Call arg count matches callee params
- ArgMode::Ref/RefMut only on Operand::Place (not Const)

**Ownership checks (forward dataflow):**
- Every droppable local is Dead or DropIf'd at every Return
- No Use(place, Move) of an already-dead place
- No Use(place, Copy) of an affine type
- No projected move out of an owned droppable aggregate. This includes
  `Use(Place(s.f), Move)`, `ArgMode::Move` on `s.f`, and `Return(s.f)`.
  Partial moves require projection-aware move paths and are not supported in
  MIR-2 v1.

The verifier calls the same `droppable_locals()` and runs the same
dataflow as drop elaboration. Because the infrastructure is shared
(see "Shared dataflow infrastructure" below), the verifier is checking
the same model, not a re-implementation.

**Panic paths:** `Terminator::Panic` is an abort — no cleanup runs.
The verifier does NOT check that droppable locals are dead at Panic
terminators. This matches the semantics: panic = process termination.

### Error handling

The verifier collects ALL errors rather than failing on the first. Each
error is a `VerifyError` with function index, optional block index,
statement index, and a message string.

```rust
struct VerifyError {
    func_idx: usize,
    block: Option<BlockId>,
    stmt: Option<usize>,
    message: String,
}

struct VerifyResult {
    errors: Vec<VerifyError>,
}
```

`VerifyResult::is_ok()` → true if no errors. The compiler pipeline checks
this and either continues (if ok) or dumps all errors to stderr and aborts
compilation. Verification failures are ICEs (internal compiler errors) —
they indicate a bug in a preceding pass, not a user error. User-facing
diagnostics are emitted by earlier stages (type inference, name
resolution); the MIR verifier catches pass implementation bugs.

Monomorphization verification (`Verify (mono)`) uses the same error model.

## 5. Monomorphization

See `monomorphization.md` for the full design.

**Input:** MirModule (generic). **Output:** MonoModule (concrete).

**What it does:**
1. BFS from entry points to discover all concrete instantiations
2. Clone function bodies, substituting all type params
3. Resolve all Callee::Witness → MonoCallee::Direct(MonoFuncId)
4. Resolve all AssociatedProjection types
5. Compute layouts for all concrete types (the layout pass runs once
   on generic MIR for non-generic types, then monomorphization computes
   layouts for all concrete instantiations of generic types)
6. Produce a self-contained MonoModule

## 6. Verify (mono)

**Purpose:** Check monomorphization invariants on the produced MonoModule.
Runs AFTER monomorphization, not before.

- No MirTy::TypeParam, SelfType, or AssociatedProjection in any body or type
- No Callee::Witness in any body
- All TypeInfo.layout values are Some (fully computed)
- All MonoCallee::Direct targets are valid MonoFuncIds
- All MonoFunction.body is Some unless extern_info is Some

## Shared dataflow infrastructure

Drop elaboration, move checking, liveness, and verification all need
CFG analysis. A shared module provides reusable infrastructure:

```rust
struct CfgInfo {
    rpo: Vec<BlockId>,                          // reverse postorder
    predecessors: HashMap<BlockId, Vec<BlockId>>,
}

fn compute_cfg_info(body: &MirBody) -> CfgInfo;

trait Lattice: Clone + PartialEq {
    fn bottom() -> Self;
    fn join(&mut self, other: &Self) -> bool;   // merge, returns true if changed
}

trait ForwardTransfer<S> {
    fn entry_state(&self, body: &MirBody) -> S;
    fn transfer_block(&self, body: &MirBody, block: BlockId, state: &mut S);
}

trait BackwardTransfer<S> {
    fn exit_state(&self, body: &MirBody) -> S;
    fn transfer_block(&self, body: &MirBody, block: BlockId, state: &mut S);
}

fn forward_fixpoint<S: Lattice>(
    cfg: &CfgInfo,
    body: &MirBody,
    transfer: &impl ForwardTransfer<S>,
) -> Vec<S>;

fn backward_fixpoint<S: Lattice>(
    cfg: &CfgInfo,
    body: &MirBody,
    transfer: &impl BackwardTransfer<S>,
) -> Vec<S>;
```

Forward and backward use separate transfer traits — a forward pass has no
exit state, a backward pass has no entry state. Splitting makes invalid
combinations unrepresentable.

Drop elaboration, move checking, and the verifier all parameterize over
this. No more duplicated RPO computation, predecessor maps, or worklist
iteration across passes.

## Future: borrow checking

A borrow checking pass would slot between drop elaboration and verification:

```
clone elab → drop elab → borrow check → verify → monomorphize
```

It would use the same shared dataflow infrastructure. Reference creation
points are:
- `Rvalue::Ref(Place)` / `Rvalue::RefMut(Place)` — standalone refs
  (complex lifetimes, flow into locals/structs)
- `ArgMode::Ref` / `ArgMode::RefMut` on call args — call-scoped refs
  (trivially bounded lifetime)

The borrow checker handles these two cases with different strategies.
Call-scoped borrows are checked locally. Standalone refs require
tracking through the CFG using the move path tree.

The flat Place model with prefix checking (see `place.md`) provides the
alias analysis. The projection-aware move path tree provides the
granularity for "borrowing s.f doesn't conflict with moving s.g."
