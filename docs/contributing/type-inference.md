# Type Inference (lib)

Kestrel uses a **bidirectional, constraint-based** type system with a fixpoint solver. The implementation lives in `lib/kestrel-type-infer/`. This document is the contributor-level overview — enough to find your way around and safely add a constraint or diagnostic. For deeper invariants see `lib/kestrel-type-infer/AGENTS.md` and `lib/kestrel-type-infer/docs/`.

## Where it runs

```
Source ─▶ … ─▶ HIR Lower (HirBody) ─▶ Type Infer (TypedBody) ─▶ Analyze / MIR Lower / Codegen
                                       ^^^ this crate
```

Inference is a memoized query: `InferBody { entity, root }`, returning `Option<Arc<TypedBody>>` (Arc-shared so cache hits don't deep-copy). The `TypedBody` has:

- Resolved types for every `HirExprId` / `HirPatId`.
- Resolved member resolutions (which entity a method / field / subscript call landed on).
- Promotion records (where a value needs implicit `FromValue` wrapping, e.g. `Int` into `Optional[Int]`).
- Collected `InferError`s.

Downstream code (analyzers, MIR lowering) reads these fields; it does not re-run inference.

## Files at a glance

| File | Purpose |
|------|---------|
| `src/ty.rs` | `TyVar`, `TyKind`, `TySlot` — the inference-side type representation. |
| `src/ctx.rs` | `InferCtx` — solver state. |
| `src/constraint.rs` | `Constraint` enum. |
| `src/error.rs` | `InferError` enum. |
| `src/generate.rs` | HIR walker that emits constraints. |
| `src/solver.rs` | The fixpoint loop and per-constraint solve routines. |
| `src/unify.rs` | Unification. |
| `src/resolve.rs` | `TypeResolver` / `WorldResolver` — name resolution for types during inference. |
| `src/where_clauses.rs` | `WhereClausesOf` query — resolves entity-scoped where clauses. |
| `src/result.rs` | `TypedBody`, `ResolvedTy`, error descriptions. |
| `src/compare.rs` | Structural comparison, assignability checks. |
| `AGENTS.md` | Adding `InferError` variants; solver reporting rules; memberwise-init invariants. |

## Types on the inference side

During inference we work with `TyVar` — an integer handle into `InferCtx` — and `TyKind`, a normal enum of type constructors:

```
TyKind::Infer                      placeholder, to be resolved
TyKind::Error                      poison absorber; unifies with anything
TyKind::Never                      !
TyKind::Unit / Int / Float / Bool / String / …   primitives
TyKind::Tuple(Vec<TyVar>)
TyKind::Pointer(TyVar)
TyKind::Function { params, ret }
TyKind::Named { entity, type_args }          struct / enum / protocol / alias
TyKind::TypeParameter(Entity)
TyKind::AssociatedType { container, name }
TyKind::SelfType
```

`TySlot` tracks substitution state for a `TyVar`: fresh (`Infer`), bound to a `TyKind`, or unified with another `TyVar`.

Every type defaults to `Infer` until a constraint pins it down.

## Constraints

The solver operates on these variants (`constraint.rs`):

| Variant | Meaning | Deferred? |
|---------|---------|-----------|
| `Equal { a, b }` | Structural equality. Branches of `if`/`match`, array elements. | No — unifies eagerly. |
| `Coerce { from, to, expr }` | Value flows from `from` to `to`. Tries `Equal`, then `FromValue` promotion. Used at let bindings, arguments, returns, assignments. | No. |
| `Conforms { ty, protocol }` | `ty : Protocol`. | Yes — needs concrete `ty`. |
| `Associated { container, name, result }` | `Container.Name` projection. | Yes — needs concrete container. |
| `Member { receiver, name, args, result, … }` | Method / field / computed property / subscript / init resolution. | Yes — needs concrete receiver. |
| `Call { callee, args, result }` | Function or subscript call. | Yes — needs concrete callee. |
| `OverloadedCall { candidates, type_args, args, result }` | Overload resolution by labels + arity + type compatibility. | Yes. |
| `Implicit { expected, name, args, result }` | `.Case(args)` resolved against an expected type (enum shorthand). | Yes — needs concrete expected. |
| `ImplicitPat { … }` | Same, but in pattern position. | Yes. |
| `Reduce { … }` | Numeric / literal narrowing on known-concrete sides. | Yes. |
| `TupleRestPat { … }` | `(a, .., c)` pattern binding. | Yes. |
| `TupleIndex { tuple, index, result }` | `t.0`, `t.1`, … | Yes. |

"Deferred" means the solver leaves the constraint in the pool and retries it each round until its inputs are concrete.

Constraints are generated in `generate.rs` while walking the HIR body. For each HIR expression, the generator emits the constraints that the expression implies and associates them with the source span.

## The solver

```rust
// src/solver.rs
pub fn solve(ctx: &mut InferCtx<'_>, hir: &HirBody) {
    fixpoint(ctx);                       // Phase 1: main solving
    apply_literal_defaults(ctx);         // Phase 2: literals default to Int64 / Float64 / String / …
    fixpoint(ctx);                       // Phase 3: solve again with defaults applied
    report_unresolved_type_params(ctx);  // Phase 4: diagnose unbound generics at call sites
    // Phase 4.25: never-fallback for branches where Never leaked through
    report_unsolved(ctx);                // Phase 5: emit "could not infer" for remaining TyVars
}
```

### Fixpoint iteration

`fixpoint` repeatedly walks the constraint pool. For each constraint, the solver attempts one of three outcomes:

- **Solved** — constraint discharged, possibly recording substitutions or new sub-constraints.
- **Deferred** — not enough information yet; keep it for the next round.
- **Errored** — unresolvable. The solver calls `ctx.report_error(InferError::…)` and substitutes `TyKind::Error` for the affected `TyVar` so downstream constraints see the poison absorber and stop firing cascades.

The loop terminates when no constraint made progress in a full pass.

### Literal defaults

Integer literals without an explicit type default to `Int64`, floats to `Float64`, strings to `String`, etc. Defaulting runs **after** the first fixpoint so that contextual types get a chance to propagate first (`let x: Int32 = 5` resolves `5` to `Int32`, not `Int64`). `apply_literal_defaults` emits `Equal` constraints that pin each unresolved literal `TyVar` to its default; the second fixpoint then re-propagates.

### Unification (`unify.rs`)

`unify(a, b)` resolves two `TyVar`s:

1. Two `Infer`s — alias one to the other.
2. One `Infer` — occurs check, then substitute.
3. `Error` on either side — absorb silently.
4. `Never` — assignable to anything on the right-hand side.
5. Named-with-named — check entity identity, then unify `type_args` pairwise.
6. `Function` — unify params and return type.
7. `Tuple` — check arity, unify elements.
8. `TypeAlias` — expand, retry.
9. Primitives — exact equality.
10. Everything else — report `TypeMismatch`.

The occurs check prevents infinite types like `T = List[T]`.

## Name resolution during inference

Type paths inside bodies are resolved via the `TypeResolver` trait. The concrete implementation, `WorldResolver`, holds the `QueryContext`, the compilation root, and the body-owner entity — that last one is what anchors where-clauses and type parameters.

```rust
pub struct WorldResolver<'a> {
    ctx: &'a QueryContext<'a>,
    root: Entity,
    body_owner: Entity,
}
```

**Non-obvious invariant:** where-clauses belong to an entity's own scope. Use `ctx.query(WhereClausesOf { entity, root })` to get resolved clauses — never re-resolve names from a call site's scope. The query resolves associated-type equalities, protocol bounds, and subject paths within the declaring entity's scope, which is the only scope where they make sense.

## Type substitutions

Four distinct substitution mechanisms show up in inference:

### 1. Generic parameter substitutions

Nominal types (`Struct`, `Enum`, `Protocol`, `TypeAlias`) carry `type_args: Vec<TyVar>`. Applying substitutions walks composite types and replaces `TypeParameter(entity)` leaves with the mapped type. Cycle detection uses a visited set to break recursive substitutions.

### 2. `Self` substitution

Inside a method body, `TyKind::SelfType` means the method's containing type (with its own type parameters still abstract). `substitute_self(replacement)` recursively replaces `SelfType` throughout a type and rewrites naked associated types (`Item`) into qualified form (`Self.Item`).

### 3. Type alias expansion

`TyKind::Named { entity, type_args }` for a type-alias entity expands by looking up the alias's resolved target and applying `type_args` to it. Expansion is iterated — alias chains collapse.

### 4. Associated type projection

`Container.Name` resolves in one of three ways:

- **Via `Associated` constraint** during solving: the solver asks the semantics layer for the concrete associated type once `container` is concrete.
- **Via `WhereClausesOf`**: a `where T.Item = Int64` clause on the enclosing entity directly equates the projection, so subsequent uses of `T.Item` unify with `Int64` without querying protocols.
- **Via `substitute_self`** when entering a method body: naked `Item` inside a protocol becomes `Self.Item`, which then resolves as above.

## Literal flow (`let x: Int? = 5`)

1. HIR: `HirExpr::Literal(Int(5))`, assigned to a local with annotation `Int?`.
2. Generate: `Conforms(ty5, ExpressibleByIntLiteral)` and `Coerce(ty5, tyLocal)` where `tyLocal` is `Int?`.
3. Fixpoint round 1:
   - `Conforms` — deferred; `ty5` is still `Infer`.
   - `Coerce` — deferred; `from` is an unresolved literal.
4. Apply literal defaults: `Equal(ty5, Int64)`.
5. Fixpoint round 2:
   - `Equal` — substitutes `Int64` for `ty5`.
   - `Coerce(Int64, Int?)` — tries `Equal` (fails), falls back to `FromValue` check. Optional has a `FromValue` conformance that accepts `Int64`; promotion recorded.
6. `TypedBody` records the promotion so MIR lowering can emit the wrapper.

## Error recovery

The solver's philosophy is **absorb, don't cascade**. Reporting an error returns an `Error` `TyVar` whose `TyKind` is `Error`:

```rust
let result = ctx.report_error(InferError::TypeMismatch { … });
// result is the Error var — use it wherever the constraint's output
// would have gone.
```

`Error` unifies with anything silently. Constraints whose inputs became `Error` don't fire cascade errors. Analyzers further downstream check `cx.typed.errors.is_empty()` and suppress themselves on a tainted body.

## Adding a `Constraint` variant

1. Add it to `Constraint` in `constraint.rs` with a doc comment: what rule, eager or deferred, why.
2. Generate it where the HIR construct it represents is walked, in `generate.rs`.
3. Add a `try_solve_*` function in `solver.rs`. Return `Solved` / `Deferred` / `Errored`.
4. Wire it into the fixpoint dispatch (the big match in `fixpoint`).
5. If the constraint can fail, add the corresponding `InferError` variant (see below — **five** files).
6. Write focused tests under `lib/kestrel-test-suite/testdata/inference/`.

## Adding an `InferError` variant

From `lib/kestrel-type-infer/AGENTS.md` — the variant must be mirrored in **five** files:

1. `kestrel-type-infer/src/error.rs` — the variant plus its span arm in `InferError::span`.
2. `kestrel-type-infer/src/result.rs` — `describe_error()` match arm (short detail string).
3. `kestrel-compiler/src/diagnostic.rs` — match arm building the user-facing `Diagnostic` (message, labels, notes).
4. `kestrel-analyze/src/body/type_check.rs` — `format_error()` match arm returning `(message, label_text)`.
5. `kestrel-compiler-driver/src/lib.rs` — both `describe()` and `format_error()` arms.

Missing any one produces a non-exhaustive-match error only in a downstream crate — so do the whole set in one commit.

Emit from the solver via `ctx.report_error(InferError::YourVariant { … })`, not via the accumulator.

## Debugging

- `VERBOSE_DEBUG_OUTPUT=1 triage <test>` enables `debug_trace!` in the solver — member resolution, unification steps, where-clause lookups.
- Add `debug_trace!` calls rather than `eprintln!` so output stays filterable.
- `kestrel dump` can print the HIR and the inferred types for a `.ks` file. Useful when a constraint never fires or fires with unexpected inputs.
- **"Cannot infer type"** usually means a `TyVar` stayed `Infer` — the constraint that should have pinned it either wasn't generated, or deferred forever because its own inputs never resolved. Trace back from the unsolved var to the constraint that carries it.
- **"Type mismatch"** on what looks like compatible types — check that aliases expanded (`TypeAlias` branch in `unify`) and that `Self` substitution happened at the method entry.

## Further reading

- `lib/kestrel-type-infer/AGENTS.md` — invariants, five-file rule, memberwise-init validation.
- `lib/kestrel-type-infer/docs/architecture.md`, `design.md` — deeper design rationale.
- `.claude/skills/type-inference/` — solver crate deep-dive skill.
