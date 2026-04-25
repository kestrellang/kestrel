# Kestrel LSP + VSCode Extension — Checklist

Tracks progress against the milestone plan. Tick each item as it lands.

## M0 — Scaffolding
- [x] Add `lib/kestrel-lsp` to root `Cargo.toml` `[workspace] members`
- [x] `lib/kestrel-lsp/Cargo.toml` — depend on `tower-lsp`, `tokio`, `kestrel-compiler`, `kestrel-compiler-driver`, `kestrel-reporting`, `kestrel-span`, `kestrel-analyze`, `kestrel-hecs`, `serde`, `toml`, `walkdir`
- [x] `src/main.rs` — stdio `tower_lsp::Server::new(...).serve(...)` boilerplate
- [x] `src/lib.rs` — `Backend` struct + `impl LanguageServer`
- [x] `src/server.rs` — shared state, URL ↔ path conversion, fresh-`Compiler`-per-pass strategy (see comment for rationale)
- [x] `src/documents.rs` — `OpenDoc { line_index, version }`, full-text replace on `didChange`
- [x] `cargo build -p kestrel-lsp` succeeds

## M1 — Document sync, diagnostics, TextMate grammar
### Server
- [x] `position.rs` — `LineIndex` with line-start byte offsets and per-line UTF-16 column mapping
- [x] `position.rs` unit tests (ASCII, multibyte UTF-8, emoji, CRLF, past-EOF, empty doc)
- [x] `convert.rs` — `codespan_reporting::Diagnostic<usize>` → `lsp_types::Diagnostic` (severity, message, primary range, related labels)
- [x] `convert.rs` — `AnalyzeDiagnostic` → `lsp_types::Diagnostic` (descriptor_id as code)
- [x] `handlers/diagnostics.rs` — `set_source`, run `infer_all` + `analyze_all` on `spawn_blocking`, publish per-file
- [x] `project.rs` — walk-up to nearest `flock.toml`, parse `[package]` and `[dependencies]`, recursively load `.ks` files from `[package].source` and path deps (registry deps deferred to M5)
- [x] `initialize`/`initialized`/`shutdown` advertising correct `ServerCapabilities`
- [x] `didOpen`/`didChange`/`didClose`/`didSave` wired
- [x] Reanalysis on `spawn_blocking`; stale results dropped via `revision_token` check
- [ ] Debounce reanalysis (~150ms) — currently fires on every change; cheap enough at M1 sizes
- [ ] Make `Compiler::build` idempotent or add a rebuild API in `kestrel-ast-builder` so we can drop the `Compiler::new()`-per-pass strategy

### TextMate grammar
- [x] `editors/vscode/syntaxes/kestrel.tmLanguage.json`
  - [x] keywords (control / declaration / modifier / language constants)
  - [x] line, doc, and block comments (block comments do nest in the lexer; TextMate doesn't model nesting fully)
  - [x] string literals: regular, raw triple-quoted, escape sequences, `\(...)` interpolation
  - [x] numeric literals (int, float, hex, binary, octal, with `_` separators)
  - [x] decorators / attributes (`@name`)
  - [x] type names (PascalCase heuristic + builtin types)

### Extension
- [x] `editors/vscode/package.json` — `contributes.languages` (id `kestrel`, ext `.ks`), `contributes.grammars`, `activationEvents`, `main: ./out/extension.js`
- [x] `language-configuration.json` — comments, brackets, autoclosingPairs, surroundingPairs, wordPattern
- [x] `src/extension.ts` — bootstrap `LanguageClient` with `ServerOptions { command, transport: stdio }` + `kestrel.restartServer` command
- [x] Settings `kestrel.lsp.path`, `kestrel.lsp.trace.server`, `kestrel.stdlibPath`
- [x] `tsconfig.json`, `.vscodeignore`, `README.md`
- [x] `npm install && npm run compile` produces `out/extension.js`
- [x] Stdio smoke test: initialize → didOpen of a file with a stray backtick → publishDiagnostics with the parse error at the right range
- [ ] Manual Extension Development Host pass against `examples/pong/pong.ks`

## M2 — Hover, go-to-definition, semantic tokens
- [ ] `syntax.rs::node_at_offset` (rowan `token_at_offset` + walk-up)
- [ ] **Confirm or add** `DeclSpan` component on declaration entities in `kestrel-ast-builder` (verify first; ask before modifying)
- [ ] `handlers/hover.rs` — find body, run `InferBody`, look up expr type, render via existing `kestrel-type-infer`/`kestrel-reporting` printer
- [ ] `handlers/definition.rs` — `ResolveName` → entity → `DeclSpan` → `lsp_types::Location`
- [ ] `handlers/semantic_tokens.rs` — declare legend (keyword, type, function, parameter, property, namespace, comment, string, number, operator), full + delta (delta optional in M2)
- [ ] Server advertises `hoverProvider`, `definitionProvider`, `semanticTokensProvider` in `initialize`
- [ ] Hover unit test against an `InferBody` fixture
- [ ] Definition unit test: function call → declaration

## M3 — Completion
- [ ] **Add query** `VisibleNamesAt { file, byte_offset }` in `kestrel-name-res` — share scope chain with `ResolveName`
- [ ] `handlers/completion.rs` — detect `.` trigger vs identifier prefix
- [ ] Member completion: receiver type → `ProtocolMembers` + nominal members + `ExtensionsFor`
- [ ] Scope completion: `VisibleNamesAt` filtered by prefix
- [ ] Snippet completions for top-level keywords on empty lines (`func`, `struct`, `protocol`, `extend`, `import`)
- [ ] Server advertises `completionProvider { triggerCharacters: ["."] }`
- [ ] Test: `ball.` in pong example yields field/method list

## M4 — References, rename, document symbols
- [ ] **Add query** `ReferencesTo { entity }` in `kestrel-name-res` — scans `TypedBody.resolutions` + AST paths
- [ ] `handlers/references.rs` — return `Vec<Location>`
- [ ] `handlers/rename.rs` — collision check via `ResolveName(new_name)` at each site, build `WorkspaceEdit`, support `prepareRename`
- [ ] `handlers/symbols.rs` — `documentSymbol` walks module decls into `DocumentSymbol` tree
- [ ] Server advertises `referencesProvider`, `renameProvider { prepareProvider: true }`, `documentSymbolProvider`
- [ ] Test: rename a struct field across multiple files
- [ ] Test: outline shows nested types correctly

## M5 — Formatting, code actions, workspace discovery, polish
- [ ] `handlers/formatting.rs` — CST trivia-preserving formatter (whitespace/indent only in M5)
- [ ] `handlers/code_actions.rs` — map `AnalyzeDiagnostic.descriptor_id` → quick-fix; implement top 3–5 fixes
- [ ] Extend `project.rs` to resolve registry deps via `flock.lock` + flock's cache dir; load resolved sources via `set_source`
- [ ] If cache is missing for a pinned dep, surface a warning + suggest running `flock build`; do NOT auto-fetch from the LSP
- [ ] `workspace/didChangeWatchedFiles` handler for `flock.toml` and `flock.lock` → reload project sources (filesystem watchers are already registered in `extension.ts`)
- [ ] Wire `kestrel.stdlibPath` setting through to the compiler config
- [ ] Setting `kestrel.flockCachePath` (default = flock's default cache location) for non-standard installs
- [ ] CI: add `cargo build -p kestrel-lsp --release` and `cd editors/vscode && npm ci && npm run compile`
- [ ] `vsce package` produces a `.vsix` cleanly (sideload-installable; publishing deferred)

## Cross-cutting / quality gates
- [x] Unit tests for `position.rs`
- [ ] Unit tests for `convert.rs`, `syntax.rs::node_at_offset`
- [ ] Integration test harness driving the LSP over in-memory stdio with fixtures from `lib/kestrel-test-suite/testdata/`
- [ ] `triage` full run green before merging any M2+ change that touches `kestrel-ast-builder` or `kestrel-name-res`
- [ ] No `eprintln!` / `println!` in compiler crates introduced for LSP debugging — use `debug_trace!` (per CLAUDE.md)
- [x] No two queries running concurrently against the same `World` (single fresh `Compiler` per pass; analysis runs in `spawn_blocking`)
- [x] Position math is the single source of truth in `position.rs`; no ad-hoc offset arithmetic elsewhere
