# Pass Pipeline

The MIR pass pipeline (OSSA): what each pass does and the order they run.

## Pipeline

```
HIR
 |
 v
lower_to_ossa        Emit OSSA with copy_value, destroy_value, block args
 |
 ├─ thunk             Generate calling-convention wrappers for ApplyPartial
 ├─ drop_shim         Synthesize __drop$T functions in OSSA form
 ├─ layout            Compute struct/enum sizes (non-generic types)
 ├─ ossa_verify       Check linear ownership invariant
 |
 v
monomorphize          Generic → concrete, substitute types, resolve witnesses
 |
 ├─ expand            CopyValue/DestroyValue → clone/drop calls, erased
 |                    closure retain/release dispatch
 ├─ mono_verify       No TypeParam/Witness remaining; containment invariants
 |
 v
codegen               Emit native code
```

## Ownership in the Lowerer

OSSA does not run separate ownership-analysis passes. The lowerer emits
ownership instructions directly, so copies and drops are visible in the
IR the moment they are needed, and the verifier checks them structurally.

### Copy vs move

The lowerer decides copy vs move at emit time (see `lowering.md`). For
Clone types: if the value is used again after this point, emit
`CopyValue`; if it's the last use, consume it directly (move semantics).
SSA use-def chains carry the same information that liveness analysis
would compute — the lowerer knows a value has later uses because it
tracks scope values — so no backward liveness pass is required.

### Drops

The lowerer emits `DestroyValue` directly at scope exits using its
scope-tracking stack (see `lowering.md`). Every `@owned` value created in
a scope is either consumed (moved, returned, forwarded) or explicitly
destroyed before the scope exits. Because SSA values are defined exactly
once, there is no "maybe initialized" state to track: a `ValueId` either
exists (was produced by an instruction) or doesn't. The verifier checks
consumption rather than initialization, so no forward init-state
dataflow is needed.

### Merge points

Block arguments resolve ownership at merge points statically. Each
predecessor passes `@owned` values through block args or destroys them
before jumping, so there are no drop flags and no per-merge branch
expansion. Joins are structural rather than inferred from mutable locals.

### Nested droppable fields

Drop-shim synthesis handles types with droppable fields transitively.
Every `@owned` value gets a `DestroyValue` regardless of whether the type
was independently flagged as droppable, and the verifier catches any
missing destroy — so nested droppable fields cannot slip through.

## Structural Passes

These passes do structural work — generating new functions or computing
layouts — rather than ownership analysis.

### thunk

Generates wrapper functions for `ApplyPartial` targets conforming to the
thick-callable ABI. The wrapper is expressed in OSSA form, using
ValueIds.

### drop_shim

Synthesizes `__drop$T` functions for droppable types. The shim body is
expressed in OSSA form:

```
fn __drop$MyStruct(%self: @owned MyStruct):
    %ref = begin_mut_borrow %self
    call MyStruct.deinit(%ref)        // convention: MutBorrow
    end_mut_borrow %ref
    (%name, %items) = destructure_struct %self
    destroy_value %name               // drops String
    destroy_value %items              // drops Array
    // %self is consumed by destructure
```

Synthesis is a fixed-point over the type graph, with transitive
detection of droppable fields.

**Constraint on deinit implementations:** `deinit` receives `self` by
MutBorrow. It must not move fields out of `self` (no `take` on self's
fields). The drop shim unconditionally destructures and destroys all
fields after calling `deinit`. If `deinit` moved a field out, the
subsequent `destroy_value` on that field would be a double-free. The
verifier enforces this: the mut borrow on `self` prevents consumption
of `self` or its fields during `deinit`.

### layout

Computes struct/enum memory layouts. Layout computation doesn't touch
instructions — it reads TypeInfo and field types.

### monomorphize

Operates on the OSSA IR. Key points:
- Substitute `TyId` in `ValueDef.ty`
- Resolve `Callee::Witness` → `Callee::Resolved(MonoFuncId)`
- Walk `InstKind` for type substitution
- Block arguments carry type info that must be substituted
- After type substitution, `Ownership` must be recomputed from the
  concrete type's `CopyBehavior`. A generic `T` that was Affine may
  become `Int64` (Bitwise → @none). The mono_verify pass catches
  any ownership annotation inconsistencies.
- A `FuncThick`'s closure kind is part of the type's identity, so it is
  **kind-exact** in both places identity matters: `mangle.rs` emits a
  one-char kind tag (`n`/`m`/`c`/`e`), and `witness.rs match_pattern`
  requires `k1 == k2` — a `mutating (T) -> U` witness does not satisfy a
  normal `(T) -> U` requirement, because the two differ in environment
  ownership and copy class.

## Post-Mono Elaboration

Unlike the structural passes above, these run on the `MonoModule` and turn
the abstract ownership instructions into concrete calls.

### expand

`mono::expand::expand_destroy_copy` rewrites the abstract `CopyValue` /
`DestroyValue` instructions into real work: a clone-shim call, a drop-shim
call, an inline tuple walk — or, for a **boxed closure**, the type-erased
share/release dispatch.

A value of type `escaping (…) -> …` does not name its environment type, so
expand cannot call anything about it directly. Instead it emits two
`Op::ClosureWord` projections (the handle, then the retain or release shim)
and an indirect `Callee::Thin` call:

```
%handle = op1 closure_word(1) %closure
%shim   = op1 closure_word(3) %closure   // 2 = retain, 3 = release
call %shim(%handle)                      // consuming
```

All actual semantics — allocate, retain, release, destroy the captures —
live in per-environment shims synthesized by `kestrel-mir-lower` against the
`@builtin(.SharedBox)` / `@builtin(.UniqueBox)` binding, so swapping the
binding retargets everything. Only the two-word load and the indirect call
are compiler-emitted. A capture-free value carries a null handle and no-op
shims, so no null check is needed (post-mono block splitting is
unavailable).

Copy uses `is_shared()`, not `is_boxed()`: the copy keeps the bitwise
4-word copy and adds a retain so the extra handle is counted. A `consuming`
closure is `not Copyable`, so no clone of it may ever be elaborated and its
retain word must stay unreachable.

## The Six Resource Predicates

Copy and drop classification for `FuncThick` is decided by the **closure
kind**, not by folding the environment payload. Six predicates spread across
pre- and post-mono encode the same three rows and must change as one atomic
edit — a `Bitwise` reading
plus a droppable env double-frees; `Clone` without drop leaks:

| Kind | copy | drop |
|------|------|------|
| `Normal` / `Mutating` (view) | `Bitwise` | no |
| `Escaping` (shared box) | `Clone(Cloneable)` | yes (release) |
| `Consuming` (unique box) | `None` (move-only) | yes (release) |

The six sites: `ty_query::copy_behavior`, `ty_query::needs_drop`,
`expand::ty_needs_drop`, `audit::mono_needs_drop`,
`clone_shim::ty_needs_clone_shim`, and `mono::concrete_copy`
(`drop_fix::field_needs_drop` follows via `needs_drop`). With no stdlib —
no `Cloneable` protocol in the module — the escaping row degrades to
`Bitwise`, the only self-consistent answer when nothing can clone anyway.

## Verification

### ossa_verify

Checks the linear ownership invariant with a CFG worklist over ownership
state. The IR requires each block's parameters to describe the complete
set of `@owned` values live at block entry. Each predecessor consumes its
outgoing values by passing them as block arguments; the target block
receives fresh parameter values. With that invariant, joins are
structural rather than inferred from mutable locals, and verification is
O(blocks + instructions + edges).

**Checks:**
1. Every ValueId defined exactly once
2. Every @owned value consumed exactly once on every reachable path
   (this subsumes "live-in completeness" — if a value is live at a
   block entry but not in the param list, check 2 catches it as
   "unconsumed on the predecessor's exit path")
3. No use after consume (consumed @owned values cannot be read)
4. Borrow liveness: every @guaranteed value is either end_borrow'd
   or forwarded as a @guaranteed block arg on all paths. The
   verifier tracks open borrows as a set — no same-block assertion.
5. Borrow provenance: while any value with `borrow_source = %v` is
   live, `%v` cannot be consumed. For mut borrows, `%v` also cannot
   be read. Provenance propagates through @guaranteed block args
   and forwarding extractions.
6. Block argument consistency (predecessor arg count/type/ownership matches)
7. Address state consistency (see "Address State Tracking" below)
8. Op operands must be @none (trivial)
9. Trivial correctness (values of trivial types have Ownership::None;
   copy_value/destroy_value/begin_borrow must not appear on @none values)
10. Return consistency (return value matches function signature;
    functions returning `!` end with Panic/Unreachable, never Return)

**Error model:**
```rust
pub struct VerifyError {
    pub block: BlockId,
    pub inst: Option<u32>,
    pub message: String,
}

pub fn verify_ossa(body: &OssaBody, module: &MirModule) -> Vec<VerifyError>
```

The verifier runs after every pass, not just at the end. This attributes
errors to the specific pass that introduced them, so a bug in any
individual pass surfaces immediately.

**Address State Tracking:**

The verifier tracks memory initialization state at two granularities:

*Whole-address state* applies to addresses from function parameters,
globals, `CopyAddr` results, and other non-`Uninit` sources. The
verifier tracks a single `Init | Uninit` state per address:

| Instruction | Precondition | Effect |
|-------------|-------------|--------|
| `Take` | Init | → Uninit |
| `DestroyAddr` | Init | → Uninit |
| `StoreInit` | Uninit | → Init |
| `StoreAssign` | Init | → Init (destroys old) |
| `Load` | Init | (no change) |
| `CopyAddr` | Init | (no change) |
| `BeginBorrowAddr` | Init | (no change, frozen) |
| `BeginMutBorrowAddr` | Init | (no change, frozen) |

*Sub-field state* applies to addresses from `Uninit` instructions. The
verifier reads the field count from `StructDef` (not byte-level layout)
and tracks `Init | Uninit` per `(base_addr, FieldIdx)` pair. The
`FieldAddr` instruction is the bridge — it tells the verifier which
field is being addressed without requiring layout information:

```rust
struct UninitAddrState {
    ty: TyId,
    fields: HashMap<FieldIdx, InitState>,  // each starts Uninit
}
```

Transfer rules for sub-field state:

| Instruction | Precondition | Effect |
|-------------|-------------|--------|
| `Uninit { ty }` | — | Create state; all fields `Uninit` |
| `FieldAddr { base, field }` | base has UninitAddrState | Associate result addr with `(base, field)` |
| `StoreInit { addr }` on field addr | Field is `Uninit` | → `Init`. Error if already `Init` (double-init). |
| `DestroyAddr { addr }` on field addr | Field is `Init` | → `Uninit`. For failable init cleanup. |
| `Take { addr }` on base addr | All fields `Init` | Consume whole allocation. Error if any field `Uninit`. |

The verifier does not need layout (field offsets, sizes, alignment) for
sub-field tracking. It only needs the field count from `StructDef`,
which is available before the layout pass runs. This is why `FieldAddr`
exists as a separate instruction rather than using raw `Op2 PtrOffset` —
it carries typed field identity that the verifier can reason about.

### mono_verify

Post-mono checks: no `TypeParam`/`Witness` survives, plus containment
invariants. **Invariant 3b** (defense-in-depth) is that no `Bitwise` or
`Clone` type may contain a non-Copyable child. Closures extend it, because a
`FuncThick` has no nominal entity for the ordinary child lookup to see:

- an `Escaping` field may not sit in a `Bitwise` container — the container
  must be pushed to `Clone(_)` by the clone-shim overwrite so a copy SHARES
  the handle; bit-copying it would skip the share (the double-free class this
  check exists to make loud);
- a `Consuming` field makes *neither* `Bitwise` nor `Clone(_)` legal —
  cloning the container would have to duplicate a unique one-shot owner — so
  its container must be `None`.

## Optional Optimizations

These passes are not required for correctness and may run at any point.

### copy_optimize

Dead-copy elimination. The optimization is valid when:
1. `%original` has no uses between the `CopyValue` and its
   `DestroyValue` on any reachable path, AND
2. `%original` reaches `DestroyValue` on all paths from the
   `CopyValue`.

When both hold, the copy is the last consumer of the original's
value — the clone can be replaced with a move.

```
// Before:
%copy = copy_value %original       // clone() call
destroy_value %original            // drop original

// After:
%moved = move_value %original      // no clone needed (rename)
// %moved has the same lifetime as %original did
```

### canonicalize

Cleanup pass:
- Eliminate redundant `MoveValue` (chain of renames → single value)
- Coalesce block arguments when a block has a single predecessor
- Remove dead code (unused @none values)

## Summary

```
Required passes:
  thunk → drop_shim → layout → ossa_verify

With optional optimization:
  copy_optimize → thunk → drop_shim → layout → ossa_verify → canonicalize
```

The passes that run do structural work — generate new functions, compute
layouts — rather than ownership analysis. Ownership analysis is handled
by the verifier, which runs after every pass and is linear in the CFG
size under the block-parameter live-in invariant.
