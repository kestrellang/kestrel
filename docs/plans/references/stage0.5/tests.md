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
| `pointer_mutating_write` | `Pointer(mutating: x).write(v)` then `x == v` |
| `pointer_to_noncopyable` | `Pointer(to: x)` where `x: not Copyable` — address capture borrows without moving; exact deinit count |
| `pointer_mutating_alias` | `Pointer(mutating: x)` captured while `x` is also passed to a `mutating` param — may-alias holds, last write wins |

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
- Existing E200-class: `pointer_mutating_immutable_arg` (a `let` passed to
  `Pointer(mutating:)`).

## Notes

- deinit-count detection pattern per `references-tests.md` §"Detecting UAF".
- No `ret_borrow`/escape tests here — stage1.
