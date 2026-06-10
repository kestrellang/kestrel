# Stage 1.5 — Tests

> **Status 2026-06-10**: the accessor + arm-decay waves below are
> LANDED (suite green incl. __llvm trials); the named-binding /
> `&`-pattern wave remains blocked on item 2 ratification.

Accessor matrix is writable now (semantics/errors defined). Named-binding
and `&`-pattern waves follow item 2's semantics ratification. All
execution tests get `// backends: cranelift,llvm`; no-clone pins use the
stage-1 `Tracked` clone-count/lives-count pattern.

## Place accessors — execution

- **Pure ref pair** (`ref` + `mutating ref`, Array-like COW struct):
  read / `= v` / `+= v` / mutating-member through the place; exact
  deinit counts; sibling-copy invisibility (makeUnique ordering).
- **Pure get/set** (computed, e.g. bit-packed): same operations through
  writeback; proves the fallback path end-to-end.
- **Cross-mix `get` + `mutating ref`**: the divergence pin — `get`
  normalizes, `+=` goes through the ref and SKIPS the normalization;
  asserts the documented coherence-contract semantics (intended
  behavior, not a bug).
- **Cross-mix `ref` + `set`**: borrowed reads (no clone), writes through
  the set hook (observable side effect).
- **No-clone pins**: clone-count == 0 for reads/RMW through `ref`
  accessors on Cloneable elements; NotCopyable element compiles for all
  place-context uses (strongest pin: misclassification is a compile
  error).
- **Property form**: `var first: T { ref {...} }` read + member-through
  + binding decay (copy).
- **Evaluation order**: `arr(i) = expr-that-mutates-arr` pins RHS-first
  (matches the shipped assignment-through-ref order).

## Place accessors — diagnostics

- Duplicate read provider (`get` + `ref`) / duplicate write provider
  (`set` + `mutating ref`).
- NotCopyable RMW with only get/set → copy-guard + "add a
  `mutating ref` accessor" hint.
- `ref` accessor in a protocol extension → rejected.
- `x(i) = v` with no write provider → no-setter error naming both
  providers.
- Declared `-> &T` subscript still E481.

## Subscript resolution (gates Array adoption)

- Inherent `Int64` ref subscript + Slice extension: `arr(5)` routes
  inherent, `arr(1..<3)` still reaches the extension (REQUIRES the
  fallback-tier decision — `compiler-arch.md`; this test is the gate).
- Read/write/RMW consistency: all three pick the same candidate for the
  same index type.
- `arr(checked: i)` / labeled forms unaffected.

## `at`/`mutableAt` removal

- Migrate `references/ret_borrow/*` + `references/escape/*` uses to
  subscript syntax in the same change that lands the Array accessors;
  no orphaned `at(index:)` callers remain.

## Arm-value decay (item 5)

- `match c { 1 => b.peek(), _ => 0 }` compiles; result owned.
- All-arms-refs (`if c { b.peek() } else { b.peek() }`) decays — owned
  result, refs end before merge (no E497).
- NotCopyable pointee as raw arm value → copy-guard error (wording pin).
- Cloneable pointee → exactly one clone per taken arm.
- Follow-up wave: array/tuple literal elements (`[b.peek(), x]`) —
  lands with the literal half, watch #127 interaction.

## Named ref bindings / `&` patterns — blocked on item 2 ratification

Sketch: binding survives statement boundaries (the `end_stale_refs_since`
carve-out pin); store-through writes the referent; `let s = r` copies;
block-exit ends the borrow; E497 on cross-merge attempts;
`&v` payload borrow reads in place (no bucket copy — deinit-count pin);
`&mutating v` requires mutable scrutinee root.
