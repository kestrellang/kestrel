# Indirection — tests

> **Shipped: 20/20 green** (8 execution × cranelift+llvm + 4 diagnostics).
> Testdata under `lib/kestrel-test-suite/testdata/indirection/`. Run via
> `/triage` only (`triage 'indirection.*'`). Execution tests carry
> `// test: execution` / `// stdlib: true` / `// backends: cranelift,llvm` and a
> `@main func main() -> lang.i64` returning 0 on success; diagnostics tests carry
> `// test: diagnostics` and inline `// ERROR` annotations.

## Shipped tests

| File | Kind | Pins (rule) |
|---|---|---|
| `peel/member_read.ks` | exec | field read peels to the pointee (R1) |
| `peel/method_call.ks` | exec | non-mutating method dispatches to the pointee (R1, `pointeeRef`) |
| `mutate/field_write.ks` | exec | `w.f = v` writes through `pointeeMutRef`, visible on re-read (R4) |
| `mutate/mutating_method.ks` | exec | mutating method persists its effect (R4) |
| `cow/cow_barrier.ks` | exec | CoW fork in `pointeeMutRef`: a shared `CowBox` alias is NOT mutated (R4) |
| `shadow/wrapper_wins.ks` | exec | `rc.clone()` is RcBox's (shares); `rc.pointeeRef().clone()` is the pointee's (deep) (R2/R3) |
| `nested/two_levels.ks` | exec | `RcBox[RcBox[Account]]` reaches the leaf through two peels, read + write (R5) |
| `operators/via_extend_works.ks` | exec | `==`/`!=` work via `extend RcBox: Equatable` (R7) |
| `reject/no_argument_coercion.ks` | diag | `f(rc)` where `f` wants the pointee is a clean type error (R6) — the flagship pin |
| `operators/without_extend_rejects.ks` | diag | `rc1 == rc2` without the `extend` rejects (`!: Equal`) — operators don't peel (R7) |
| `readonly/write_rejected.ks` | diag | write through a read-only `Indirection` wrapper → E208 (R8/D2) |
| `diagnostics/not_found_names_pointee.ks` | diag | `rc.nonexistent` errors on the pointee type (R8/D1) |

## Coverage matrix

| Rule (semantics.md) | Tests |
|---|---|
| R1 receiver peel | `peel/*`, `mutate/*` |
| R2 wrapper-wins | `shadow/wrapper_wins` |
| R3 `.pointeeRef()` reach | `shadow/wrapper_wins` |
| R4 mutability routing + CoW | `mutate/*`, `cow/cow_barrier`, `readonly/write_rejected` |
| R5 transitivity | `nested/two_levels` |
| R6 no arg coercion | `reject/no_argument_coercion` |
| R7 operators via extend | `operators/*` |
| R8 errors | `readonly/*`, `diagnostics/*` |
| R9 identity/copy/drop | exercised implicitly by `shadow` (clone) + `cow` (alias) |

## Test-authoring notes (learned during bring-up)

- A pointee used with `CowBox` (or reached via `rc.pointeeRef().clone()`) must
  declare `: Cloneable` **with an explicit `clone()`** — a plain Copyable struct
  does NOT auto-satisfy `T: Cloneable` (E454/E458 otherwise).
- Validate manually with a **release** binary
  (`cargo build -p kestrel --release`), not `target/debug/kestrel`: a debug build
  trips a pre-existing, unrelated copy-bound ICE on any `RcBox`/`Pointer` with a
  Copyable-struct payload. Triage runs release, so it's unaffected.

## Follow-up waves (not yet written)

The shipped set is a representative subset. Still worth adding:

- **No-clone pins** — read through `pointeeRef` performs zero clones (the
  `Tracked` heap-counter pattern from `references/no_clone/`).
- **`Formattable` via extend** — `"\(rc)"` works with `extend RcBox: Formattable`,
  rejects without.
- **Read-only mutating-method reject** — calling a `mutating` pointee method
  through a read-only wrapper (D2 via a method, not just a field write).
- **Nested stops at non-`Indirection`** — a chain whose pointee is a plain struct
  stops there (member-not-found at the leaf).
- **Interaction audit** — copy semantics unchanged (R9); NonCopyable field
  through the peel; the `Target = &U` generic-body case.
