# Design Rationale

Why each decision was made. Each section: MIR-2 design → what broke →
OSSA design -> why. This is the "don't revert this" document.

## SSA Values Instead of Mutable Locals

**MIR-2:** Values live in mutable `LocalId` slots. A local can be
assigned, moved-from, reassigned, and dropped. Ownership state changes
across the function and must be reconstructed by forward dataflow
analysis (`init_state.rs`: Dead/Live/Maybe per local per block).

**What broke:** The analysis is approximate. At control-flow join points,
a local that's Live on one path and Dead on another becomes Maybe —
requiring runtime drop flags (`DropIf` + `SetDropFlag`). The flag
machinery spans the entire function, requires ordered insertion of
flag updates, and interacts with overwrite-drops and scope-exit drops
in fragile ways. Call-result temporaries excluded from the droppable
set caused leaks; including them caused double-frees because
clone_elab's liveness analysis disagreed with drop_elab's init-state
analysis.

**OSSA:** SSA values. Every instruction produces a new `ValueId`.
Values are immutable once defined. Ownership is a property of the value
definition — `@owned`, `@guaranteed`, or `@none`. No post-hoc
reconstruction of Maybe state. No drop flags.

**Why:** The root cause of MIR-2's bugs is not in any individual pass —
it's in the fundamental decision to reconstruct ownership from mutable
local state. Every pass (clone_elab, drop_elab, drop_flag_expand) is a
compensating mechanism for information that was lost when the lowerer
emitted `UseMode::Copy` and `LocalId` assignments without ownership
semantics. SSA makes ownership structural: under the block-parameter
live-in invariant, the verifier is linear in the CFG size and ownership
bugs in modeled values become verifier failures.

## Three Ownership Kinds (Not Four)

**Swift SIL:** Four ownership kinds: `@owned`, `@guaranteed`, `@unowned`,
`@none`.

**Kestrel OSSA:** Three: `@owned`, `@guaranteed`, `@none`. No `@unowned`.

**Why:** `@unowned` in Swift handles weak references to class instances
that could be deallocated while a non-owning reference exists. Kestrel
has no reference types in the language (no `&T` as a value), no weak
references, and is single-threaded. There is no scenario where a value
could be invalidated while a non-owning reference exists. Adding
`@unowned` would be dead complexity.

## No begin_access / end_access

**Swift SIL:** `begin_access [read|modify|init|deinit]` / `end_access`
enforce exclusivity (Law of Exclusivity: no simultaneous mutable and
immutable access to the same memory).

**Kestrel OSSA:** No access markers. No exclusivity checking.

**Why:** Kestrel is single-threaded. The only way to create aliasing
in Kestrel is through Pointer operations, which are explicitly unsafe.
The language-level borrow model (ParamConvention::Borrow/MutBorrow)
ensures that callers don't hold conflicting references — but this is
enforced by the type system (param conventions), not by MIR-level
access markers. Adding access markers would be correct but unnecessary
complexity for a single-threaded language.

## Call-Scoped Borrows (Today), Cross-Block Ready (IR)

**Swift SIL:** Borrows can flow through block arguments ("reborrow phi
arguments"), escape into struct fields via `store`, and span multiple
basic blocks. This requires complex borrow scope tracking with
"enclosing values" and "borrowed-from" annotations on phi arguments.

**Kestrel OSSA:** The IR and verifier accept @guaranteed block
parameters and cross-block borrow scopes. But the lowerer today only
emits call-scoped borrows — begin and end in the same block, bracketing
a single call. Borrows don't flow into block arguments, struct fields,
or closures today.

**Why the split:** Kestrel has no user-facing reference types today.
Borrows exist only at function call boundaries. The lowerer exploits
this by emitting only the simplest borrow pattern. But the IR doesn't
enforce this restriction — it accepts @guaranteed block params with
borrow provenance tracking (`ValueDef.borrow_source`). This means
adding lexical lifetimes or NLL later is a lowerer change, not an IR
or verifier redesign.

The cost of accepting @guaranteed block params in the IR from day one
is near zero — the verifier already tracks @owned consumption per path,
and tracking @guaranteed provenance uses the same infrastructure. The
cost of retrofitting it later would be significant: reworking the block
param model, every terminator, the verifier, and codegen.

**What the verifier does:** Tracks open borrows as a set per block. At
each block exit, every @guaranteed value must be either end_borrow'd or
forwarded as a @guaranteed block arg. The `borrow_source` field on
ValueDef propagates through block args and forwarding extractions. While
any value with a given `borrow_source` is live, that source cannot be
consumed (shared borrow) or read/consumed (mut borrow). Call-scoped
borrows trivially satisfy this — they open and close in the same block.

## Block Arguments Instead of Drop Flags

**MIR-2:** At control-flow join points where a value might or might not
be initialized, drop elaboration allocates a boolean drop flag. The flag
is set to `true` when the value is initialized and `false` when moved.
At scope exit, `DropIf { place, flag }` conditionally calls the drop
shim. A separate `drop_flag_expand` pass then lowers `DropIf` into
three new basic blocks (branch on flag → drop or skip → continue).

**What broke:** Drop flags are function-wide mutable state. The flag
must be updated at every initialization and every move. The ordering
of flag updates relative to overwrite-drops and scope-exit drops is
fragile. The flag expansion pass creates 3 blocks per DropIf, making
the CFG significantly larger. And the flag is a runtime cost — a branch
for every conditional drop.

**OSSA:** Block arguments. At every merge point, each predecessor
explicitly passes @owned values or destroys them before jumping. The
block's parameter list declares what values it expects. There is no
ambiguity and no runtime flag.

**Why:** Block arguments are the standard SSA solution for this problem.
They are zero-cost at runtime (Cranelift block params are register
allocation constraints, not branches). They make the CFG explicit about
what values are alive at every point. Each block's parameter list is the
complete owned live-in set for that block. Every value passed as a block
argument is consumed by the jump (ending its lifetime in the predecessor)
and reborn as a new value in the successor (starting a new lifetime).

## CopyValue as an Instruction (Not a Pass)

**MIR-2:** The lowerer emits `UseMode::Copy` on operands, then
`clone_elab` runs a backward liveness analysis to determine which
copies need clone calls (live after → clone) and which can be optimized
to moves (dead after → just move).

**What broke:** `clone_elab` must run before `drop_elab` (ordering
dependency). If clone_elab makes a mistake (e.g., for temp-to-user
copies), drop_elab produces wrong results. The two passes disagree on
which values are alive, causing leaks or double-frees.

**OSSA:** `CopyValue` is an explicit instruction in the IR. The lowerer
emits it when it knows a copy is needed. If the lowerer emits too many
copies, the `copy_optimize` pass removes unnecessary ones — but this is
an optimization, not a correctness requirement.

**Why:** Moving the copy decision to the lowerer eliminates the ordering
dependency. The lowerer knows at emit time whether a value will be used
again (it has the HIR scope). The copy_optimize pass can run at any
point without affecting correctness. And `CopyValue` is explicit in the
IR — the verifier can check that it's only used on Clone types, the
codegen can emit the witness call, and there's no hidden "this Copy
might become a Move later" semantic.

## DestroyValue at the Lowerer (Not a Pass)

**MIR-2:** The lowerer emits no cleanup instructions. `drop_elab` runs
forward init-state analysis to find which locals are alive at returns,
overwrites, and scope exits, then inserts `Drop`/`DropIf` statements.

**What broke:** The analysis misses cases: call-result temporaries,
transitively droppable fields, `Pointer.read()` copies. Each miss is a
leak or use-after-free.

**OSSA:** The lowerer emits `DestroyValue` at every scope exit for
every unconsumed @owned value. This is a local decision using a scope
tracking stack — no dataflow analysis.

**Why:** The lowerer has strictly more information than a post-hoc
analysis pass. It knows the scope structure from the HIR, it knows
which values it created, and it knows which ones were consumed. The
scope stack is ~50 lines of code. The init-state analysis was ~170
lines plus ~450 lines of drop_elab plus ~100 lines of drop_flag_expand
— all replaced by a simple scope tracker.

And if the lowerer gets it wrong (forgets a DestroyValue), the OSSA
verifier catches it immediately: "unconsumed @owned value %42 at exit
of block bb7." The error message tells you exactly which value leaked
and where. MIR-2's verifier could only say "local %3 is Live at
Return" — which was correct but didn't tell you whether the bug was
in the lowerer, clone_elab, or drop_elab.

## Trivial Types as @none (Excluded from Tracking)

**MIR-2:** All types go through init_state and liveness analysis. The
`needs_drop()` query filters at the point of use, but the dataflow
tracks everything.

**OSSA:** Trivial types get `Ownership::None`. They are completely
excluded from ownership tracking. The verifier ignores them.
`CopyValue`, `DestroyValue`, `BeginBorrow`, and `EndBorrow` should not
appear on @none values (verifier rejects if they do).

**Why:** Most values in a typical function are trivial (integer
arithmetic, boolean conditions, pointer offsets). Tracking them through
ownership analysis is wasted work. In MIR-2, the init-state bitset
included every local — shrinking it to only @owned values reduces
verifier cost significantly.

## Forwarding Instructions Consume Aggregates

**MIR-2:** `StructExtract` was not possible on droppable aggregates
(the verifier rejected partial moves). The workaround: move the whole
aggregate or clone the field.

**OSSA:** Extracting from an owned aggregate consumes that aggregate.
When more than one field is needed, `DestructureStruct` extracts all
fields at once so the aggregate is consumed exactly once. A single
`StructExtract` from an owned aggregate is only for cases where all
unextracted fields are @none. Extracting from a borrowed aggregate is a
non-consuming projection and produces a guaranteed value.

**Why:** This is how Swift SIL works, and it's the natural model for
an ownership-based IR. If you own a struct and want one field, you
destructure it — taking ownership of the field and destroying the rest.
This eliminates the need for clone-to-extract patterns.

For the initial implementation, the lowerer can continue using
whole-aggregate moves and defer partial extraction. The IR supports it;
the lowerer can adopt it incrementally.

## Module-Level Types Shared with MIR-2

**Decision:** kestrel-mir-3 reuses body-independent type and declaration
metadata from mir-2 where possible: `StructDef`, `EnumDef`, `ProtocolDef`,
`WitnessDef`, `TypeInfo`, `TyArena`, `MirTy`, etc.

`FunctionDef` and `MirModule` cannot be reused as-is because MIR-2's
`FunctionDef.body` is `Option<MirBody>`. The chosen approach: make
`FunctionDef` generic over the body type:
`FunctionDef<B>` where `B` is `MirBody` (mir-2) or `OssaBody` (mir-3).
Similarly `MirModule<B>`. This keeps one source of truth for function
metadata (name, params, kind, extern_info) while allowing different
body representations. The generic parameter is straightforward — it
appears only in `body: Option<B>`.

**Why:** The module-level types represent declarations, not
instructions. They are independent of the ownership model. Duplicating
the leaf metadata would create two sources of truth for type handling.
Making the container generic lets mir-3 benefit from improvements to
shared metadata without forking.

## Ops Restricted to @none Operands

**Decision:** `Op1`, `Op2`, `Op3` only accept @none operands. To use
a field of an @owned struct in an arithmetic op, the lowerer must first
borrow the struct and extract the trivial field from the borrow.

**Why:** Ops are pure computations on trivial values (integers, floats,
pointers). Allowing @owned operands in ops would require the verifier
to distinguish "read" from "consume" for ops — adding complexity for
no benefit. If an @owned value is used in an op, the lowerer should
extract the trivial component first. This makes the data flow explicit.

## Block Argument Verbosity

**Tradeoff acknowledged:** The block-parameter live-in invariant requires
every live @owned value to appear as a block argument at every join point.
In functions with many long-lived owned values and nested control flow,
this creates large argument lists (e.g., 10 owned values × 5 nested
if/else = 50 block args threaded per merge).

In comparison, MIR-2 used shared mutable slots — joins were free (just
write to the shared slot). OSSA's block args are cleaner semantically
but produce more verbose IR.

**Mitigations:**
1. The `canonicalize` pass coalesces block args when a block has a
   single predecessor (no actual merge needed).
2. In practice, most functions have 2-5 live owned values at any point.
   The explosion is worst-case, not typical.
3. Cranelift handles large block param lists efficiently (they map to
   register allocation constraints, not branches).
4. The semantic clarity (knowing exactly what's alive at each point)
   outweighs the verbosity cost for correctness.

## References

- Swift SIL: [Ownership.md](https://github.com/swiftlang/swift/blob/main/docs/SIL/Ownership.md)
- Swift SIL: [Instructions.md](https://github.com/swiftlang/swift/blob/main/docs/SIL/Instructions.md)
- Rust MIR: `Operand` / `Rvalue::Use(Operand)` pattern
- MIR-2 design rationale: `lib/kestrel-mir-2/docs/design-rationale.md`
