# G18 — decisions

## 1. mir-lower, not hir-lower and not type-infer

The coercion needs to know the condition's **type** — is it already `lang.i1`,
is it `Bool`, is it some other conformer — and it needs to emit a call.

**hir-lower is ruled out by a hard rule, not a preference.** Its `AGENTS.md`
states that hir-lower has no type information and must never act as if it does.
A desugaring there could only be "wrap every condition in a `boolValue()` call
unconditionally", which would put a witness call in front of every `if` in the
language including `lang.i1` ones, and would then need to be un-done downstream.

**type-infer is ruled out by its own design.** `condition_check.rs` exists as a
post-inference *analyzer* precisely because the solver records no constraint on
the condition tyvar — its module doc says so: "primitive `lang.i1` doesn't
implement protocols, so a `Conforms` constraint would fail for direct `i1` usage
in conditions". Adding the coercion there would mean reversing that decision.

mir-lower is where the branch terminator is actually emitted, has full resolved
types, and already emits witness calls for the analogous implicit protocol
dispatches (`Matchable.matches` for string patterns,
`ArrayMatchable.matchLength` for array patterns). `coerce_condition_to_i1` is
modelled directly on `emit_string_match_test`.

## 2. One shared helper, two call sites — not six

`HirExpr::If` is the single HIR shape that `if`, `else if`, desugared `while`,
desugared `guard … else`, the non-binding link of `if let p = e, cond`, and the
non-binding link of multi-condition `while let p = e, cond` all lower into. One
insertion in `lower_if` covers all six. The `match` arm guard
(`DecisionTree::Guard`) is the only other production point. See
`problem.md`'s inventory for the full five-site `emit_branch` audit.

## 3. Gate on the `Builtin::Bool` ENTITY, never structurally

`std.core.Bool` is a nominal struct — `MirTy::Named`, **not** `MirTy::Bool`. So
"skip the call when the condition is already `lang.i1`" does not cover it, and
`Bool` is the condition type of essentially every `if` ever written.

This matters because **there is no MIR inliner**: `kestrel-mir/src/passes/` is
`clone_shim`, `copy_check`, `copy_propagation`, `drop_fix`, `drop_shim`,
`layout`, `thunk` — nothing that would fold a trivial witness call away. A call
here would be a real regression in cranelift and in debug builds.

The skip is sound, not a hack: `Bool.boolValue()` is by construction
`{ self.value }` over its single `lang.i1` field, and both backends already
scalarize `Bool` to i1, so branching on it directly computes exactly what the
witness would.

**A structural test was rejected.** "A single-field struct wrapping `lang.i1`"
would silently swallow a user's own `struct MyFlag { var value: lang.i1 }` whose
`boolValue()` need not be the identity — reintroducing the exact bug for the
exact shape most likely to hit it. A **name** test would be just as bad: a user
may declare their own `struct Bool` in their own module. The gate is entity
identity via `MirModule::bool_struct`, mirroring how `is_copyable_protocol` /
`is_cloneable_protocol` already replaced a name-suffix scan with a lang-item
entity for the same class of reason.

Both near-miss shapes are pinned by
`testdata/builtins/boolean_conditional/user_type_named_bool_is_not_the_lang_item.ks`,
which declares a user `Test.Bool` *and* a structurally-identical `MyFlag`, both
with inverting `boolValue()`s, and asserts each dispatches through the witness
(2 witness calls, exit 9). Name resolution from the root scope still finds
`std.core.Bool` for the lang item, so the user type is a different entity and is
never skipped.

Measured, not assumed: on a stdlib-heavy program that only branches on
`Bool` / `lang.i1`, the `boolValue` witness-call count after the fix is **0**,
and the `Branch` count is byte-identical before and after (1835). See the
verification section below.

## 4. Adding `@builtin(.Bool)` to `struct Bool`

`Builtin::Bool` previously resolved *purely* by name — `ResolveBuiltin` strategy
1 (`ResolveTypePath` on `"Bool"`) — with no attribute anchor and no
`from_attribute_name` arm. The `builtin_kind` comment even said so: "Bool is
resolved by name, doesn't need `@builtin`".

That is precisely the shape of the recorded lesson that **a lang item with no
attribute arm is silently inert**: `EntityBuiltin` returns `None`, the entity
never enters `BuiltinIndex`, and there is no strategy-2 fallback if the name
lookup ever finds the wrong thing or nothing.
`DefaultArrayLiteralType` sat inert exactly this way, masked by the type being
findable by name.

Adding the annotation plus the `"Bool" => Some(Self::Bool)` arm does **not**
change resolution order — name lookup still wins — it makes strategy 2 a real
fallback and puts `Builtin::Bool` in `BuiltinIndex`. It also turns the existing
`every_stdlib_builtin_annotation_is_recognized` test into a live guard over the
annotation, which was confirmed by removing the arm and watching the test fail
with `core/bool.ks: @builtin(.Bool)`.

The new `bool_builtin_round_trips_through_entity_builtin` unit test closes the
other half: name lookup can only tell you it found *something* called "Bool".
Round-tripping the resolved entity back through `EntityBuiltin` proves it is the
entity that carries `@builtin(.Bool)`.

## 5. The E101 backstop reuses the front-end's diagnostic, not "internal compiler error"

A non-conforming condition is normally rejected by `ConditionCheckAnalyzer`
before lowering. But `lower_to_mir_raw` / `lower_to_mir_stage` are called by the
LSP and by `kestrel dump mir` with **no guarantee the analyzer ran**, so the path
is reachable.

Modelled on `emit_move_out_of_borrow_backstop` (`body/mod.rs`), which reuses the
front-end's own E503 with ordinary user-facing wording — deliberately **not** the
"this is an internal compiler error; please file a bug report" framing of
`emit_no_enclosing_loop_backstop`. A non-Bool condition is a real source error,
not a compiler inconsistency. The conformance test uses the same definition as
`condition_check.rs:156-169` (`ConformingProtocols` contains the protocol), so
the two can never disagree about what they admit.

Double-emission was checked, not assumed. `kestrel build` and
`kestrel dump diagnostics` each report the E101 exactly **once** — they gate on
the analyzer and never reach MIR lowering. The LSP (`compiler_worker.rs`) only
ever calls `analyze_all`, never `lower_to_mir_*`, so it cannot duplicate either.
The one place both fire is `kestrel dump mir`, which deliberately bypasses the
error gate to show you the MIR anyway; a duplicate line in that dev-only dump is
the intended cost of the path being reachable at all.

Absent stdlib (`boolean_conditional_protocol == None`, e.g. a hand-built
unit-test module) falls back to the raw branch with no diagnostic — the same
answer lowering gave before, and the same fallback shape as
`emit_array_match_length`.

Still-generic conditions (`MirTy::TypeParam`, `MirTy::AssociatedProjection`)
skip the conformance test and emit the witness call directly: conformance was
already proved by the where clause, and monomorphization resolves the witness.
`MirTy::Error` returns unchanged with no diagnostic, to avoid cascading.

## 6. OSSA: the synthesized scalar stays tracked — `consume` was verified to break

The design proposed `self.consume(result)` on the synthesized scalar so it would
not ride through both arms and the merge. **This was tested against real
`kestrel dump mir -s verify` output and it fails**:

```
bug: internal compiler error: OSSA verify failed in 'Test.main' at bb0:
     @owned value ValueId(6) is live at block exit but never consumed
     (ty=TyId(149), own=Owned)
```

The branch terminator *reads* the condition but does not consume it, so
untracking it leaves an owned value with no consumer at the block exit.

**Neither `consume` nor the `extra_vals` fallback was needed.** Leaving the
value scope-tracked lets `lower_if`'s existing liveness threading carry it
through both arms to the merge and destroy it there, and OSSA verify is clean:

```
    %v6 = call @witness std.core.BooleanConditional.boolValue for Test.Inverted(@borrow %v5)
    branch %v6, bb1(%v2, %v4, %v6), bb2(%v2, %v4, %v6)
bb1(%v7: @owned Pointer[…], %v8: @owned Test.Inverted, %v9: @owned Bool):
    …
bb3(%v15: @owned Int64, %v16: …, %v17: …, %v18: @owned Bool):
    destroy_value %v18
```

So `lower_if` needed **no** new `extra_vals` / `destroy_extra_test_values`
mechanism — which is the reason it never had one. The cost is one extra block
param per arm, of a trivial `lang.i1` whose destroy is a no-op, and only on the
non-`Bool` path.

The `match` guard arm needed nothing at all: its existing `extra_vals` sweep
(`pattern.rs`) already collects every scope-owned guard temporary that is not a
match slot, and the synthesized scalar is exactly that.

The coercion is inserted **before** `end_stale_refs_since(mark)` in `lower_if`.
The witness call is part of computing the condition, so any single-use ref it
borrows must still be live at that point.

## 7. Placement of the resolution: `LowerCtx::new`, once

`boolean_conditional_protocol` and `bool_struct` are resolved once in
`LowerCtx::new`, beside the existing Copyable/Cloneable resolution, and carried
on `MirModule` — not queried per branch. A trivial stdlib program has ~1835
branch sites after mono.

`MirModule`'s exhaustive destructure in `mono/mod.rs` forced an explicit decision
about the new fields (they are `_`-bound): condition lowering is complete before
mono, and the resulting `Callee::Witness` is resolved by the ordinary witness
machinery.

## 8. Relationship to G16

G16 (relaxing what E101 accepts) is **orthogonal and can land in either order**.
G18's gate is evaluated at the call site against `ConformingProtocols`, the same
query `condition_check.rs` uses — so whatever E101 admits, G18 handles it, and
whatever E101 starts admitting after G16, G18 will dispatch correctly without
further change.

## Verification

- `cargo build --workspace` clean.
- `cargo test -p kestrel-hir`: 7/7, including
  `every_stdlib_builtin_annotation_is_recognized` (confirmed a live guard by
  removing the arm and watching it fail).
- New `kestrel-mir-lower` unit tests: `bool_condition_emits_no_boolvalue_witness_call`,
  `custom_conditional_emits_boolvalue_witness_call`,
  `bool_builtin_round_trips_through_entity_builtin` — all pass.
- MIR counts on a stdlib-heavy program, before → after:
  - `Branch` terminators: 1834 → 1834 (inverted repro), 1835 → 1835 (Bool-only).
    Identical, as required — this fix adds calls that *feed* branches, never
    branches.
  - `boolValue` witness calls: 0 → 0 on the Bool-only program (the gate fires in
    practice, not only in the unit test); 0 → 1 on the inverted repro.
- 21 `.ks` tests under `builtins/boolean_conditional/` (8 new execution tests,
  2 converted from `diagnostics` to `execution`).
