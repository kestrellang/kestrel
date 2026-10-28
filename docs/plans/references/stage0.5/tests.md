# Stage 0.5 — Tests

Location: `lib/kestrel-test-suite/testdata/references/` (new tree). Run via
`/triage` only. Parameters everywhere use **conventions** (`x: T`,
`mutating x: T`) — no `&` appears in any accepted position this stage. (The
`references-prototype/references-tests.md` matrix predates the syntax
decisions — treat it as behaviors to pin, do not copy its spellings.)

## Execution

| Test | Pins |
|---|---|
| `pointer_to_roundtrip` | `Pointer(to: x).read() == x` |
| `pointer_write_through` | `var x` then `Pointer(to: x).write(v)` then `x == v` — the sole init is write-capable (§10.2 revised, no `mutating:` twin) |
| `pointer_to_noncopyable` | `Pointer(to: x)` where `x: not Copyable` — address capture borrows without moving; exact deinit count |
| `pointer_alias_with_mutating_param` | `Pointer(to: x)` captured while `x` is also passed to a `mutating` param — may-alias holds, last write wins |

No `&`-param or sugar-equivalence executions exist: parameter behavior is
already pinned by the existing `mutating`-param suites, and this stage does
not touch it.

## Diagnostics

One file per `errors.md` row:

- E-REF-01: `param_ref_rejected` (`x: &T`), `param_mut_ref_rejected`
  (`x: &mutating T`), `function_type_param_ref_rejected`
  (`(&mutating T) -> R`).
- E-REF-02..09: `return_ref_rejected`, `binding_ref_rejected`,
  `field_ref_rejected`, `tuple_ref_rejected`, `generic_arg_ref_rejected`,
  `function_type_return_ref_rejected`, `nested_ref_rejected`,
  `expression_amp_rejected`.

No place-mutability diagnostics to test: `Pointer(to:)` accepts any place
(`errors.md`).

## Notes

- deinit-count detection pattern per `references-tests.md` §"Detecting UAF".
- No `ret_borrow`/escape tests here — stage1.
