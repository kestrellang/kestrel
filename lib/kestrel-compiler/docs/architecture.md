# kestrel-compiler Architecture

Orchestration layer for the Kestrel compiler frontend. Wraps the `kestrel-hecs` world and provides a high-level query-based API that drives the full compilation pipeline: lexing, parsing, AST building, and type inference.

## Pipeline Position

```
Source Text → Lex → Parse → AST Build → Name Res → HIR Lower → Type Infer
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                          this crate orchestrates all phases
```

The compiler owns the ECS world and provides methods that invoke each phase. Lex and parse are memoized queries; AST building runs in the mutation phase; name resolution, HIR lowering, and type inference run as queries in the read phase.

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `Compiler` | `lib.rs` | Main database. Owns the ECS `World`, manages file entities, runs queries |
| `SourceText` | `components.rs` | Component storing raw source text for a file entity |
| `FilePath` | `components.rs` | Component storing the display path for a file entity |
| `Diagnostic` | `diagnostic.rs` | Error/warning with span, message, and severity |
| `Severity` | `diagnostic.rs` | `Error` or `Warning` |
| `InferSummary` | `lib.rs` | Statistics from a type inference run: error counts, samples |

## Query System

| Query | Module | Input → Output |
|-------|--------|---------------|
| `LexFile` | `queries/lex.rs` | File entity → `Vec<SpannedToken>` |
| `ParseFile` | `queries/parse.rs` | File entity → `ParseResult` (CST + errors) |

Both queries accumulate `Diagnostic` values as side effects. Dependencies are tracked automatically — changing `SourceText` invalidates `LexFile`, which invalidates `ParseFile`.

## Key Methods

| Method | Description |
|--------|-------------|
| `new()` | Creates compiler with empty world and seeded lang module |
| `set_source(path, source)` | Adds or updates a source file, returns entity handle |
| `lex(entity)` | Runs the `LexFile` query |
| `parse(entity)` | Runs the `ParseFile` query |
| `build(file_entity)` | Calls `build_declarations` to create AST entities (mutation phase) |
| `infer_all()` | Runs type inference on all entities with `Body` components |
| `load_dir(path)` | Loads all `.ks` files from a directory recursively |
| `diagnostics()` | Collects all accumulated diagnostics from the current revision |

## Incrementality

```
set_source("foo.ks", new_text)   ← marks SourceText changed
    │
    ▼
LexFile                          ← re-executes (input changed)
    │
    ▼
ParseFile                        ← re-executes (dependency changed)
    │
    ▼
build_declarations               ← re-runs for changed file
    │
    ▼
downstream queries               ← re-execute only if AST fingerprints differ (backdating)
```

Unchanged files are served from cache at every level.

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | `Compiler` struct, `InferSummary`, high-level orchestration |
| `components.rs` | `SourceText`, `FilePath` components |
| `diagnostic.rs` | `Diagnostic`, `Severity` |
| `queries/lex.rs` | `LexFile` query implementation |
| `queries/parse.rs` | `ParseFile` query implementation |

## Dependencies

| Crate | Usage |
|-------|-------|
| `kestrel-hecs` | ECS world, queries, accumulators |
| `kestrel-lexer` | Token types, `lex()` function |
| `kestrel-parser` | `Parser`, `ParseResult` |
| `kestrel-syntax-tree` | `SyntaxKind`, `SyntaxNode` |
| `kestrel-ast-builder` | `build_declarations` |
| `kestrel-type-infer` | `InferBody` query |
| `kestrel-span` | `Span` for diagnostics |
