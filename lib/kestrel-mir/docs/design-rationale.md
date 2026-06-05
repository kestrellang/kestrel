# Design Rationale

Why each decision was made. This is the "don't revert this" document for
the OSSA MIR.

## SSA Values Instead of Mutable Locals

The MIR uses SSA values. Every instruction produces a new `ValueId`.
Values are immutable once defined. Ownership is a property of the value
definition — `@owned`, `@guaranteed`, or `@none`. There is no post-hoc
reconstruction of "maybe initialized" state and no drop flags.

**Why:** The alternative is to store values in mutable `LocalId` slots,
where a local can be assigned, moved-from, reassigned, and dropped, and
ownership state must be reconstructed by forward dataflow analysis
(Dead/Live/Maybe per local per block). That analysis is necessarily
approximate: at control-flow join points, a local that's Live on one
path and Dead on another becomes Maybe, requiring runtime drop flags
and a chain of compensating passes (clone elaboration, drop
elaboration, drop-flag expansion) to recover ownership information that
was lost the moment the lowerer emitted untyped local assignments.

SSA makes ownership structural instead. Under the block-parameter
live-in invariant, the verifier is linear in the CFG size, and ownership
bugs in modeled values surface directly as verifier failures rather than
as silent leaks or double-frees.

## Three Ownership Kinds (Not Four)

Kestrel OSSA has three ownership kinds: `@owned`, `@guaranteed`, `@none`.
There is no `@unowned`.

**Why:** Swift SIL has a fourth kind, `@unowned`, which handles weak
references to class instances that could be deallocated while a
non-owning reference exists. Kestrel has no reference types in the
language (no `&T` as a value), no weak references, and is single-threaded.
There is no scenario where a value could be invalidated while a non-owning
reference exists. Adding `@unowned` would be dead complexity.

## No begin_access / end_access

Kestrel OSSA has no access markers and does no exclusivity checking.

**Why:** Swift SIL uses `begin_access [read|modify|init|deinit]` /
`end_access` to enforce the Law of Exclusivity (no simultaneous mutable
and immutable access to the same memory). Kestrel is single-threaded.
The only way to create aliasing in Kestrel is through Pointer operations,
which are explicitly unsafe. The language-level borrow model
(`ParamConvention::Borrow`/`MutBorrow`) ensures that callers don't hold
conflicting references — but this is enforced by the type system (param
conventions), not by MIR-level access markers. Adding access markers
would be correct but unnecessary complexity for a single-threaded
language.

## Call-Scoped Borrows (Today), Cross-Block Ready (IR)

The IR and verifier accept `@guaranteed` block parameters and cross-block
borrow scopes. The lowerer today only emits call-scoped borrows — begin
and end in the same block, bracketing a single call. Borrows don't flow
into block arguments, struct fields, or closures today.

**Why the split:** Kestrel has no user-facing reference types today.
Borrows exist only at function call boundaries. The lowerer exploits
this by emitting only the simplest borrow pattern. But the IR doesn't
enforce this restriction — it accepts `@guaranteed` block params with
borrow provenance tracking (`ValueDef.borrow_source`). This means
adding lexical lifetimes or NLL later is a lowerer change, not an IR
or verifier redesign.

The cost of accepting `@guaranteed` block params in the IR from day one
is near zero — the verifier already tracks `@owned` consumption per path,
and tracking `@guaranteed` provenance uses the same infrastructure. The
cost of retrofitting it later would be significant: reworking the block
param model, every terminator, the verifier, and codegen.

Swift SIL supports the full general case — borrows flowing through block
arguments ("reborrow phi arguments"), escaping into struct fields via
`store`, and spanning multiple basic blocks — at the cost of complex
borrow scope tracking with "enclosing values" and "borrowed-from"
annotations on phi arguments. Kestrel's IR is shaped to permit that
evolution without paying its full complexity up front.

**What the verifier does:** Tracks open borrows as a set per block. At
each block exit, every `@guaranteed` value must be either `end_borrow`'d
or forwarded as a `@guaranteed` block arg. The `borrow_source` field on
`ValueDef` propagates through block args and forwarding extractions. While
any value with a given `borrow_source` is live, that source cannot be
consumed (shared borrow) or read/consumed (mut borrow). Call-scoped
borrows trivially satisfy this — they open and close in the same block.

## Block Arguments Instead of Drop Flags

At every merge point, each predecessor explicitly passes `@owned` values
or destroys them before jumping. The block's parameter list declares what
values it expects. There is no ambiguity about initialization state and no
runtime flag.

**Why:** Block arguments are the standard SSA solution for the
merge-point problem. They are zero-cost at runtime (Cranelift block params
are register allocation constraints, not branches). They make the CFG
explicit about what values are alive at every point. Each block's
parameter list is the complete owned live-in set for that block. Every
value passed as a block argument is consumed by the jump (ending its
lifetime in the predecessor) and reborn as a new value in the successor
(starting a new lifetime).

The alternative — boolean drop flags allocated at join points where a
value might or might not be initialized — is function-wide mutable state.
A flag must be updated at every initialization and every move; its
ordering relative to overwrite-drops and scope-exit drops is fragile;
expanding each conditional drop into branch-on-flag blocks inflates the
CFG; and the branch is a runtime cost. Block arguments avoid all of this.

## CopyValue as an Instruction (Not a Pass)

`CopyValue` is an explicit instruction in the IR. The lowerer emits it
when it knows a copy is needed. If the lowerer emits too many copies, the
`copy_optimize` pass removes unnecessary ones — but this is an
optimization, not a correctness requirement.

**Why:** Putting the copy decision in the lowerer eliminates any ordering
dependency between a copy-elaboration pass and a drop-elaboration pass —
the kind of dependency where, if the copy analysis disagrees with the
drop analysis about which values are alive, the result is a leak or a
double-free. The lowerer knows at emit time whether a value will be used
again (it has the HIR scope), so it can decide directly. The
`copy_optimize` pass can then run at any point without affecting
correctness. And `CopyValue` is explicit in the IR — the verifier can
check that it's only used on Clone types, the codegen can emit the witness
call, and there's no hidden "this copy might become a move later"
semantic.

## DestroyValue at the Lowerer (Not a Pass)

The lowerer emits `DestroyValue` at every scope exit for every unconsumed
`@owned` value. This is a local decision using a scope-tracking stack —
no dataflow analysis.

**Why:** The lowerer has strictly more information than a post-hoc
analysis pass. It knows the scope structure from the HIR, it knows which
values it created, and it knows which ones were consumed. A forward
init-state analysis that tries to recover this after the fact reliably
misses cases — call-result temporaries, transitively droppable fields,
`Pointer.read()` copies — and each miss is a leak or a use-after-free.
The scope stack is ~50 lines of code, replacing the hundreds of lines of
init-state analysis, drop elaboration, and drop-flag expansion that the
dataflow approach requires.

And if the lowerer gets it wrong (forgets a `DestroyValue`), the OSSA
verifier catches it immediately: "unconsumed @owned value %42 at exit
of block bb7." The error message tells you exactly which value leaked
and where, rather than reporting a correct-but-uninformative "local is
Live at Return" that leaves you unsure whether the bug is in the lowerer
or in some downstream analysis.

## Trivial Types as @none (Excluded from Tracking)

Trivial types get `Ownership::None`. They are completely excluded from
ownership tracking. The verifier ignores them. `CopyValue`,
`DestroyValue`, `BeginBorrow`, and `EndBorrow` should not appear on
`@none` values (the verifier rejects them if they do).

**Why:** Most values in a typical function are trivial (integer
arithmetic, boolean conditions, pointer offsets). Tracking them through
ownership analysis is wasted work. Restricting the live-in bitset to only
`@owned` values keeps the verifier cheap.

## Forwarding Instructions Consume Aggregates

Extracting from an owned aggregate consumes that aggregate. When more than
one field is needed, `DestructureStruct` extracts all fields at once so
the aggregate is consumed exactly once. A single `StructExtract` from an
owned aggregate is only for cases where all unextracted fields are `@none`.
Extracting from a borrowed aggregate is a non-consuming projection and
produces a guaranteed value.

**Why:** This is how Swift SIL works, and it's the natural model for an
ownership-based IR. If you own a struct and want one field, you
destructure it — taking ownership of the field and destroying the rest.
This eliminates the need for clone-to-extract patterns.

For the initial implementation, the lowerer can continue using
whole-aggregate moves and defer partial extraction. The IR supports it;
the lowerer can adopt it incrementally.

## Module-Level Types Are Generic Over the Body

Body-independent type and declaration metadata — `StructDef`, `EnumDef`,
`ProtocolDef`, `WitnessDef`, `TypeInfo`, `TyArena`, `MirTy`, etc. — is
kept separate from the body representation. `FunctionDef` and `MirModule`
are generic over the body type: `FunctionDef<B>` and `MirModule<B>`, where
`B` is `OssaBody`. The generic parameter is straightforward — it appears
only in `body: Option<B>`.

**Why:** The module-level types represent declarations, not instructions.
They are independent of the ownership model. Keeping one source of truth
for function metadata (name, params, kind, extern_info) while letting the
body representation vary avoids duplicating the leaf metadata and creating
two sources of truth for type handling.

## Ops Restricted to @none Operands

`Op1`, `Op2`, `Op3` only accept `@none` operands. To use a field of an
`@owned` struct in an arithmetic op, the lowerer must first borrow the
struct and extract the trivial field from the borrow.

**Why:** Ops are pure computations on trivial values (integers, floats,
pointers). Allowing `@owned` operands in ops would require the verifier
to distinguish "read" from "consume" for ops — adding complexity for no
benefit. If an `@owned` value is used in an op, the lowerer should extract
the trivial component first. This makes the data flow explicit.

## Block Argument Verbosity

**Tradeoff acknowledged:** The block-parameter live-in invariant requires
every live `@owned` value to appear as a block argument at every join
point. In functions with many long-lived owned values and nested control
flow, this creates large argument lists (e.g., 10 owned values × 5 nested
if/else = 50 block args threaded per merge). This is the price of making
ownership structural rather than reconstructing it from shared mutable
slots, where joins are free but ownership is implicit.

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
