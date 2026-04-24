---
name: type-inference
description: Internals of the lib type-infer crate — the constraint solver, unification, member resolution, literal defaults, error recovery, and the pitfalls they hide. Use when debugging "could not infer type", "type mismatch", "ambiguous member", where-clause bugs, protocol-conformance wrong answers, associated-type resolution, literal-default surprises, or any solver-crate change. Also use when adding a new `Constraint` variant, `InferError` variant, or extending `TypeResolver`. For "where does this source construct live?" routing, use `kestrel-pipeline` instead.
---

# type-inference

Operational reference for `lib/kestrel-type-infer/`. Explains the three-phase
architecture, the **12** constraint variants (design doc says 8 — it's stale),
the solver's fixpoint loop, and every pitfall worth remembering.

## When to open which file

| Question | File |
|---|---|
| How is the solver laid out? What file does what? What's the generate→solve→resolve dance? | `solver.md` — architecture, fn anchors, dispatch map |
| "Could not infer type" cascade, poisoning rules, wrong error count, Error vs Never, wildcard (`_`) infer vars | `pitfalls.md` — symptom-indexed |
| How does `Equal`/`Coerce`/`Member`/`Associated`/`Conforms`/`Call`/`OverloadedCall`/`Implicit`/`ImplicitPat`/`Reduce`/`TupleRestPat`/`TupleIndex` work? | `solver.md` → "Constraint dispatch" |
| Where do I add a new `InferError` variant? | `lib/kestrel-type-infer/AGENTS.md` (5-file checklist lives there) |

When unsure: start with `solver.md` for "how it works", `pitfalls.md` for "why it broke."

## Three-phase summary (memorize this)

```
HirBody ─► generate.rs ─► constraints ─► solver.rs ─► substitutions ─► resolve.rs ─► TypedBody
           (walk once)    (fixpoint)                  (lower to concrete)
```

**Generate** (`generate.rs`) — walks the HIR once and emits one or more constraints
per expr/pat/stmt. Allocates fresh `TyVar`s, records `expr_types` / `local_types`.

**Solve** (`solver.rs`) — drains constraints, dispatches via `try_solve`
(solver.rs:589), each arm returns `SolveResult::{Solved, Deferred, Error}`. Deferred
constraints are re-queued for the next round. Fixpoint: `fixpoint()` at solver.rs:330
→ loops `solve_round()` (solver.rs:343) until a round makes no progress. Then
`apply_literal_defaults` (solver.rs:2862), another fixpoint, then
`default_never_fallback` (solver.rs:117), then `report_unsolved` (solver.rs:370) for
anything still unresolved.

**Resolve** (`resolve.rs`) — after solving, lowers every `TyVar` to `ResolvedTy`
and builds `TypedBody` (expr types + member entity resolutions + promotions +
inferred type args). Runs member/associated/conformance queries during solving
too; the "resolve" split is conceptual, not file-strict.

## Key types (skim before reading solver code)

| Type | File | Role |
|---|---|---|
| `TyVar(u32)` | `ctx.rs` | Index into `InferCtx::types`. Cheap copy. |
| `TySlot` | `ctx.rs` | `Unresolved { literal } \| Redirect(TyVar) \| Resolved(TyKind)`. Union-find via `Redirect`. |
| `TyKind` | `ctx.rs` | `Named { entity, args } \| Param \| Tuple \| Function \| Never \| Error` |
| `LiteralKind` | `ctx.rs` | Integer/Float/String/Bool/Char/Null/Array/Dictionary — marker on unresolved literal TyVars |
| `Constraint` | `constraint.rs` | 12 variants (see below) |
| `InferCtx` | `ctx.rs` | Solver state: types vec, constraints queue, errors, resolutions, promotions, type_args, `where_clause_assoc_subs`, `wildcard_tvars`, `errored_coerce_exprs` |
| `InferError` | `error.rs` | Diagnostic enum. Every `TyKind::Error` is preceded by a reported `InferError` (ErrorGuaranteed pattern) |
| `TypedBody` | `result.rs` | Output: expr types, resolutions, promotions, inferred type args, errors |

## Constraint variants (12, not 8)

All in `constraint.rs`:

| Variant | Meaning | Solver fn (solver.rs) |
|---|---|---|
| `Equal { a, b }` | τ₁ = τ₂ structural equality | `solve_equal` :1000 |
| `Coerce { from, to, expr }` | Value flow, tries equal then promotion (`FromValue`) | `solve_coerce` :1155 |
| `Conforms { ty, protocol }` | τ : Protocol — deferred until ty concrete | `solve_conforms` :1329 |
| `Associated { container, name, result }` | `Container.Name → τ` | `solve_associated` :1352 |
| `Member { receiver, name, args, result, expr }` | field/method/init/subscript resolution | `solve_member` :1955 |
| `Call { callee, args, result, expr }` | fn or subscript call | `solve_call` :1504 |
| `OverloadedCall { candidates, type_args, args, result, expr }` | one-of-many dispatch | `solve_overloaded_call` :1632 |
| `Implicit { expected, name, args, result, expr }` | `.CaseName` against expected type | `solve_implicit` :2432 |
| `ImplicitPat { scrutinee, name, args }` | pattern form of Implicit (`.Some(x)`) | `solve_implicit_pat` :2543 |
| `Reduce { alias, result }` | expand a type alias / associated to its reduced form | `solve_reduce` :899 |
| `TupleRestPat { scrutinee, prefix_tys, suffix_tys }` | `[a, ...rest, b]` / `(a, ...rest, b)` bindings | `solve_tuple_rest_pat` :2651 |
| `TupleIndex { tuple, index, result }` | `pair.0` projection | `solve_tuple_index` :730 |

`try_solve` dispatch: `solver.rs:589`.

## Verify before recommending

**Line numbers drift.** Every function anchor in this skill was current at the
time of writing. Before telling the user "edit solver.rs:1955," re-grep:

```
grep -n "^fn solve_member\|^fn try_solve" lib/kestrel-type-infer/src/solver.rs
```

Enum variants (`Constraint`, `TyKind`, `InferError`) are stable; dispatch fns
move. Update this skill's anchors when you notice drift.

## Extending the skill

1. New `Constraint` variant: add to `constraint.rs`, add emission in
   `generate.rs`, add `solve_*` fn + `try_solve` arm, update the table above.
2. New `InferError` variant: 5-file checklist in
   `lib/kestrel-type-infer/AGENTS.md`. Do not skip.
3. New pitfall: add a symptom-indexed entry to `pitfalls.md` with a link to the
   memory file (if one exists) and the load-bearing fact.

## Related memory (cross-references)

Solver-specific memory entries (read inline when relevant; prefer the skill as
the index):

- `solver_poison_overreach.md` — `solve_equal`/`solve_coerce` must return
  `SolveResult::Error`, not `report_and_poison`. Test suite relies on secondary
  diagnostics.
- `cascading_infer_errors.md` — the whole "could not infer type" cascade story:
  5 interacting root causes, `wildcard_tvars`, pattern poisoning.
- `solve_associated_self_ref.md` — self-referential `where_clause_assoc_subs`
  loop; check before emitting `Equal`.
- `where_clauses_of_query.md` — use the `WhereClausesOf` query, never build a
  `WorldResolver` for where clauses.
- `witness_instantiation_collapse.md` — don't dedupe protocol witnesses by
  protocol entity; include type args.
- `static_overload_first_match_truncation.md` — static overload resolution
  returned the first child by name; must consider arity.
- `self_item_leaked_to_mir.md` — `HirTy` has no `SelfType`; bare `Item` in
  `extend Iterator` leaks as `Named(Protocol)` without substitution.

Pipeline-routing memory lives in the `kestrel-pipeline` skill — use that for
"where does `AstExpr::Foo` turn into a constraint?"
