# kestrel-analyze — Design

## Roslyn Inspiration

The analyzer system is modeled after Roslyn's diagnostic analyzer framework:

| Roslyn Concept | Kestrel Equivalent |
|---|---|
| `DiagnosticAnalyzer` base class | `Describe` trait |
| `RegisterOperationBlockAction` | `BodyCheck` trait |
| `RegisterSymbolAction` | `DeclCheck` trait |
| `RegisterCompilationAction` | `CompilationCheck` trait |
| `DiagnosticDescriptor` | `DiagnosticDescriptor` struct |
| Action callbacks with context | `check(&self, cx: &Context)` methods |
| Concurrent, stateless analyzers | ZST analyzers, `Send + Sync` |

Key differences from Roslyn:
- No runtime plugin loading (all analyzers compiled in). The trait system supports future external analyzers.
- No `SymbolStartAction/EndAction` scoping — unnecessary because ECS queries can read an entity's children in one shot.
- Queries provide automatic memoization (Roslyn does this via the driver's caching layer).

## The `Analyze(id, entity)` Query Model

Each `(analyzer_id, entity)` pair is an independent, memoized query:

```
Analyze("exhaustive_return", func_entity) → Vec<AnalyzeDiagnostic>
Analyze("dead_code",          func_entity) → Vec<AnalyzeDiagnostic>
Analyze("exhaustive_return", other_func)   → Vec<AnalyzeDiagnostic>
```

Benefits:
- **Granular caching**: changing one function only re-runs analysis for that function
- **Independent**: each analyzer's results are cached separately
- **LSP-friendly**: query a single `(analyzer, entity)` for targeted diagnostics

The `Analyze` query internally:
1. Reads `AnalyzerRegistryRef` from the root entity (dependency on registry)
2. Looks up the analyzer by string ID
3. For body checks: queries `LowerBody` + `InferBody` (dependency on HIR + types)
4. For decl checks: reads `NodeKind` component (dependency on entity)
5. Calls the analyzer's `check()` method
6. Returns `Vec<AnalyzeDiagnostic>`

## Incrementality via HECS Memoization

The HECS query system provides:
- **Automatic dependency tracking**: component reads and sub-queries are recorded
- **Fingerprint backdating**: if re-execution produces the same output, downstream queries skip
- **Accumulator lifecycle**: diagnostics accumulated by a query are cleared when it re-executes

This means:
- Editing a function body → `LowerBody` re-runs → `InferBody` re-runs → `Analyze` re-runs for that body
- Editing a different file → no invalidation for unchanged bodies
- Adding a new analyzer → new `Analyze` queries run, existing ones are cache hits

## Diagnostic Flow

```
Analyzer.check() produces Vec<AnalyzeDiagnostic> (with default severity)
    ↓
Analyze query returns the vec (memoized)
    ↓
analyze_bodies() orchestrator collects all diagnostics
    ↓ (future: apply severity config overrides here)
Compiler::analyze_all() returns AnalyzeSummary
    ↓
Compiler converts to codespan_reporting::Diagnostic for rendering
```

Severity configuration (future): analyzers always emit with the descriptor's default severity. The orchestrator applies user overrides (suppress, escalate, downgrade) from config before accumulating. This keeps analyzers pure.

## Adding a New Analyzer

1. Create a file in `body/` or `decl/` following the structure in AGENTS.md
2. Define `static DESCRIPTORS` with diagnostic metadata
3. Implement `Describe` + the appropriate check trait
4. Register in `default_analyzers()` in `lib.rs`
5. The framework handles memoization, routing, and accumulation
