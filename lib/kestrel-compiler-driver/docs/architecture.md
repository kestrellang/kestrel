# kestrel-compiler-driver Architecture

"Run everything" orchestration on top of `kestrel-compiler`: whole-world
inference and analysis scans, and terminal diagnostic output.

## Pipeline Position

```
           src/main.rs (CLI)   kestrel-test-suite   kestrel-lsp (parts)
                      \               |               /
                       ──────  kestrel-compiler-driver  ──────
                                      |
                               kestrel-compiler   (per-entity queries, MIR, codegen)
```

## Why it is separate

`Compiler` (in `kestrel-compiler`) exposes per-entity queries and low-level
building blocks. The full-world scans and printing to a terminal are handy for
the CLI and the test harness, but an embedder such as the LSP shouldn't have
them imposed. They live here instead.

## Core Types

| Type | Description |
|------|-------------|
| `CompilerDriver` | Wraps a borrowed `Compiler`. `infer_all`, `analyze_all(is_executable)`, `emit_diagnostics`, `emit_diagnostics_with_warnings`, `has_errors` |
| `InferSummary` | Per-body inference results: error counts, samples, a per-variant breakdown |
| `AnalyzeSummary` | Analyzer results for the whole world |
| `ErrorSample` | One error with the entity path it came from |

`analyze_all(is_executable)` is what gates whole-program checks such as the
missing-`@main` error (E618) to executable builds.

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | Everything above, plus `error_variant_name` and `format_error` (short and debug descriptions of each `InferError`) |

Adding an `InferError` variant needs an arm in both functions here; see the
checklist in `lib/kestrel-type-infer/AGENTS.md`.

## Dependencies

| Crate | Usage |
|-------|-------|
| `kestrel-compiler` | `Compiler` |
| `kestrel-type-infer` | `InferError` |
| `kestrel-analyze` | Analyzer runs |
| `kestrel-ast-builder`, `kestrel-hecs` | Entity paths |
| `kestrel-reporting`, `codespan-reporting` | Terminal output |
