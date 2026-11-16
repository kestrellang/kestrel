# kestrel-lsp Architecture

The Kestrel language server. A `kestrel-lsp` binary that speaks LSP over stdio
(the VS Code extension spawns it) and answers editor requests from a
persistent, incrementally updated compiler.

## Pipeline Position

```
editor ⇄ JSON-RPC (stdio) ⇄ Backend (tower-lsp, async)
                               │  requests
                               ▼
                      compiler worker thread ── owns one Compiler (front end only)
                               │
                 Name Res → HIR Lower → Type Infer → Analyze
```

It uses the front end and the analyzers. It never lowers to MIR or runs
codegen.

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `Backend` | `lib.rs` | `impl tower_lsp::LanguageServer`; routes each request to `handlers/` |
| Server state | `server.rs` | State shared between handlers, URL ↔ path conversion |
| `OpenDoc` | `documents.rs` | Latest text, version and `LineIndex` per open file |
| Compiler worker | `compiler_worker.rs` | The thread that owns the `Compiler` |
| `LineIndex` | `position.rs` | Byte offsets ↔ LSP UTF-16 line/column |

## Key Design Decisions

- **One thread owns the compiler.** The hECS `World` holds `RefCell`s, so it is
  `!Send` and can't sit behind a tokio mutex or cross `spawn_blocking`. A
  dedicated worker thread owns the `Compiler` end to end, which also
  serializes access.
- **Incremental, not rebuilt.** The std loads once. When the user edits, only
  files whose content changed are despawned and rebuilt; unchanged files keep
  their entities and cached query results. `KESTREL_DEBUG=lsp-verify` builds a
  fresh compiler from the same sources and logs when the two disagree.
- **Positions go through the source map.** Every "what is at this position?"
  question resolves through `BodySourceMap` (`kestrel-hir-lower`) or the CST
  pointers on entities, never by matching spans. See `../AGENTS.md`.
- **Whole-program checks don't fire.** Analysis runs as a library
  (`analyze_all(false)`), so E618 (missing `@main`) isn't reported in the
  editor.

## Module Map

| File | Responsibility |
|------|---------------|
| `main.rs` | Stdio entry point |
| `lib.rs` | `Backend`, request routing |
| `compiler_worker.rs` | The compiler thread, invalidation, `lsp-verify` |
| `project.rs` | `flock.toml` discovery and source-set loading |
| `semantic.rs` | Cursor position → entity / HIR node |
| `references.rs` | Every site that resolves to an entity or local (used by references, rename, highlight, call hierarchy) |
| `types.rs`, `ty_format.rs` | Type-position helpers and type rendering |
| `syntax.rs` | CST navigation helpers |
| `convert.rs` | Compiler diagnostics → LSP diagnostics |
| `position.rs`, `documents.rs`, `server.rs` | Positions, open documents, shared state |
| `handlers/` | One file per LSP feature: diagnostics, hover, definition, references, rename, completion, signature help, inlay hints, semantic tokens, document and workspace symbols, highlights, code actions, code lens, call and type hierarchy |

Milestone status is in `../CHECKLIST.md`; hover design notes are in
`../HOVER.md`.

## Dependencies

| Crate | Usage |
|-------|-------|
| `kestrel-compiler`, `kestrel-compiler-driver` | The compiler and analysis runs |
| `kestrel-ast-builder`, `kestrel-name-res`, `kestrel-hir`, `kestrel-hir-lower`, `kestrel-type-infer`, `kestrel-analyze` | Entities, resolution, source maps, types, diagnostics |
| `kestrel-syntax-tree`, `kestrel-lexer`, `rowan` | CST navigation, semantic tokens |
| `kestrel-span`, `kestrel-hecs`, `kestrel-reporting`, `kestrel-debug` | Spans, world, diagnostics, `ktrace!` |
| `tower-lsp`, `tokio`, `serde`, `serde_json` | LSP protocol and async runtime |
| `toml`, `walkdir` | Project discovery |
