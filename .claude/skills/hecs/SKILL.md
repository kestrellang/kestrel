---
name: hecs
description: Reference for the hECS (Hierarchical Entity Component System) architecture used by the lib2 Kestrel compiler — entities, components, queries, and the Salsa-style memoization model. Use when answering "why is it structured this way?", "what's a component vs a query?", "where should this new fact live?", "why are struct/enum unified as Nominal?", or when designing a new query/analyzer and you need to follow the existing conventions. Historical migration notes (now completed) also live here for rationale.
---

# hECS

The lib2 compiler is built on **hECS** — Hierarchical Entity Component System — combined with Salsa-style memoized queries. The migration from the lib1 symbol-tree architecture is complete; this skill is the **reference** for how the model works now and *why* it was built this way.

## The model in one paragraph

**Entities** are declarations (structs, functions, fields, enum cases, extensions, …) with stable IDs assigned during syntax-tree construction. **Components** are syntax-derived, purely local facts attached to entities (Name, Span, GenericParams, FieldList, Conformances, Body, …) — no cross-entity lookups, so they can be extracted from one file's syntax alone. **Queries** are the computation model: memoized, dependency-tracked functions that do everything requiring cross-entity knowledge (type resolution, conformance, method lookup, body resolution, diagnostics). There are no explicit BUILD/BIND/VALIDATE phases — the query graph is demand-driven, and incremental invalidation happens when a file's inputs change.

## Which file to open

- **`architecture.md`** — the canonical hECS definition: Entity, Component, System, Query. Open this first when someone asks "what *is* hECS?" or needs the vocabulary.
- **`queries.md`** — the query catalog and tiering. Use when adding a new query, deciding whether something should be a component or a query, or looking up an existing query's call sites / purpose. Also documents the Salsa-style dependency-tracking model and how phases dissolved into the query graph.
- **`deduplication.md`** — design rationale for the big structural consolidations: why `TyKind::Struct/Enum/Protocol/TypeAlias` collapsed into a single `Nominal`, why the transformation pipeline is one trait, why conformance checking is one function for struct+enum. Open this when you're tempted to add a new `match` arm that special-cases struct vs enum — the answer is almost always "use the nominal / component abstraction instead".
- **`big-steps.md`** — the four large cross-cutting efforts (move all resolution to inference, unified type transformation, parser rewrite, symbol mangling rewrite). Reference for *why* the current architecture looks like it does at the seams between stages.
- **`GOALS.md`** — the original success criteria (incremental compilation, LSP usability, complex queries, shorter code, test suite passes). Useful as a yardstick when evaluating a proposed change.

## When to use this skill

Reach for it when the question is about the **shape** of the compiler, not the mechanics of a specific pipeline stage:

- "Should this be a component or a query?" → `queries.md` (components are pure/local; queries cross entities)
- "Why isn't there a `TyKind::Enum`?" → `deduplication.md` (Nominal unification)
- "Where do I add a new cross-cutting fact about a declaration?" → `queries.md` tiering guide
- "Why does the solver handle `x.foo()` instead of the binder?" → `big-steps.md` §1 (move all resolution to inference)
- "What invariants must a new analyzer respect?" → `architecture.md` (systems operate on components, not entity kinds)

For stage-specific routing ("where does `HirExpr::Match` get built?"), use `kestrel-pipeline` instead. For writing `.ks` code, use `write-kestrel`. For compiler-internals debugging, use `debug-kestrel`.

## Caveats

- The detail docs were originally written as **migration plans**. The migration landed, so treat "Proposed:" / "Migration steps" sections as historical rationale, not TODOs. The *design decisions* they document are still load-bearing.
- Line counts and file paths cited inside the detail docs (e.g., "`type_oracle.rs` line ~4201") reflect the lib1 codebase at the time of planning. The lib2 layout differs — grep for the symbol name, don't trust the line number.
- Query counts ("48 queries in `kestrel-semantic-model/src/queries/`") are snapshots. If you need the current list, run `rg 'impl Query for' lib2/` rather than trusting the catalog.
