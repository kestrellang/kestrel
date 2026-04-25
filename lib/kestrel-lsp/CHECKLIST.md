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
- [x] **Confirmed** `DeclSpan` component already exists in `kestrel-ast-builder/src/components.rs:43` and is set by every builder via `get_decl_span()`. No compiler changes needed.
- [x] `semantic.rs` — `body_entity_at(world, file, offset)`, `hir_expr_at(body, offset)`, `hir_expr_span(expr)`. Unit tests cover both lookups.
- [x] `ty_format.rs` — `format_ty(world, ResolvedTy)` walks `Named`/`Param`/`SelfType`/`Tuple`/`Function`/`Never`/`Error` and resolves entity paths from the world.
- [x] `handlers/hover.rs` — find body via `Valued.text_range`, run `LowerBody` + `InferBody`, look up `expr_types[id]`, render via `format_ty`. Smoke verified: hover on `42` → `lang.i64` with the literal's exact range.
- [x] `handlers/definition.rs` — Three-way dispatch on the HIR expression: `Def(entity)` → `DeclSpan`, `Local(id)` → `HirBody.locals[id].span`, `MethodCall`/`Field`/`Call`/`ProtocolCall` → `TypedBody.resolutions[id]`. Smoke verified: jump from `foo()` call to its `func foo` declaration.
- [x] `handlers/semantic_tokens.rs` — Legend declared in init capabilities (keyword, type, function, variable, property, namespace, comment, string, number, operator). Per-file: `compiler.lex(file_entity)` → classify by `Token` variant, identifiers split via PascalCase heuristic. Multi-line tokens (block comments, raw strings) are skipped (TextMate covers them). Smarter classification via `ResolveName` deferred to M3.
- [x] `Backend` advertises `hoverProvider`, `definitionProvider`, `semanticTokensProvider` and dispatches to the handlers.
- [x] M2 smoke test (`/tmp/lsp_m2.py`) — verified hover, definition, and semantic-tokens responses on a literal-only file (no stdlib needed for inference).
- [ ] Hover/definition that need stdlib operators (e.g. `a + b`) — gated on stdlib being loadable from `flock.toml` deps; works when the project's `flock.toml` resolves to the stdlib package.
- [ ] `syntax.rs` — punted; not needed by M2 since we navigate via HirExpr spans. Will likely return for M3 completion (CST node-at-position).

## M3 — Completion
- [x] No `VisibleNamesAt` query needed — `ScopeFor` already gives us declarations + selective_imports + wildcard_imports per entity, with a `parent` link for the chain. We walk that chain in the LSP rather than baking the consumer-specific shape into the compiler.
- [x] `handlers/completion.rs` — `is_after_dot` decides member vs scope; `dot_receiver_identifier` extracts the bare receiver name; `identifier_prefix` finds the partial word for filtering.
- [x] **Member completion**: receiver type → nominal `children_of` (fields, methods, init), `ExtensionsFor` → extension children, `ProtocolMembers` for protocols. Receiver resolution: locals via `LowerBody.locals` + `TypedBody.local_types`, free names via `ResolveName` in the enclosing scope. Static-on-type access (`Foo.`) handled by treating the type entity itself as `Named { entity, args: [] }`.
- [x] **Scope completion**: walks `ScopeFor` parent chain from the enclosing decl to the module root, plus locals from the enclosing `HirBody` (filtered by `local.span.start <= offset`), plus type parameters of any enclosing generic. Wildcard imports are flattened by listing each source module's children.
- [x] **Top-level snippets**: `module` / `import` / `func` / `struct` / `protocol` / `extend` injected when the enclosing entity is a `NodeKind::Module` (the file root).
- [x] Server advertises `completionProvider { trigger_characters: ["."] }`.
- [x] Smoke verified: `p.x` after a Point local → `x`/`y` fields with `FIELD` kind; `or` prefix in body → `origin` function with `FUNCTION` kind; empty file → 6 keyword snippets.
- [x] Unit tests for `syntax::identifier_prefix`, `is_after_dot`, `dot_receiver_identifier` (3 new tests, 11 total).
- [ ] **Known limitation** — Member completion needs the source to parse. The instant `foo.` is typed (no member yet), the parser fails and no `Body` component is created, so we can't find the receiver's local. Real-IDE behaviour will need a "tolerant retry": insert a placeholder identifier after the dot, re-parse, lower. Tracked for follow-up.
- [ ] Method-chain receivers (`a.b.c.`, `f().`, parenthesised) — falls through to the bare-identifier path, returns nothing. Will use a CST walk in M4/M5.

## Parser recovery — missing tokens & HirName (in flight, predates M4)
Background fix for the M3 known limitation: parser must produce a well-formed CST even on broken source so completion / hover / def can keep operating mid-edit. See `~/.claude/plans/i-want-to-build-proud-pillow.md` for the full design.
- [x] **Phase 1** — `SyntaxKind::Missing` wrapper, `Event::MissingToken { kind, at }`, `EventSink::missing_token`. Postfix dot recovers via `select!.or(empty().to(None))` + `validate`. AST builder descends through `Missing` wrappers via `member_identifier_at`. `ParseError::from_token_error` passes `RichReason::Custom` through (was getting eaten by the `expected/found` formatter). 3 unit tests: `foo.`, `foo.bar`, `foo.;`.
- [ ] **Phase 2** — `HirName` enum; migrate `HirExpr::Field` end-to-end (lowering → inference short-circuit to `ResolvedTy::Error` → analyzer suppression). Verifies the pipeline can carry "missing" without a diagnostic cascade.
- [ ] **Phase 3** — roll `HirName` to remaining variants (MethodCall, ProtocolCall, ImplicitMember, ImplicitVariant pattern, struct pat field, loop labels, call/pat arg labels, Deinit).
- [ ] **Phase 4** — apply `expect_or_missing` at other high-friction sites (`:` after param, `=` in let, `{` block open, `)` call close).
- [ ] **Phase 5** — LSP completion handler swaps text heuristics for HIR walk; delete `dot_receiver_identifier` / `is_after_dot`.
- [ ] **Phase 6** — `recover_via_skip` + statement-boundary application (skipped-token recovery; optional follow-up).

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
