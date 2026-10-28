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

A new lint (E508) catches a `Cloneable` type that looks like it needs the
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
| Analyzers | `observabledrop` in `KNOWN_ATTRIBUTES`. (E508 is *not* an analyzer — see below.) |
| Post-mono | E508 lint `copy_lifetime::unmarked_observable_drops`, reported by `kestrel-compiler` (`Compiler::mono_warnings`, printed by `kestrel build`). |
| MIR lowering | `record_memory_only_drop` → `record_observable_drop`; `MirModule::memory_only_drop` → `observable_drop`. |
| Mono | Carries `observable_drop` across (rename). |
| copy_lifetime | `TypeFacts::memory_only` → `TypeFacts::observable` (inverted, walks pointees); callers flip. |
| Stdlib | Remove `@memoryonlydrop` ×8; add `@observabledrop` to `TcpStream`. |

## Error cases

| Condition | Diagnostic |
|-----------|-----------|
| `Cloneable` struct/enum, not `@observabledrop`, whose `deinit` (transitively, through Kestrel calls) reaches an `@extern` function other than the C allocator and `mem*` functions | **E508** warning: "`Cloneable` type `{T}` has a `deinit` that calls `{f}`; mark it `@observabledrop` so its copies are not dropped early" — label on the type name ("copies of this type may be dropped before the end of their scope"), note naming the extern reached. `{f}` is the first source-level function on the call path from the deinit (`println`, `closeFd`). |

## Edge cases

- **Where the lint runs:** after monomorphization, where every call —
  generic and protocol calls included — is resolved to a concrete function, so
  the walk is exact: a `deinit` that only prints is caught through
  `println → writeLine[W] → W.write` (`W = Stdout`). The first version was an
  analyzer and could not see through those calls. Trade-off: E508 appears at
  `kestrel build` time only, not in the editor, and only for types the program
  instantiates. Calls through function values are still skipped.
- **Crash paths:** functions returning `!` (`fatalError`) are not followed —
  an allocation that can abort with a message would otherwise flag every
  deinit that builds a string.
- **Tests:** E508 is checked by *execution* tests: a `// WARN:` annotation (or
  the `// warnings: checked` header, to assert none) makes the harness match
  the build's post-mono warnings two ways.
- **Memory:** the C allocator and `mem*` externs (`malloc`, `calloc`, `realloc`, `free`, `memcpy`, `memmove`, `memset`, `memcmp`) are allowed; any other extern
  counts as observable.
- **Recursion / cycles** in the call walk: visited set.
- **Generic deinits** (`RcBox[T]` dropping its `T`): contents are handled by
  the type-level propagation (a type containing an observable type needs no
  mark of its own), not by the lint.
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
