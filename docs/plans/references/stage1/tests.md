# Stage 1 — Tests

Q8 is decided (transparent place — `semantics.md`), so both halves are now
writable. The `references-prototype/references-tests.md` matrix uses
pre-decision syntax (`&x` at call sites, `*r` deref — neither exists in the
adopted surface); treat it as a list of *behaviors to pin*, not as test
sources.

## Diagnostics (writable now)

One per `errors.md` row E-REF-10..20 (14 and 18 dissolved — their tests
become *positive* execution tests below). Highest value:

- `return_borrow_of_local_rejected` — the silent-UAF class; must error, not
  compile. (`// skip:` until the escape checker lands, per the harness
  note in `references-tests.md`.)
- `return_mutating_of_shared_root_rejected` — the const-cast guard.
- `ref_function_value_rejected`, `throws_ref_return_rejected`,
  `generic_arg_ref_inference_rejected` — the three miscompile backdoors.
- `mutating_through_shared_ref_rejected` (E-REF-20) — `arr.first()` receiver
  of a mutating method; also pins that E205's "temporary" wording does *not*
  fire for `&mutating` results.
- `copy_out_notcopyable_rejected` — `let x = box.peek();` where the pointee
  is NotCopyable; existing copy-guard code, ref-aware wording.

## Execution (transparent place)

| Test | Pins |
|---|---|
| `return_ref_of_param_field` | `&self.field` accessor, checked Param root; read via decay `let v = obj.first();` and compare |
| `binding_decay_copies` | `let x = arr.first(); arr.set(0, …);` → `x` unchanged (decay is a copy, not a view) |
| `binding_decay_clones_cloneable` | decay of a `&String` element retains/clones — no aliasing, no double-free (the `string_literal_return_no_alias` class) |
| `return_mut_ref_write_through` | `arr.at(0).increment()` mutates the element in place — mutating method *through* the ref |
| `mut_ref_pass_through` | returned `&mutating` passed onward to `func bump(mutating x: Int64)`, original observed changed |
| `operator_through_ref` | `arr.first() == 42` — Equatable dispatch peels the ref |
| `interpolation_through_ref` | `"\(arr.first())"` — interpolation receiver see-through |
| `subscript_through_ref` | paren-call on a ref-typed receiver |
| `for_in_through_ref` | `for x in pair.left()` where `left() -> &Array[T]` — `iter()` is borrowed-self |
| `match_scrutinee_decays` | `match opt.peek()` on a Copyable pointee — copy-out, arms see an owned value |
| `consuming_method_copies_out` | consuming method on a ref receiver consumes a *copy*; original still alive |
| `scalar_ret_borrow_not_loaded` | the `resolve_scalar` miscompile pin; write-through observation distinguishes pointer-out from value-out |
| `array_first_accessor` | PointerDerived propagation through `Array.first()` |
| `mut_to_shared_coercion` | §10.1 — a `.mutatingValue`-rooted ref passed to a borrow param; place pass-through, no copy (borrow-args decision, `semantics.md`) |
| `intra_block_consume_while_borrowed` | existing `try_consume` gate, inherited free |

### No-clone pins (added 2026-06-09, borrow-args = place contexts)

The rows above pin everywhere a copy *must* happen; these pin where one
must **not**. Without them every borrowed-self/borrow-arg test passes
identically under a silently-inserted clone — the CopyValue→clone
mono-expand machinery is this codebase's most precedented failure mode.
The NotCopyable variants are the strongest pins: a misclassification as
value context fails at *compile time* via the copy guards, not as a
silent perf regression.

| Test | Pins |
|---|---|
| `borrowed_self_through_ref_no_clone` | Cloneable pointee with instrumented `clone()` (counter via Pointer cell); borrowed-self method called through `&T`; clone-count == 0 + exact deinit count |
| `borrowed_self_through_ref_notcopyable` | NotCopyable pointee; borrowed-self method through a ref **compiles** + exact deinit count |
| `borrow_param_ref_no_clone` | ref passed to a borrow-convention free-function param; clone-count == 0 |
| `borrow_param_ref_notcopyable` | NotCopyable pointee ref passed to a borrow param; compiles + exact deinit count |
| `mut_ref_write_through_no_clone` | Cloneable heap pointee mutated through `&mutating`; clone-count == 0 + exact deinit — kills the clone-mutate-writeback impostor that `mut_ref_pass_through` cannot distinguish |

Instrumented `clone()` note: the counting clone must payload-clone
properly — never `clone() { self }` (the known heap-payload aliasing
footgun).

## Harness

- Run via `/triage` only.
- **One `KESTREL_BACKEND=llvm` execution lane is required** for every
  `ret_borrow` test — Cranelift-only runs cannot see LLVM ABI mistakes
  (`references-gaps.md` §6).
- Detection mechanics (deinit-count, Pointer-cell readback, exit codes):
  `references-tests.md` §"Detecting UAF" still applies.
