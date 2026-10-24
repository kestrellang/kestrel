# Observable Drop Design

## Overview

The copy lifetime passes (`lib/kestrel-mir/src/passes/copy_lifetime/`) drop an
implicit copy right after its last read instead of at the end of its scope, so a
copy no longer keeps copy-on-write storage shared (the difference between an
O(n) and an O(n²) loop). Dropping a copy earlier is invisible unless the drop
itself does something observable — closing a socket, deleting a file.

Until now the passes allowed early drops only for types *opted in* with
`@memoryonlydrop` (8 stdlib storage types). This inverts the default: early
drops are allowed **unless** a type *opts out* with `@observabledrop`. The
opt-out list is tiny because early drops only ever touch *copies*:

- `not Copyable` types (`File`, `TcpListener`, lock guards) are never
  implicitly copied, so they are never affected.
- Copyable (bitwise) types have no `deinit` by language rule.
- What remains is `Cloneable` types whose `deinit` has an external effect —
  in the stdlib, only `TcpStream`.

A new lint (E507) catches a `Cloneable` type that looks like it needs the
attribute but lacks it.

## Syntax

```kestrel
/// Shared handle; the last drop closes the connection.
@observabledrop
public struct TcpStream: Readable, Writable, Cloneable { ... }
```

An attribute, not a protocol: it changes nothing the type checker sees (no
`where T: ...` use, no methods) — it only tells the optimizer to leave the
type's lifetimes alone. Lowercase single word, matching `@fileconstant`.

## Semantics

- **Observable type:** a type is observable if it is marked `@observabledrop`,
  or if anything it can drop is observable: its stored fields, enum payloads,
  type arguments, and pointer pointees. Pointees matter because `RcBox[T]`
  stores only a `Pointer`; `RcBox[TcpStream]`, `Array[TcpStream]` and
  `struct Server { var conn: TcpStream }` are all observable.
- **Copy lifetime passes:** `drop_at_last_use` never shortens a copy of an
  observable type. `owned_params` and `borrow_copies` are unaffected — they
  change no drop timing (a move hands the value over; a borrow removes the copy
  entirely).
- The attribute on a non-copyable type is allowed and has no effect.
- Originals and explicit `.clone()` calls were never shortened; unchanged.

## Pipeline impact

| Stage | Change |
|-------|--------|
| Lexer / Parser / AST | None — attributes are generic. |
| Analyzers | `observabledrop` in `KNOWN_ATTRIBUTES`; new DeclCheck `observable_drop` (E507). |
| MIR lowering | `record_memory_only_drop` → `record_observable_drop`; `MirModule::memory_only_drop` → `observable_drop`. |
| Mono | Carries `observable_drop` across (rename). |
| copy_lifetime | `TypeFacts::memory_only` → `TypeFacts::observable` (inverted, walks pointees); callers flip. |
| Stdlib | Remove `@memoryonlydrop` ×8; add `@observabledrop` to `TcpStream`. |

## Error cases

| Condition | Diagnostic |
|-----------|-----------|
| `Cloneable` struct/enum, not `@observabledrop`, whose `deinit` (transitively, through Kestrel calls) reaches an `@extern` function other than the C allocator and `mem*` functions | **E507** warning: "`Cloneable` type `{T}` has a `deinit` that calls `{f}`; mark it `@observabledrop` so its copies are not dropped early" — label on the type name ("copies of this type may be dropped before the end of their scope"), note naming the extern reached. `{f}` is the function the deinit calls directly on the path to the extern (`close`, not `libc_close`). |

## Edge cases

- **Unresolvable calls** (protocol calls, closures, function values) in a
  `deinit`: the lint ignores them. It is a helper, not a safety gate; false
  alarms would teach people to ignore it. Known consequence: a `deinit` that
  only prints is not flagged (`println` dispatches through `Formattable` /
  writer protocol calls before reaching `write`).
- **Memory:** the C allocator and `mem*` externs (`malloc`, `calloc`, `realloc`, `free`, `memcpy`, `memmove`, `memset`, `memcmp`) are allowed; any other extern
  counts as observable.
- **Recursion / cycles** in the call walk: visited set.
- **Generic deinits** (`RcBox[T]` dropping its `T`): implicit drops are not
  calls in HIR, so the lint does not see them. Contents are handled by the
  type-level propagation, not the lint.
- **Copyable types with a `deinit`** (the bug class `TcpListener` hit): out of
  scope; tracked as a separate task.
- **`@observabledrop` on `not Copyable` types:** allowed, no effect, no warning.

## Open questions (resolved)

- *Protocol or attribute?* Attribute — see Syntax.
- *Opt-in (`@memoryonlydrop`) or opt-out?* Opt-out: 1 stdlib mark instead of 8,
  user types get the optimization by default; the lint covers the flipped
  failure direction.
- *Name?* `@observabledrop`. "Effect" is reserved for `throws`/`yields`/`async`
  (`docs/future_effects.md`).
- *Could an "original is still alive" check replace the attribute?* No: the
  wins come from mutating the original while a copy exists, which is exactly
  when the original can let go of the shared box.
