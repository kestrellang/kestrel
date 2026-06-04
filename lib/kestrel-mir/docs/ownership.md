# Ownership Model

How ownership flows through the OSSA IR: kinds, rules, forwarding,
verification invariants.

## Ownership Kinds

Three kinds form a simple lattice (no meet/join needed at the IR level):

```
     None (trivial)
      |
    Owned (affine)
      |
  Guaranteed (borrowed)
```

- **@none**: Int64, Bool, UInt8, Float64, raw pointers, thin function refs,
  Unit, Never. No copy_value, no destroy_value, no tracking. The value
  exists and can be freely used or ignored.

- **@owned**: String, Array, closures, any struct/enum with Clone or no
  copy behavior. Must be consumed exactly once on every path. Consuming
  uses: destroy_value, consuming call arg, return, store_init/store_assign,
  block arg passing, forwarding into aggregate construction.

- **@guaranteed**: Created by begin_borrow/begin_mut_borrow. The value
  is valid for the scope of the borrow. Cannot be consumed. Reads are
  free. Ends with end_borrow/end_mut_borrow.

## The Linear Ownership Rule

> Every @owned value has exactly one consuming use on every reachable
> control-flow path from its definition.

This is the central OSSA invariant. It prevents:

1. **Leaks**: An @owned value with no consuming use on some path is leaked.
   The verifier rejects this.

2. **Double-free**: An @owned value consumed on two paths that both
   execute is double-freed. The verifier rejects this.

3. **Use-after-free**: An @owned value used after its consuming use is
   a use-after-free. The verifier rejects this.

### What Counts as a Consuming Use

For an @owned value `%v`:

| Consuming use | Semantics |
|---------------|-----------|
| `destroy_value %v` | End lifetime, call destructor |
| `move_value %r, %v` | Rename (new ValueId, same value) |
| `call f(%v)` with `Consuming` convention | Transfer to callee |
| `return %v` | Transfer to caller |
| `store_init %addr, %v` | Transfer to uninitialized memory |
| `store_assign %addr, %v` | Destroy old memory contents, then transfer to memory |
| `jump bb(%v)` / `branch .., bb(%v)` | Transfer to block param |
| `struct { .f: %v }` | Forward into aggregate |
| `tuple(%v, ...)` | Forward into aggregate |
| `enum .Variant(%v)` | Forward into aggregate |
| `array [%v, ...]` | Forward into aggregate |
| `apply_partial f(%v)` | Capture into closure |

After any consuming use, `%v` is dead. Any subsequent read or consume
is a verifier error.

### What Counts as a Non-Consuming Use (Read)

| Non-consuming use | Semantics |
|-------------------|-----------|
| `copy_value %r, %v` | Produce independent copy; %v stays alive |
| `begin_borrow %r, %v` | Borrow; %v stays alive but frozen |
| `discriminant %r, %v` | Read integer tag; %v stays alive |
| `op1/op2/op3` arguments | Must be @none; ops only operate on trivial values |
| `call f(%v)` with `Borrow`/`MutBorrow` | Borrow for call duration |
| `load %r, %addr` | Read trivial value from address |
| `copy_addr %r, %addr` | Produce independent copy from memory; memory stays initialized |
| `begin_borrow_addr %r, %addr` | Borrow initialized memory; memory stays initialized |

After a non-consuming use, the source value is still alive or the source
memory remains initialized and can be used again (within ownership
constraints — e.g., no use during borrow freeze).

**Note on `discriminant`:** This is the only non-consuming read allowed
directly on an @owned value without a borrow scope. All other non-trivial
reads of @owned values go through `begin_borrow`. The exception exists
because `Switch` needs the integer tag to determine which arm to take,
but the @owned enum itself must survive to be passed as a block argument
into the matching arm for payload extraction. Requiring a borrow scope
around the discriminant read would force the enum to stay borrowed through
the switch terminator and into successor blocks — which would require
cross-block borrows for every match on an owned enum. The verifier must
whitelist `Discriminant` as a non-consuming use of @owned operands.

## Forwarding

### Forwarding Rule

**Construction**: Result ownership = most restrictive operand ownership.

```rust
fn forwarding_ownership(operands: &[Ownership]) -> Ownership {
    if operands.iter().any(|o| *o == Ownership::Owned) {
        Ownership::Owned
    } else if operands.iter().any(|o| *o == Ownership::Guaranteed) {
        Ownership::Guaranteed
    } else {
        Ownership::None
    }
}
```

If any operand is @owned, the aggregate is @owned (it contains owned
resources that need cleanup). If all operands are @none, the aggregate
is @none.

**Destruction**: When destructuring an @owned aggregate, each extracted
field inherits ownership from its type. The aggregate is consumed once.
Fields the source program does not keep must still be extracted by the
lowerer and destroyed if they are @owned.

### Mixed Trivial + Non-Trivial

```
%n = literal Int64 42                    // @none
%s = call String.init("hello")           // @owned
%pair = struct Pair { x: %n, y: %s }    // @owned (because %s)
```

Destructuring:
```
(%x, %y) = destructure_struct %pair     // @none, @owned
// %pair consumed. %x freely usable. %y must be consumed.
```

A single-field `struct_extract` from an owned aggregate is legal only when
all unextracted fields are @none. For multiple fields, or when any
unextracted field needs cleanup, use `destructure_struct` so the aggregate
is consumed once and unwanted owned fields can be destroyed explicitly.

For address-backed values, choose the memory operation explicitly:
```
%x = load %pair_addr.x                  // @none
%y = copy_addr %pair_addr.y             // @owned clone/copy from memory
```

### Forwarding and @guaranteed

When passing a @guaranteed value through a forwarding instruction, the
result is @guaranteed. This is useful for reading fields of a borrowed
struct without copying:

```
// %borrowed_pair is @guaranteed (from begin_borrow)
%name = struct_extract %borrowed_pair, .name   // @guaranteed
// %name is valid as long as the borrow scope is open
// No destroy_value needed (it's borrowed, not owned)
```

## Borrow Scoping Rules

### Invariant 1: Nesting

Borrow scopes must be properly nested. If `begin_borrow %b, %v`
is followed by another `begin_borrow %c, %v`, then `end_borrow %c`
must come before `end_borrow %b`.

### Invariant 2: No Consume During Borrow

While a borrow is active on `%v` (between begin_borrow and end_borrow),
`%v` cannot be consumed (no destroy, no move, no return, no forward).
It can still be read by ops.

For mutable borrows (begin_mut_borrow), the constraint is stronger:
`%v` cannot be read OR consumed until end_mut_borrow. The borrow has
exclusive access.

### Invariant 3: Borrow Scopes

The verifier tracks open borrows as a set per block. A borrow must be
closed (via `end_borrow` / `end_mut_borrow`) on all paths, OR forwarded
as a `@guaranteed` block argument to a successor. The verifier does NOT
assert same-block begin/end — it checks that all borrows are closed or
forwarded at every block exit.

Today, the lowerer only emits call-scoped borrows (begin and end in the
same block, bracketing a single call):

```
%ref = begin_borrow %owned_value
call some_function(%ref)    // convention = Borrow
end_borrow %ref
```

This pattern trivially satisfies the verifier. But the IR and verifier
accept cross-block borrows so that lexical lifetimes can be added later:

```
// Future: borrow survives through control flow
%ref = begin_borrow %str            // @guaranteed, provenance = %str
branch %cond, bb1(%ref), bb2(%ref)  // @guaranteed block arg

bb1(%r: @guaranteed):               // inherits provenance from %ref
    call print(%r)
    jump bb3(%r)

bb2(%r: @guaranteed):
    call log(%r)
    jump bb3(%r)

bb3(%r: @guaranteed):
    end_borrow %r                   // closes the borrow on all paths
    // %str is unfrozen here — can be consumed again
```

The verifier enforces: while any value with `borrow_source = %str` is
live, `%str` cannot be consumed. This is checked using the provenance
field on `ValueDef`, which propagates through block args and forwarding
extractions.

### Invariant 4: Reborrow

A @guaranteed value (from begin_borrow or from a forwarding projection
like struct_extract on a borrowed aggregate) can be passed directly as
a Borrow or MutBorrow call argument without wrapping in another
begin_borrow. The value is already borrowed — re-borrowing it would
be redundant. The verifier accepts @guaranteed values in Borrow and
MutBorrow call argument positions.

`EndBorrow` and `EndMutBorrow` terminate both SSA-value borrows
(from `BeginBorrow`/`BeginMutBorrow`) and address borrows (from
`BeginBorrowAddr`/`BeginMutBorrowAddr`). The verifier matches each
end to its corresponding begin by the `ValueId`.

### Current Scope Simplification

Kestrel has no user-facing reference types today. Borrows exist only at
call boundaries (ParamConvention::Borrow/MutBorrow). The lowerer
exploits this by emitting only call-scoped borrows. This means:

1. Borrow scopes happen to be local to a single block (today).
2. The lowerer doesn't emit @guaranteed block arguments (today).
3. No NLL analysis is needed (today).
4. No lifetime annotations on types.

The IR and verifier are not restricted to this — they accept
@guaranteed block params with provenance tracking. When Kestrel adds
user-facing reference types, the lowerer starts emitting cross-block
borrows and @guaranteed block args. The IR, verifier, and codegen
don't change.

This is a massive simplification over Swift SIL, which must track
`@guaranteed` values through block arguments ("reborrow phi arguments")
and across function boundaries.

## Trivial Type Bypass

Values of trivial types (`Ownership::None`) are completely excluded
from ownership tracking. The verifier ignores them. They never appear
in `copy_value`, `destroy_value`, `begin_borrow`, or `end_borrow`
instructions.

This is a significant performance win for the verifier: most values in
a typical function are trivial (integer arithmetic, boolean conditions,
pointer offsets). Only String, Array, closure, and custom struct/enum
values need ownership tracking. Trivial values are skipped entirely —
they carry no init_state, no liveness, and no per-use filtering.

## Partial Moves

OSSA supports partial extraction through destructuring: a single field
can be moved out of an aggregate while the rest are destroyed.

```
// Move just one field out, destroy the rest
(%name, %age, %addr) = destructure_struct %person
// %person is consumed once. %name and %addr are @owned; %age is @none.
destroy_value %addr
// %name remains live and must be consumed later.
```

The key insight: an owned aggregate is consumed once. The lowerer must
extract all fields it needs and destroy the rest.

The lowerer may also move an aggregate whole (`move_value` the entire
struct) and defer partial extraction. The IR supports both; the lowerer
doesn't have to use partial extraction immediately.

## Ownership Guarantees

The invariants above combine into a small set of guarantees the OSSA
model enforces. Each one falls out of the linear ownership rule plus the
verifier; together they make whole classes of memory bug unrepresentable.

### No aliased owners from memory reads

A plain `Load` is valid only for trivial `@none` values. For non-trivial
memory, the lowerer must choose one explicit operation: `copy_addr` to
create an independent owned copy, `take` to move the value out and leave
memory uninitialized, or `begin_borrow_addr` for a temporary guaranteed
borrow. The verifier rejects a non-trivial `Load`, so the IR cannot
accidentally create two owners for the same memory — every owned value
read from an address has an explicit, single-owner provenance.

### No leaks

There is no "droppable set" to keep in sync with type information. Every
@owned value must be explicitly consumed on every path. If the lowerer
forgets to emit `destroy_value` — even for a deeply nested type whose
fields are droppable — the verifier catches it ("unconsumed @owned value
at block exit"). If the verifier passes, leaks are impossible.

### Call results consumed exactly once

Call results are new @owned ValueIds, subject to the same linear rule as
any other owned value. Moving a result into another instruction consumes
it; otherwise it must be destroyed. The verifier catches both leaks
(never consumed) and double-frees (consumed twice), so temporaries that
flow out of calls cannot silently leak or be freed twice.

### Ownership flow encoded in the CFG, not in flags

There are no runtime drop flags. Block arguments carry ownership across
merge points: at every join, each predecessor explicitly passes its owned
values forward or destroys them before branching. Because the CFG itself
encodes which values are live on which path, there is no fragile ordering
between flag updates and drops to get wrong — conditional ownership is a
structural property of the block-argument graph.
