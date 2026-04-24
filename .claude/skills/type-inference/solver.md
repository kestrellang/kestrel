# Solver architecture (lib2 `kestrel-type-infer`)

Flat union-find over `TyVar`s with a fixpoint constraint loop. Deliberately
lean — ~10k lines total across 12 files, most of the bulk in `solver.rs`
(3.2k), `resolve.rs` (1.9k), `generate.rs` (1.8k).

## Pipeline position

```
CST → AST → name-res → HIR lower → type-infer (HERE) → MIR lower → codegen
```

`InferBody { entity, root }` is the query entry point (`lib.rs`). It reads
`LowerBody`, builds an `InferCtx`, runs `generate()` then `solve()`, and
returns a `TypedBody`.

## File map

| File | Lines | Role |
|---|---|---|
| `lib.rs` | 602 | `InferBody` query, orchestration, per-body entry |
| `ty.rs` | 135 | `TyVar`, `TySlot`, `TyKind`, `LiteralKind` |
| `constraint.rs` | 169 | `Constraint` enum (12 variants), `CallArg` |
| `ctx.rs` | 593 | `InferCtx`: types vec, constraint queue, resolutions, promotions, type_args, `where_clause_assoc_subs`, `wildcard_tvars`, `errored_coerce_exprs` |
| `generate.rs` | 1832 | HIR walk → constraint emission; `gen_expr` / `gen_stmt` / `gen_pat` |
| `solver.rs` | 3205 | fixpoint loop, `try_solve` dispatch, per-constraint solver fns, literal defaults |
| `unify.rs` | 594 | Structural unification, literal guards, occurs check, Error/Never handling |
| `resolve.rs` | 1949 | `TypeResolver` trait + `WorldResolver` impl; member/associated/conformance resolution |
| `where_clauses.rs` | 210 | `WhereClausesOf` memoized query |
| `compare.rs` | 237 | Type comparison helpers used by overload/extension dispatch |
| `result.rs` | 456 | `TypedBody` output, `ResolvedTy`, lowering from `TyVar` → `ResolvedTy` |
| `error.rs` | 226 | `InferError` variants; add-a-variant checklist in `AGENTS.md` |

## Solver entry points (anchors)

Current as of 2026-04-24 — re-grep if the file shifted.

| Fn | solver.rs line | Note |
|---|---|---|
| `solve` (public entry) | 24 | Fixpoint → literal defaults → fixpoint → never fallback → report_unsolved |
| `fixpoint` | 330 | Loops `solve_round` until no progress |
| `solve_round` | 343 | Drains `ctx.constraints`, dispatches each via `try_solve`, re-queues `Deferred` |
| `try_solve` (dispatch) | 589 | Match on `Constraint` variant |
| `solve_reduce` | 899 | Type-alias/assoc reduction |
| `solve_equal` | 1000 | Bare structural equality |
| `solve_coerce` | 1155 | Equal → fallback to `FromValue` promotion |
| `solve_conforms` | 1329 | Defer if ty unresolved; else ask resolver |
| `solve_associated` | 1352 | `Container.Name`, checks `where_clause_assoc_subs` first |
| `solve_call` | 1504 | Callee-type-dispatched (Function vs Named) |
| `solve_overloaded_call` | 1632 | Score candidates, pick, emit as resolved call |
| `emit_resolved_call` | 1710 | Shared emitter for `Call` / `OverloadedCall` resolved result |
| `solve_member` | 1955 | The big one: resolve + instantiate + emit where clauses + equate args/return |
| `solve_implicit` | 2432 | `.Name(args)` against expected type |
| `solve_implicit_pat` | 2543 | Same but for patterns (poisons pat args on mismatch) |
| `solve_tuple_rest_pat` | 2651 | `[a, ...rest, b]` binding types |
| `solve_tuple_index` | 730 | `tup.0` projection |
| `apply_literal_defaults` | 2862 | Between fixpoint phases — converts remaining `Unresolved { literal: Some(_) }` to the builtin default type |
| `default_never_fallback` | 117 | Last resort: unused generic TyVars → Never |
| `report_unsolved` | 370 | Phase-4 diagnostic reporting for anything still Unresolved |
| `poison_if_unresolved` | 206 | Helper: bind an unresolved TyVar to `TyKind::Error` (skips literals — they carry info) |
| `poison_unresolved_type_args` | 220 | Recursive: poison inner Unresolved args of a `Named` |
| `report_and_poison` | 1072 | Combined report-error-then-poison. **Only** used in phase 4 (see `pitfalls.md`) |
| `kind_to_tyvar` / `kind_to_tyvar_sub` | 2954, 2960 | `TyKind` → `TyVar`, with optional substitution for where-clause lowering |
| `lower_hir_ty_sub` | 3077 | HIR type → TyVar, honors `where_clause_assoc_subs` |
| `lower_hir_ty_plain` | 3203 | HIR type → TyVar, NO sub map — used where subs would create self-loops |

## Generation entry points

In `generate.rs`:

- `gen_expr` dispatch: line 60 (`HirExpr` match). Returns the `TyVar` for the
  expression; every branch must `ctx.expr_types.insert(id, tv)` at the end.
- `gen_stmt` dispatch: line 551.
- `gen_pat` dispatch: line 607 (takes scrutinee `TyVar`, constrains it against
  the pattern shape and binds locals).

Rule: every `gen_expr` branch allocates exactly one result `TyVar`, emits
zero-or-more constraints, and returns the `TyVar`. Locals go into
`ctx.local_types` at their binding site; references look them up.

## Unification (`unify.rs`)

`unify(ctx, a, b)` is the core primitive. Steps:

1. `resolve(a)` / `resolve(b)` — follow `Redirect` chain to roots.
2. If `a == b` → `Ok(())`.
3. **Error absorbs**: either side `TyKind::Error` → `Ok(())` silently. This is
   intentional (prevents cascades) but requires the caller to check `is_error`
   BEFORE unifying when it wants to propagate poison. See `pitfalls.md`.
4. **Never unifies with anything**, but does NOT bind an Unresolved side — let
   other constraints win.
5. **Both unresolved** → link via `Redirect`; keep literal marker if either had
   one.
6. **Unresolved non-literal + concrete** → occurs-check, then `Redirect` to
   concrete.
7. **Unresolved literal + concrete** → "literal guard": concrete must conform
   to `ExpressibleBy*<Lit>` protocol (`conforms_to_literal_protocol`). If yes,
   bind; if no, `Err(UnifyError::LiteralGuard)` — `solve_coerce` then falls
   back to promotion.
8. **Both concrete** → `unify_concrete`: structural (Named entity+args; Tuple
   arity+elems; Function arity+params+ret; Param entity).

`UnifyError` variants: `Mismatch`, `LiteralGuard`, `OccursCheck`.

## Literal defaults

After phase-1 fixpoint, any `Unresolved { literal: Some(kind) }` that didn't
get bound by constraint-driven unification gets its builtin default type via
`apply_literal_defaults` (solver.rs:2862). Builtins queried:

- `Integer` → `@builtin(DefaultIntegerLiteralType)` (typically `Int64`)
- `Float` → `DefaultFloatLiteralType` (typically `Float64`)
- `String` → `DefaultStringLiteralType` (`String`)
- `Bool` → `DefaultBooleanLiteralType` (`Bool`)
- `Char` → `DefaultCharLiteralType` (`Char`)
- `Null` → `DefaultNullLiteralType` (`Optional[_]`)
- `Array` → `DefaultArrayLiteralType` — generic; element TyVars linked by the
  originating constraint
- `Dictionary` → `DefaultDictionaryLiteralType` — same

Then a second fixpoint runs to let the new bindings propagate (array-literal
element types, iterator adapters, etc.).

## Type resolver (`resolve.rs`)

`WorldResolver` is the sole impl — exposed to the solver as a trait for
testability, but nothing else implements it in-tree. Reads:

- `VisibleChildrenByName` (name-res query)
- `ExtensionsFor` (name-res query)
- `WhereClausesOf` (local query, `where_clauses.rs`)
- Conformance tables via world components

Key `resolve.rs` methods:

- `resolve_member` — searches receiver's direct children + applicable
  extensions, filters by args/labels, picks best overload.
- `conforms_to` — walks conformance declarations and extension conformances.
  Handles `Param` / `Named(TypeParameter)` by reading where-clause bounds.
- `resolve_associated_type` — handles concrete, `Param`, and `TypeAlias`
  containers. For abstract containers, searches protocol bounds for the assoc
  entity.
- `builtin(BuiltinFeature)` — looks up language-level entities (literal
  protocols, default types, operators).

## Error recovery (ErrorGuaranteed)

Every `TyKind::Error` must be preceded by a reported `InferError`:

1. Push an `InferError` via `ctx.report_error(err)` (which returns an Error
   `TyVar`) or `ctx.errors.push(err) + poison_if_unresolved`.
2. Downstream unifications absorb the Error silently — no cascade.

Failure modes if you skip step 1: you'll see `TyKind::Error` in the output
with NO diagnostic. Always pair them.

**Do not** use `qctx.accumulate(Diagnostic::...)` from inside the solver. That
path is for HIR-lower / decl-level analyzers. Solver errors flow through
`InferError` → `diagnostic.rs` match arm → user-facing diagnostic. See
`lib2/kestrel-type-infer/AGENTS.md`.

## Adding an `InferError` variant (do not skip any file)

5 files must be touched — non-exhaustive-match errors only surface on a
dependent crate rebuild, which is slow to discover.

1. `lib2/kestrel-type-infer/src/error.rs` — variant + `span()` arm.
2. `lib2/kestrel-type-infer/src/result.rs` — `describe_error()` arm.
3. `lib2/kestrel-compiler/src/diagnostic.rs` — user-facing `Diagnostic` arm.
4. `lib2/kestrel-analyze/src/body/type_check.rs` — `format_error()` arm
   (message + label text).
5. `lib2/kestrel-compiler-driver/src/lib.rs` — both `describe()` and
   `format_error()`.

## Adding a `Constraint` variant

1. `constraint.rs` — add variant.
2. `generate.rs` — emit it from the appropriate `gen_*` branch (or via a
   `ctx.foo(...)` helper method on `InferCtx` — add that in `ctx.rs`).
3. `solver.rs` — new `solve_foo` fn, arm in `try_solve`.
4. Update the constraint table in `SKILL.md`.
5. Tests: `lib2/kestrel-test-suite/testdata/type_infer/…` — add a
   `diagnostics` or `execution` case covering the new constraint behavior.
   Run via the `triage` skill, not `cargo test`.

## Query layering

`kestrel-type-infer` depends on:

- `kestrel-hir` for `HirBody` / `HirExpr` / `HirTy` / `HirPat`.
- `kestrel-name-res` for resolution queries (visibility, extensions, scope).
- `kestrel-hecs` for the ECS/query infra.
- `kestrel-ast-builder` for component inspection.
- `kestrel-debug` for `ktrace!` (enabled via `VERBOSE_DEBUG_OUTPUT=1`).

Downstream crates that consume `TypedBody`:

- `kestrel-analyze` — body-level diagnostics (type checking cascade
  suppression lives here).
- `kestrel-mir-lower` — reads `expr_types`, `resolutions`, `promotions`,
  `type_args` to emit MIR.
