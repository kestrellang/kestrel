# Stage 1.5 — Semantics

> **Status 2026-06-10**: IMPLEMENTED (items 1+5). One stage-1 gap
> surfaced during implementation: a `&mutating` return rooted in a
> direct FIELD projection (`mutating func m() -> &mutating T { self.v }`)
> is E494 — the escape checker verifies only param-rooted and
> Pointer-derived mutable roots. Accessor bodies inherit this, so
> `mutating ref` bodies use the Pointer bridge
> (`self.ptr().offset(by:).mutatingValue`), like Array. Worth a stage-2
> work item if field-backed mutable places are wanted without Pointer.

## Place accessors — DECIDED 2026-06-10

**Per-operation provider rule**: each logical operation on an
accessor-backed place has exactly one provider. Duplicate providers are
decl-time errors (`errors.md`).

| operation | provider |
| --- | --- |
| read (value context) | `ref` (then copy-out decay), else `get` |
| read (place context: member/operator/borrow-conv arg) | `ref` in place, else `get` (owned temp) |
| `x(i) = v` | store through `mutating ref`, else `set(newValue)` |
| `x(i) += v` | in-place through `mutating ref`, else `get` → op → `set` writeback |
| `x(i).mutate()` (mutating member) | `mutating ref`; else writeback via `get`/`set` for Copyable; copy-guard error for NotCopyable with an "add a `mutating ref` accessor" hint |

- At most one read provider (`get` XOR `ref`) and one write provider
  (`set` XOR `mutating ref`). **Cross-mixes are allowed** (decided
  2026-06-10 after surveying drawbacks): `get` + `mutating ref` is
  computed-read / in-place-write (Swift's `get` + `_modify` shape);
  `ref` + `set` is borrowed-reads / write-hook. The one semantic novelty:
  in `get` + `mutating ref`, the read half of an RMW happens *through
  the ref*, bypassing `get` — intended, inherited from Swift's
  uncompiler-checked **coherence contract** (both providers must view
  the same logical storage), and must be pinned by a test so the
  divergence is documented behavior.
- **`get`/`set` writeback is the permanent fallback, not a legacy
  path.** Computed accessors (bit-packed storage, derived values,
  Dictionary's enum-payload buckets) cannot fabricate addresses; they
  keep get/set and the writeback lowering. This subsumes the old
  "value-subscript writeback" item-1 remainder.
- **Lowering rides stage 1 unchanged**: each `ref`/`mutating ref`
  accessor is a synthesized `-> &T` / `-> &mutating T` callable flowing
  through `CallableRefReturn`, the escape checker, and the mutable-root
  rule (a `mutating` receiver IS a mutable root, so E495 holds by
  construction). The fabricated ref is expression-scoped like every
  stage-1 ref; E497 applies unchanged.
- **COW**: unique-ing happens inside the accessor body BEFORE the ref is
  fabricated (`makeUnique()` first in `mutating ref`), same contract as
  `Array.mutableAt`. The `f(arr(0), arr)` same-expression re-share wart
  remains the accepted §10.4 behavior.
- **Scope restriction**: `ref` accessors are legal only on concrete
  inherent decls (struct bodies, concrete `extend`). Protocol extensions
  and protocol requirements stay get/set — ref-returning witness
  requirements remain the boundary stage 1 drew. (Note the unified
  `Slice` subscript couldn't be a ref accessor anyway: its read is
  computed through `SeqIndex.readSeq`, so there is no address to hand
  out.)
- **Evaluation order — pinned by the shipped path**: RHS first, then the
  LHS ref fabrication, then the store
  (`kestrel-mir-lower/src/body/expr.rs:649` lowers `value` before
  `:657` lowers the target ref; then PtrTo + StoreAssign + EndBorrow).
  Accessor-routed assignment MUST preserve this order so
  `arr.mutableAt(index: i) = v` and `arr(i) = v` are one rule.

## Match scrutinee place rule — IMPLEMENTED 2026-06-10

A match containing any `&` binding pattern evaluates its scrutinee as a
**place**: the place is evaluated once via the borrow-conv machinery
(var slot / binding pass-through / accessor element; an rvalue's owned
temp IS the match-scoped pin) and its raw address — an owned pointer
scalar — threads through the decision tree's block params. Every test
and leaf reads through an intra-block `BeginBorrowAddr` view; `&v`
binds the payload projection in place (a real nested borrow, so it is
scope-tracked and block-local like every named binding); plain bindings
in place-mode force their copy. Matches without `&` bindings keep the
decay lowering byte-identical. `&mutating v` requires the scrutinee
place to be a mutable root (E210, the E495 predicate family) and stores
write the payload in place.

## Named ref bindings — IMPLEMENTED 2026-06-10 (ratified semantics)

- **Statement-boundary survival**: bindings are registered multi-use
  (`ref_binding_vals`) — `end_stale_refs_since` spares them; every
  "the ref's single use — end it" decay site funnels through
  `end_ref_if_single_use`, which copies WITHOUT ending a binding's
  borrow. Lexical scope exit / the next terminator own the end.
- **Block-local enforcement**: a binding live at an inside-fn
  terminator ends silently when fully used (remaining-use counter,
  decremented once per read expr) and is the binding-worded E497 when
  uses remain. `if` conditions work (they lower pre-branch).
- Store-through (`r = v` via the shipped mut-ref assign path, RHS
  first, no trailing EndBorrow), no-rebind, `let s = r` decays —
  ratified and shipped.
- Interaction with `diamond_conditional_move_let_drop_timing`
  (still-open bug — a conditionally-consumed `let r = &x` inherits it).
- ~~Cross-block: bindings stay block-local~~ RELAXED 2026-06-11
  ("1.75"): bindings thread through all control flow as @guaranteed
  block args and end at lexical scope exit. The block-local rule below
  is historical; `if` conditions, arms, merges, and loops all work.
- **Accepted E498 blind spot**: a place-mode match (or store-through)
  whose address came from a single-use expression ref ends that ref at
  pin time (`PtrTo` then EndBorrow) — a later consume of the
  underlying storage inside an arm isn't chained to the address.
  Stage-2 dangle territory; the storage itself is pinned for the
  match's duration, so the common shapes stay sound under may-alias.
