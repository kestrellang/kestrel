# Solver pitfalls — symptom-indexed

Each entry: observable symptom → root cause → what to check/change → memory
cross-ref. If you land here from a real failure, the rule is: **verify the
cause before applying the fix**. Solver pitfalls look nearly identical across
different bugs.

---

## "Could not infer type" reported, but real error is elsewhere

**Symptom**: user's test expects a `TypeMismatch` or `NoMember`, gets
`UnresolvedType` on an unrelated TyVar instead — or, after a single mismatch,
a swarm of "could not infer type" on every variable touching the bad one.

**Causes (there are five; they interact):**

1. `unify::unify` silently absorbs `TyKind::Error` (unify.rs:42). When one side
   is Error, `unify` returns `Ok(())` WITHOUT binding the other side — the
   Unresolved TyVar stays bound to nothing.
2. `solve_equal` / `solve_coerce` delegate straight to `unify`. They must
   check `is_error(a) || is_error(b)` FIRST and poison the unresolved side.
3. `solve_member` / `solve_call` / `solve_implicit` / `solve_overloaded_call`
   fail with `SolveResult::Error` but their `result` TyVar stays Unresolved.
   `try_solve` now wraps these and calls `poison_if_unresolved(result)` on
   `SolveResult::Error`.
4. `ImplicitPat` / `TupleRestPat` pattern bindings (e.g. `x` in `.Some(x)`)
   stay Unresolved when the scrutinee is Error/non-enum/no-matching-case.
   `solve_implicit_pat` and `report_unsolved` must poison `arg_tys` /
   `prefix_tys` / `suffix_tys` explicitly.
5. `HirTy::Infer` in explicit type args (e.g. `cast_ptr[_, T]`): the fresh
   TyVar from `_` has no context to pin it. Tracked as wildcards
   (`ctx.wildcard_tvars`); `contains_unresolved_type_args` SKIPS wildcards
   when deciding whether a `Named` is "still infer."

**Poisoning rule**: `poison_if_unresolved` MUST skip TyVars with literal
markers (`Unresolved { literal: Some(_) }`). Those carry real info —
`true` is `Unresolved { literal: Some(Bool) }`, and poisoning it hides
legitimate downstream mismatches (e.g. `if c { true } else { 42 }` returning
`i64`).

**Cross-ref**: `memory/cascading_infer_errors.md` (2026-04-23, -18 failures).

---

## After adding diagnostic → test suite regresses with "only one error instead of two"

**Symptom**: a test that expects two diagnostics (e.g. `if` branch mismatch +
return-type mismatch) now only reports one.

**Cause**: `solve_equal` or `solve_coerce` is poisoning via
`report_and_poison(...)` instead of returning `SolveResult::Error(...)`.

**Rule**: In `solve_equal` / `solve_coerce`, **always** return
`SolveResult::Error(...)` on Mismatch / LiteralGuard / final-coerce-failure
paths. Never call `report_and_poison` there. Poisoning converts both sides to
`TyKind::Error`, and `unify`'s Error absorption then swallows the secondary
diagnostic (e.g. the coerce-to-return-type that would point at the `if`
keyword after branches disagree).

`report_and_poison` is reserved for `report_unsolved` (phase 4 — after the
fixpoint, when cascades are the expected behavior). The per-expression
cascade-suppression mechanism for coerce is `ctx.errored_coerce_exprs.insert(expr)`
— that's per-expr, not per-TyVar, so it doesn't silence legitimate downstream
errors.

**Test canaries**: `if_else_branches_mismatch_{bool_int,tuple_int}`,
`if_else_if_else_branches_mismatch`, `literal_without_conformance`,
`wrong_literal_type`, `default_type_mismatch`,
`wrong_type_for_associated_value`, `wrong_type_multiple_params`.

**Cross-ref**: `memory/solver_poison_overreach.md`.

---

## Associated type stays Unresolved forever (no error, no resolution)

**Symptom**: a constraint like `T.Iter → τ` never solves. No error is
reported. The TyVar for `τ` goes into `report_unsolved` at phase 4.

**Cause**: self-referential `where_clause_assoc_subs` lookup. When
`emit_protocol_assoc_type_where_clauses` stores a fresh TyVar AS BOTH the
result of an `Associated` constraint and the substitution entry for the same
assoc entity, then `solve_associated` looks up the subs, finds the same TyVar
back, and `solve_equal(tv, tv)` → `unify(tv, tv)` is a trivial no-op.

**Fix**: In `solve_associated`, compare the subs lookup result to
`ctx.resolve(result)`. If identical, skip the sub and directly build a
concrete `Named` type via `ctx.named(entity, vec![])`. Applies to both the
entity-keyed lookup and the name-based fallback. `lower_hir_ty_plain` exists
for exactly this case — it does NOT consult `where_clause_assoc_subs`.

**Cross-ref**: `memory/solve_associated_self_ref.md`.

---

## Where-clause bounds silently dropped; `? !: NotEqual` far from the real site

**Symptom**: a method with `where Self.Item: Equatable` (or similar) compiles
fine in isolation but when called from another body, constraints referencing
`Self.Item` look unbound. The diagnostic points at the callsite, not the
clause.

**Cause**: where clauses on entities are resolved by the `WhereClausesOf`
memoized query. If you bypass it and build a `WorldResolver { owner: ... }`
to call a where-clause method, the owner is the BODY being inferred — LHS
subjects get resolved in the wrong scope.

**Rule**: always use:

```rust
let clauses = ctx.query(crate::where_clauses::WhereClausesOf { entity, root });
```

Names in the clause are resolved in `entity`'s own scope via
`ResolveTypePath { context: entity, ... }`. There is deliberately **no**
`context` parameter — the entity IS the context. `WorldResolver::body_owner`
is named with `body_` prefix to make the constraint obvious at call sites.

**Cross-ref**: `memory/where_clauses_of_query.md`.

---

## Wrong protocol witness; type args collapse to 8 bits / first instance

**Symptom**: `Convertible[Int32]` and `Convertible[Int64]` both resolve to the
same witness; values get truncated in codegen.

**Cause**: witness instantiation was keyed by protocol entity alone, ignoring
type args. All instances of a generic protocol shared one witness.

**Fix**: include type args in the witness instantiation key. Don't dedupe
by protocol entity. **Cross-ref**: `memory/witness_instantiation_collapse.md`.

---

## Static overload picks the wrong arity

**Symptom**: `Type.method()` with multiple overloads resolves to the first
static child by name regardless of arity.

**Cause**: `try_resolve_static_call*` variants returned the first child
matching `name`.

**Fix**: score all name-matching children by label set + arity, pick best
match (same algorithm as instance-method overload resolution).
**Cross-ref**: `memory/static_overload_first_match_truncation.md`.

---

## Bare `Item` / `Self` type leaks to codegen as `Named(Protocol)`

**Symptom**: runtime SIGSEGV reading a `struct { ... Item ... }` field when
Item should be `Int8`.

**Cause**: HIR has no `SelfType` variant. In `extend Iterator` (or similar
protocol-extension scope), an unqualified `Item` lowers to
`HirTy::Named { entity: TypeAlias(Item) }` with no substitution map. By
monomorphization time it's still abstract.

**Fix**: when generating constraints inside a protocol extension, emit
`Associated { container: Self_tv, name: "Item", result }` and bind `result`
to the concrete Item (derived from the impl's `where Item = X` or the
conforming struct's associated type).
**Cross-ref**: `memory/self_item_leaked_to_mir.md`.

---

## `_` wildcard in explicit type args reports "could not infer"

**Symptom**: `foo[_, T]` where `foo` has two type params — even when the first
doesn't actually need to be concrete for the call to work.

**Cause**: `HirTy::Infer` produces a fresh Unresolved TyVar with no
context to pin it; unless another constraint happens to bind it,
`report_unsolved` fires.

**Fix**: mark the TyVar via `ctx.mark_wildcard(tv)` in `generate.rs` when
lowering `HirTy::Infer`. `contains_unresolved_type_args` skips wildcard
roots; `poison_if_unresolved` / `report_unsolved` leave them alone.
Wildcard status propagates through unification (see unify.rs changes in
`cascading_infer_errors.md`).

---

## Literal `null` "stays Optional[?]" after defaults

**Symptom**: `let x = null;` with no further constraint ends up as
`Optional[?unresolved]`, not a clean type.

**Cause**: `apply_literal_defaults` instantiates
`@builtin(DefaultNullLiteralType) = Optional[T]` with a fresh TyVar for T;
without downstream constraints, T stays Unresolved. Then
`default_never_fallback` (solver.rs:117) binds it to `Never`, giving
`Optional[Never]` — which is correct but surprising.

**Rule**: don't report this as a bug. `Optional[Never]` at the end of
inference means the `null` was never used concretely. If the user expected
an annotation-driven type, check that the `let` binding has one — if it does
and this still happens, it's a constraint-emission bug in `gen_stmt` /
`gen_expr::Literal`, not a defaults bug.

---

## `Equal` constraint makes no progress; fixpoint never terminates

**Not possible under the current design**, but if a custom `Constraint`
variant you added always returns `SolveResult::Deferred(same_constraint)`
with no side effects, `solve_round` returns `false` → `fixpoint` exits
cleanly → `report_unsolved` fires. So the visible bug is "no termination
issue, but huge pile of Unresolved errors at phase 4."

**Rule**: every `solve_*` that returns `Deferred` must either make progress
some round (via a side constraint it emits) or be followed by a check in
`report_unsolved` that converts the deferred state into a clean error.

---

## Per-expression cascade: "3 errors for 3 args of the same call"

**Symptom**: user gets N diagnostics for one miscall.

**Fix**: `solve_coerce` inserts the expr into `ctx.errored_coerce_exprs`
on first failure; subsequent coerce failures against the same expr are
silently dropped. Verify this set is consulted (don't disable it while
debugging something else — it's the primary dedup path for arg-coerce
cascades).

---

## Adding a new pitfall

When you diagnose a new recurring trap:

1. Add an entry here with the same shape: symptom → cause → fix → memory link.
2. If the pattern is novel enough to warrant its own memory file, write it to
   `/Users/dino/.claude/projects/-Users-dino-Documents-Projects-kestrel/memory/`
   and link it from `MEMORY.md` (one-line index entry, under 150 chars).
3. Keep each entry short: the goal is a fast lookup from symptom, not a
   tutorial on the subsystem.
