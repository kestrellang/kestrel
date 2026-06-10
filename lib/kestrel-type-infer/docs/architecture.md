# kestrel-type-infer Architecture

Type inference and constraint solving for the Kestrel compiler. Takes HIR bodies and produces fully-typed `TypedBody` values by generating constraints, solving them via fixpoint iteration, and resolving type-dependent members.

## Pipeline Position

```
Source Text → Tokens → CST → AST Build → Name Res → HIR Lower → Type Infer → Codegen
                                                                     ^^^
                                                                  this crate
```

## Three-Phase Architecture

```
HirBody → Generate → Constraints → Solver → Substitutions → Resolve → TypedBody
          ^^^^^^^^                  ^^^^^^                   ^^^^^^^
          phase 1                   phase 2                  phase 3
```

**Generate** — Walks the HIR and emits type constraints (one per expression/pattern/statement).

**Solver** — Fixpoint iteration: processes constraints, performs unification, defers what can't be solved yet, repeats until stable.

**Resolve** — Applies final substitutions to produce concrete types. Resolves type-dependent members (fields, methods) using the TypeOracle.

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `TyVar` | `ctx.rs` | Type variable — placeholder assigned during generation |
| `TyKind` | `ctx.rs` | What a TyVar resolves to: `Named`, `Tuple`, `Function`, `Param`, `Infer`, `Never`, `Error` |
| `Constraint` | `constraint.rs` | 40+ variants: `Equal`, `Call`, `Member`, `Associated`, `ConformsTo`, ... |
| `InferCtx` | `ctx.rs` | Inference context: type registry, substitutions, constraints, deferred queue |
| `TypedBody` | `result.rs` | Final output: fully-typed expressions with resolved members |
| `InferBody` | `lib.rs` | Query entry point: entity → `Option<Arc<TypedBody>>` (see Queries) |

## Queries

All queries are keyed by `(entity, root)` and memoized per key.

| Query | Module | Output |
|-------|--------|--------|
| `InferBody` | `lib.rs` | `Option<Arc<TypedBody>>` |
| `ClosureCaptures` | `captures.rs` | `Arc<ClosureCaptureMap>` |
| `WhereClausesOf` | `where_clauses.rs` | `Vec<WhereClause>` |

**`InferBody`** — The entry point: runs all three phases for one body. Returns `None` when the entity has no HIR body (`LowerBody` produced nothing). The `TypedBody` is Arc-wrapped because memo cache hits clone the Output — a large, widely re-queried result shares one allocation instead of deep-copying.

**`ClosureCaptures`** — Place-based (disjoint) capture plan for every closure in a body. Runs *after* inference — it needs field resolutions and types — so it can capture the place `self.cap` by value instead of the whole `self`. Single source of truth for "what does each closure capture"; consumed by MIR lowering (env-struct fields, body rewriting) and `kestrel-analyze`'s `ClosureAnalyzer`. Arc-wrapped for the same memo-hit-clones-Output reason as `InferBody`.

**`WhereClausesOf`** — Resolves an entity's raw AST where clauses into structured `WhereClause` values (protocol bounds, associated-type equalities). **Declaring-scope invariant:** names in a where clause resolve in the declaring entity's own scope — there is no separate context parameter because the entity *is* the context. Never re-resolve a clause's names from a call site's scope. Usage guidance lives in `docs/contributing/type-inference.md`.

## Solver Loop

```
constraints = [initial set from generate phase]
loop {
    for constraint in constraints.drain() {
        match constraint {
            Equal(a, b) → unify(a, b)
            Call(callee, args, ret) → resolve callee type, unify params/return
            Member(recv, name, result) → resolve member on receiver type
            Associated(container, name, result) → resolve associated type
            ConformsTo(ty, protocol) → check or defer
            ...
        }
        // new constraints pushed by unification go to next round
    }
    if no_progress { break }
}
```

Constraints that can't be solved yet (e.g., receiver type still `Infer`) are deferred and retried in later rounds. The solver terminates when a round produces no new substitutions.

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | `InferBody` query, orchestration, public API |
| `generate.rs` | HIR walk → constraint emission |
| `solver.rs` | Fixpoint iteration, constraint dispatch |
| `unify.rs` | Type equality, literal guards, error/never propagation |
| `resolve.rs` | Member resolution, TypeOracle integration, `WhereClause` definition |
| `result.rs` | `TypedBody` definition, `build_result` (final substitution application) |
| `captures.rs` | `ClosureCaptures` query — post-inference closure capture plans |
| `where_clauses.rs` | `WhereClausesOf` query — entity-scoped where clause resolution |
| `constraint.rs` | `Constraint` enum (40+ variants) |
| `ctx.rs` | `InferCtx`, `TyVar`, `TyKind`, type registry, substitutions |

## Design Details

See [design.md](design.md) for comprehensive documentation of:

- All constraint variants and when they're emitted
- Literal type defaults and inference
- Closure parameter and return type inference
- Protocol conformance checking
- Where clause constraint application
- Associated type resolution
- Member resolution algorithms
- Substitution tracking and propagation

## Dependencies

| Crate | Usage |
|-------|-------|
| `kestrel-hecs` | ECS world, queries, TypeOracle |
| `kestrel-hir` | `HirBody`, `HirExpr`, `HirTy`, `HirPat` |
| `kestrel-hir-lower` | `LowerBody` (HIR input for `InferBody`/`ClosureCaptures`), AST-type lowering |
| `kestrel-ast` | Arenas, operator enums |
| `kestrel-ast-builder` | Components for entity inspection |
| `kestrel-name-res` | Resolution queries (extensions, visibility) |
| `kestrel-span` | `Span` for error reporting |
| `kestrel-debug` | `ktrace!` for debug tracing |
