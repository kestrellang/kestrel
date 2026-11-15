---
name: mir
description: 'How the compiler''s MIR works and how to debug or change it: OSSA ownership, lowering from HIR, the pass pipeline and its dump stages, monomorphization, drop and clone shims, copy elimination and last-use moves. Use for MIR verify errors, leaks, double frees, wrong copies, or any change under kestrel-mir or kestrel-mir-lower.'
---

# MIR

Kestrel's MIR is **OSSA** (ownership SSA). Values are SSA `ValueId`s, control
flow joins through block parameters, and every value has an ownership kind.
There are no places, rvalues or statements, so anything that mentions
`Place::local` or `Rvalue` describes the old MIR, which was deleted in
June 2026.

Two crates:

| Crate | Owns |
|---|---|
| `kestrel-mir-lower` | HIR + `TypedBody` → OSSA. `src/body/` lowers bodies, `src/items/` lowers declarations and witness tables |
| `kestrel-mir` | The IR types, the verifier, the pass pipeline (`src/passes/`) and monomorphization (`src/mono/`) |

The post-mono steps are run by `Compiler::monomorphize_mir_until` in
`kestrel-compiler/src/lib.rs`, not by `kestrel-mir` itself.

## Which file to open

| Question | Read |
|---|---|
| What does instruction X mean, what are the ownership kinds? | `lib/kestrel-mir/docs/ir.md`, `ownership.md` |
| How does a construct lower (scopes, copy vs move, branches, calls)? | `lib/kestrel-mir/docs/lowering.md`, then the `kestrel-pipeline` skill for the per-variant function |
| What does each pass do and what does the verifier check? | `lib/kestrel-mir/docs/passes.md` (its diagram predates `drop_fix`, `clone_shim` and copy-prop; the stage list below is current) |
| How does codegen consume MIR? | `lib/kestrel-mir/docs/codegen.md` |
| Why is it built this way? | `lib/kestrel-mir/docs/design-rationale.md` |
| Rules you must not break | `lib/kestrel-mir/AGENTS.md`, `lib/kestrel-mir-lower/AGENTS.md` |

## The model in five rules

1. **Three ownership kinds.** `@owned` values must be consumed exactly once.
   `@guaranteed` values are borrows, valid until their `end_borrow`. `@none`
   values are trivial (integers, pointers) and aren't tracked.
2. **The lowerer decides copy vs move and drops.** It emits `copy_value` for
   every owned read that isn't a last use and `destroy_value` at scope exit.
   No later pass invents ownership. Passes only remove redundant pairs.
3. **Branches thread values through block parameters.** A value that is live
   across an `if` or `match` is passed to the merge block, which is why a
   `ValueId` held across a branch goes stale (see `lower_expr_holding` in
   `body/expr.rs`, and "Live value threading" in the lower crate's AGENTS).
4. **Generic until mono.** Pre-mono bodies use type parameters and witness
   calls. `mono` instantiates every reachable function at concrete types,
   resolves witnesses, interns vtables and legalizes fat pointers.
5. **Copies and drops become calls late.** After mono, `expand` turns
   `copy_value` / `destroy_value` on non-trivial types into calls to the
   type's clone and drop functions. Keep the two directions symmetric.

## The pipeline

`passes::Stage` in `lib/kestrel-mir/src/passes/mod.rs` is the single source of
truth for the order. Each stage is a dump point:

| Stage (`-s`) | What runs | Where |
|---|---|---|
| `raw` | Lowering output, nothing else | `kestrel-mir-lower` |
| `drop-fix` | Fill in drop behavior for types with droppable fields but no `deinit` | `passes/drop_fix.rs` |
| `thunk` | Calling-convention wrappers for partial applications | `passes/thunk.rs` |
| `drop-shim` | Synthesize `__drop$T` | `passes/drop_shim.rs` |
| `clone-shim` | Synthesize `__clone$T` for copyable aggregates without a user `clone()` | `passes/clone_shim.rs` |
| `layout` | Sizes of non-generic types | `passes/layout.rs` |
| `verify` | `verify_ossa` + `copy_check` (no copies of `not Copyable` values). The default dump point | `verify.rs`, `passes/copy_check.rs` |
| `mono` | Instantiate, resolve witnesses, vtables, fat pointers | `mono/` (`collect.rs`, `witness.rs`, `vtable.rs`, `fat.rs`) |
| `copy-prop` | Drop redundant copy/destroy pairs, shorten copy lifetimes, move out at last use | `passes/copy_propagation.rs`, `passes/copy_lifetime/` |
| `expand` | Copies and destroys become clone and drop calls; then the duplication audit and `verify_mono` | `mono/expand.rs`, `mono/audit.rs`, `mono/verify.rs` |

Adding or reordering a pass means updating four places together. The list is
in `lib/kestrel-mir/AGENTS.md` under "Pass pipeline & stage dumping".

## Debugging a MIR problem

1. **Reproduce in one small file**, then find the first stage where the output
   is wrong:

   ```bash
   kestrel dump mir -s raw repro.ks          # what lowering produced
   kestrel dump mir -s all -f main repro.ks  # every stage, functions whose name contains "main"
   kestrel dump mir repro.ks                 # the verify stage (aborts on a verify error)
   ```

   Every stage except `verify` is best-effort: it prints the module and sends
   verify errors to stderr, so you can look at a malformed body.
2. **Pick the layer from the stage.** Wrong at `raw` is a lowering bug
   (`kestrel-mir-lower/src/body/`). Correct at `verify` but wrong at `mono` is
   instantiation or witness resolution. Wrong only after `copy-prop` is a copy
   elimination or last-use bug. A leak or double free that appears at `expand`
   is usually an asymmetric copy and destroy.
3. **Trace with the MIR categories:** `KESTREL_DEBUG=drop-fix,last-use-move,copy-prop,copy-lifetime,clone-shim,take-alias,escape`
   for ownership; `audit-dup` reports silent bitwise duplications after
   `expand`; `instantiation` and `vtable` cover mono; `llvm-verify` dumps LLVM
   functions that fail verification.
4. Then follow the `debug` skill's protocol: one hypothesis, one fix, verify,
   and stop after three failed attempts.

## Symptoms and where they usually come from

| Symptom | Usually |
|---|---|
| Verify: `@owned` value consumed twice or never | A lowering path that copies or forgets a destroy; check the scope entries in `body/mod.rs` |
| `ValueId not in value_map` in codegen | A value used without a definition; the verifier should catch it, so run `-s verify` |
| Value used after a branch has the wrong contents | A `ValueId` held in Rust across an `if`/`match`; use `lower_expr_holding` |
| Use-after-free or double free at runtime | A bitwise duplication without a clone; run with `KESTREL_DEBUG=audit-dup` |
| A COW mutation deep-copies unexpectedly | A copy that outlives its last read; see `passes/copy_lifetime/` |
| Wrong witness picked for a generic protocol | `mono/witness.rs`; match on the protocol's type args (see the kestrel-mir AGENTS) |
| `E508` warning after build | A `Cloneable` type whose `deinit` calls an observable extern; mark it `@observabledrop` so copy-lifetime passes keep its drops at scope end. It only shows in execution tests |

## Rules that bite

- Never decide Copyable / Cloneable yourself. Route through `kestrel-copy-fold`.
- Per-type facts in mono are keyed by `(Entity, type_args)`, never the nominal
  alone.
- Read fatness from value types, not op payloads (`mono/fat.rs`).
- Closure captures come from `kestrel-type-infer/src/captures.rs`. Don't
  recompute them in lowering.
- `body/place.rs` is the only place resolver.
- Bodies lower before witness tables exist. Don't look one up during body
  lowering.

Each rule has its reasoning in the two AGENTS files.
