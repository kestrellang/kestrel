# kestrel-analyze — Architecture

## What This Crate Does

Post-inference validation passes for the Kestrel compiler. Analyzers check HIR bodies and declaration entities for correctness issues (missing returns, dead code, mutability violations, protocol conformance, etc.) and produce rich diagnostics.

## Where It Sits in the Pipeline

```
Source → Lex → Parse → AST Build → Name Res → HIR Lower → Type Infer → **Analyze** → Codegen
```

Analyzers run after type inference. Body-level checks depend on `LowerBody` (HIR) and `InferBody` (typed body). Declaration-level checks depend only on ECS components.

## Key Types

### Analyzer Traits (`traits.rs`)

Three traits, one per granularity level:

- **`BodyCheck`** — analyze function/init bodies. Receives `BodyContext` with HIR + typed body.
- **`DeclCheck`** — analyze declarations structurally. Receives `DeclContext` with entity + kind.
- **`CompilationCheck`** — whole-compilation analysis. Receives `CompilationContext` with root entity.

All extend `Describe` which provides `id()` and `descriptors()`.

### Registry (`registry.rs`)

`AnalyzerRegistry` holds `Vec<Arc<dyn BodyCheck>>`, `Vec<Arc<dyn DeclCheck>>`, `Vec<Arc<dyn CompilationCheck>>`. Built once at compiler startup via `default_analyzers()`. Stored as `AnalyzerRegistryRef(Arc<AnalyzerRegistry>)` component on the root entity.

### Queries (`lib.rs`)

- **`Analyze { analyzer, entity, root }`** — run one analyzer on one entity. Memoized per `(analyzer_id, entity)`. Looks up the analyzer in the registry, builds the appropriate context, calls `check()`.
- **`analyze_bodies(ctx, root, entities)`** — orchestrator function. Iterates entities, calls `Analyze` for each registered body analyzer.

### Diagnostics (`diagnostic.rs`)

- **`AnalyzeDiagnostic`** — rich diagnostic with descriptor ID, severity, message, labels (primary + secondary with spans), and notes. `Clone + Hash` for HECS accumulators.
- **`DiagnosticDescriptor`** — static metadata per diagnostic kind: ID (e.g. "E001"), name, default severity, category.

### Contexts (`context.rs`)

- **`BodyContext`** — `QueryContext` + entity + root + `&HirBody` + `&TypedBody`
- **`DeclContext`** — `QueryContext` + entity + root + `NodeKind`
- **`CompilationContext`** — `QueryContext` + root

### Utilities (`util.rs`)

Shared span extraction helpers (`expr_span`, `stmt_span`, `pat_span`) and entity info helpers (`entity_name`). Used by all analyzers — see AGENTS.md for the full catalog.

## How Analyzers Are Registered and Dispatched

1. `default_analyzers()` in `lib.rs` builds an `AnalyzerRegistry` with all built-in analyzers
2. The compiler stores it as `AnalyzerRegistryRef` on the root entity
3. `Compiler::analyze_all()` collects body entities, calls `analyze_bodies()`
4. `analyze_bodies()` iterates `(analyzer_id, entity)` pairs, calling `Analyze` sub-queries
5. Each `Analyze` query looks up the analyzer by ID, builds context, calls `check()`
6. Diagnostics accumulate via HECS and are collected by the compiler for rendering

## Design Principles

- **Stateless**: Analyzers are ZSTs (zero-sized types). No mutable state between calls.
- **Incremental**: `Analyze(id, entity)` is memoized. Unchanged bodies skip re-analysis.
- **Roslyn-inspired**: Trait-per-granularity mirrors Roslyn's action registration model. String descriptor IDs enable LSP-level configuration.
- **Independent**: Analyzers don't depend on each other. Adding/removing one doesn't affect others.
