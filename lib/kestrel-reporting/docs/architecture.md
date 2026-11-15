# kestrel-reporting Architecture

Diagnostic rendering. A thin layer over `codespan-reporting` that every stage
uses to collect diagnostics and print them to the terminal.

## Pipeline Position

```
every stage → Diagnostic<usize> → kestrel-reporting → stderr
```

It sits beside the pipeline rather than in it: stages build
`codespan_reporting` diagnostics (re-exported here as `Diagnostic`, `Label`,
`Severity`) and the driver or CLI prints them.

## Core Types

| Type | Description |
|------|-------------|
| `DiagnosticContext` | A self-contained diagnostic sink with its own file table: `add_file`, `throw`, `add_diagnostic`, `has_errors`, `emit`, `emit_to` |
| `ToDiagnostic` | A trait for error types that convert into a `Diagnostic` |
| `emit_all(files, diagnostics)` | Print diagnostics against any `Files` implementation, for when the file table lives elsewhere (such as the ECS world) |

## Key Design Decisions

- **One bad span doesn't stop the batch.** `emit_all` renders each diagnostic
  separately, so one with a synthetic or missing span can't hide the rest.
- **Messages are written by their stage.** Inference messages come from
  `InferError::render` in `kestrel-type-infer`; this crate only prints.

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | Everything above |

## Dependencies

| Crate | Usage |
|-------|-------|
| `codespan-reporting` | Diagnostic types and terminal rendering |
