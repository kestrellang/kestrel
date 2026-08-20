# G12 — decisions

## 1. Home: widen `control_flow.rs` into two tiers, same file

Two alternatives were rejected:

- **`util.rs`** — pinned by `AGENTS.md` to *span extraction and entity info*.
  Divergence is neither.
- **a new `body/divergence.rs`** — the `Loop` case of "does this diverge" **is**
  `block_contains_break_for`. A separate file would import Tier 1 for its only
  non-trivial branch on line one, which is a split in name only.

So `control_flow.rs` grows a second tier:

**Tier 1 — pure syntactic predicates.** `&HirBody` in, plain data out.
`block_contains_break_for` (G8/G9/G10).

**Tier 2 — typed divergence.** `expr_diverges` / `stmt_diverges` /
`block_diverges` / `block_parts_diverge`, each taking `&BodyContext<'_>`.

Tier 2 needs the context because the *leaf* case of divergence is "did inference
give this type `!`?" — a `-> !` call is an ordinary `HirExpr::Call` with no
syntactic tell. `AGENTS.md` §5 records this as **one named exception**, not a
precedent: anything else in this file that wants `BodyContext` is analyzer logic
and stays in its analyzer.

## 2. Ordering is load-bearing: structure first, `Never` as the leaf fallback only

`expr_diverges` matches `Return`/`Break`/`Continue` → `If` → `Match` → `Loop` →
`Block` → `Sugar`, and only expressions with no control-flow structure of their
own reach the `_` arm that tests for `ResolvedTy::Never`.

`guard.rs` used to test the type **first**. That lets inference override the
structural `Loop` verdict — precisely the "any breakable loop counts as
diverging" hazard G8 removed. It happened to be harmless there only because the
`Loop` arm below it was already `!contains_break_for`. Encoding the ordering
once, with the reason in the code, is what stops the next copy from getting it
backwards.

## 3. The `Loop` rule is `!contains_break_for` **alone**

Not `body_state.diverged && !contains_break_for`. The conjunct is not
conservative, it is wrong in the unsafe direction: for `loop { doWork(); }` the
body completes, so `diverged` is false and the formula denies that an infinite
loop diverges. See `problem.md` for the resulting false `E500`.

## 4. `stmt_diverges` handles `HirStmt::Let { value: Some(v) }`

`let x = fatalError();` never binds `x`, so the rest of the block is
unreachable. Only `exhaustive_return`'s copy got this right. This is an addition
beyond the diagnosis, taken because it is free once the rule is unified and its
blast radius is zero.

## 5. Per-analyzer call sites — what is delegated and what is not

- **`guard.rs`** — whole divergence section deleted; calls Tier 2 directly.
  Behavior-preserving apart from the new `Let` arm.
- **`exhaustive_return.rs`** — threads `cx` in place of `(hir, typed)` pairs and
  delegates **only the leaf fallback**. Its three-way `ReturnState` and its
  `Loop`/`If`/`Match` arms stay: they compute "returns a *value*" vs "leaves
  abnormally", which a `bool` cannot express, and its `Loop` arm already had the
  canonical rule.
- **`dead_code.rs`** — largest diff. `cx` replaces `(hir, in_loop)`;
  `stmt_diverges`/`expr_diverges`/`block_part_diverges` deleted, and with them
  `block_always_returns`/`expr_always_returns` — both branches after the
  break-check returned `true` unconditionally, so those two helpers were
  provably dead already, independent of this change.
- **`definite_assignment.rs` / `move_tracking.rs`** — formula corrected, plus an
  early `return state;` so the structural verdict is final. In `move_tracking`
  that makes the `&& !matches!(.., HirExpr::Loop { .. })` carve-out on the
  trailing Never check structurally unreachable, so it is gone.
- **`initializer.rs`** — mechanism was already correct; `break_states.is_empty()`
  *is* `!contains_break_for`, only reachability-aware and therefore strictly
  stronger. Comment corrected, code untouched. It keeps its own walk for the
  reason `AGENTS.md` §5 already gives.

## 6. `dead_code`'s `in_loop` parameter drops out entirely

`Break`/`Continue` are unconditionally `Never`-typed, valid position or not, so
the Tier-2 leaf answers correctly with no loop context. That also retires — for
free, in the same three lines — the "labeled break/continue are conservatively
non-diverging" carve-out, which suppressed a legitimate `E002` after
`break outer;`.

Three testdata files gained a `// WARN: unreachable` as a result. All three are
genuine unreachability, each following a `break`/`continue` that is `!`-typed
regardless of whether its label or its position is valid; the pre-existing
"outside of loop" / "undeclared label" errors on those lines are separate,
earlier hir-lower diagnostics and are unaffected.

**Decision rule for any future newly-appearing warning:** annotate it only if
the code genuinely cannot execute. If it does not correspond to genuine
unreachability, the Tier-2 helper is wrong — fix the helper.

## 7. Blast radius: the 30 `fatalError` testdata files are *not* affected

The diagnosis expected 30 files to need re-annotation. All 30 are
`test: execution`, which check exit code and stdout only — analyzer warnings are
invisible to them. Verified by running the suite: real blast radius from
`fatalError` is zero files.
