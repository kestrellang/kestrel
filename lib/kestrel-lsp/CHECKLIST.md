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
- [x] **Known limitation resolved by Parser-recovery work below** — Member completion no longer needs the source to parse cleanly. `foo.` recovers as `Field { name: HirName::Missing }` and the LSP picks up the receiver's type from inference. Verified via the `member_completion_after_trailing_dot_lists_struct_fields` unit test in `handlers/completion.rs`.
- [x] Method-chain receivers (`a.b.c.`, `f().`, parenthesised) — fixed by treating `ResolvedTy::Error` from the HIR-Field path as a miss so the existing CST-Dot fallback fires. Verified by `member_completion_on_chained_receiver`, `_on_call_receiver`, `_on_paren_receiver`, `_on_chain_with_following_stmt`, `_on_call_with_following_stmt`.

## Parser recovery — missing tokens & HirName (in flight, predates M4)
Background fix for the M3 known limitation: parser must produce a well-formed CST even on broken source so completion / hover / def can keep operating mid-edit. See `~/.claude/plans/i-want-to-build-proud-pillow.md` for the full design.
- [x] **Phase 1** — `SyntaxKind::Missing` wrapper, `Event::MissingToken { kind, at }`, `EventSink::missing_token`. Postfix dot recovers via `select!.or(empty().to(None))` + `validate`. AST builder descends through `Missing` wrappers via `member_identifier_at`. `ParseError::from_token_error` passes `RichReason::Custom` through (was getting eaten by the `expected/found` formatter). 3 unit tests: `foo.`, `foo.bar`, `foo.;`.
- [x] **Phase 2** — `HirName` enum (`Name(String) | Missing`) + `as_str` / `as_str_or_empty` / `is_missing` / `Display`. `HirExpr::Field::name` migrated end-to-end: `name_from_ast` bridge in `kestrel-hir-lower::expr` (empty string from AST → `HirName::Missing`); inference's Field arm in `generate.rs` calls `ctx.poison(result_tv)` for `Missing` and skips the `Member` constraint; consumers in mir-lower / analyze / hir-lower tests use `.as_str()` or `.as_str_or_empty()`. New integration test `missing_field_name_resolves_to_error_without_cascade` verifies no `NoMember` cascade fires for the missing case.
- [x] **Phase 3** — `HirName` migrated for `HirExpr::MethodCall::method`, `HirExpr::ProtocolCall::method`, `HirExpr::ImplicitMember::name`, `HirPat::ImplicitVariant::name`, `HirStructPatField::field_name`, `HirStmt::Deinit::name`. Inference short-circuits Missing → poison for the call sites; analyzers (`match_pattern::is_invalid`, struct-pattern field validation, refutable describe, param-pattern duplicate check, for-loop pattern destructure detection, mutability classification, move tracking, initializer init-delegation check) all branch on `as_str()`. `name_from_ast` bridge moved into `kestrel-hir-lower::ctx`. Loop / Break / Continue labels and `HirCallArg` / `HirPatArg::label` stay `Option<String>` — labels are control-flow / call-site matching, not type-inference name resolution, and the parser never recovers a `loop label:` / `arg:` with a missing identifier.
- [x] **Phase 4** — `)` closing call-args recovers via `Some.or(empty.to(None))` + `validate`. `arg_list_parser` succeeds with `rparen = None` when the close paren is absent (cursor mid-edit `foo(1, 2`), emitting "expected `)`"; `emit_call_expr` synthesizes a zero-width `Missing[RParen]` at the lparen end so source round-trips. Lets inference still type the args and completion fire on the receiver / next dot. New parser test `missing_close_paren_recovers_with_missing_node`. **Other Phase-4 sites skipped — not load-bearing:** `=` in let is already optional via `.or_not()`; `{` opening block is optional via `function_body_parser`; `:` after param would require widening `ParameterData` (`colon: Option<Span>`, `ty: Option<TyVariant>`) with a larger blast radius for marginal IDE benefit. Phase 6's block-level recovery already keeps the rest of the function parsing when a single statement is broken.
- [x] **Phase 5** — Completion handler now dispatches by HIR shape: `member_completion` finds the smallest `HirExpr::Field` covering the cursor and asks `TypedBody.expr_types` for the base's type; `Some(items)` fires member completion, `None` falls through to scope completion. `is_after_dot` and `dot_receiver_identifier` deleted from `syntax.rs`. AST builder gained a follow-up: `lower_pure_path` now detects a trailing `Dot + Missing` (or bare trailing `Dot`) and folds it into a synthesized `AstExpr::MemberAccess` with empty member, so the HIR ends up with `Field { name: HirName::Missing }` that completion can pick up. Two new unit tests: `foo.|` returns struct fields; bare identifier returns `None` so scope-completion runs.
- [x] **Phase 6** — `BlockItem::Recovered(Span)` + `block_item_recovery` parser applied via `recover_with(via_parser(...))` on the block-item alternation. Stops at `}` or any statement-starter keyword (`let`, `var`, `guard`, `deinit`, `if`, `while`, `for`, `loop`, `match`, `return`, `break`, `continue`, `throw`, `try`); refuses to consume tokens that begin an expression so `{ () }` and other tail expressions reach the trailing-expression alternative. Emit wraps the recovered range in `SyntaxKind::Error`. Two new parser tests verify (a) garbage between two `let`s no longer poisons the second one, and (b) `{ () }` still parses as a trailing expression. **Known follow-up:** garbage that starts with an expression-starter (e.g. `foo bar baz`) currently still poisons the body — the leading identifier is treated as a tail expression candidate and the trailing parse fails. Needs a "try expression, recover what's left over" pattern; tracked but not blocking.

### Polish follow-ups (deferred)
- [ ] Expression-starter garbage in a block (`foo bar baz`) still poisons the body. The leading identifier is consumed as a tail-expression candidate and the trailing parse fails before block-item recovery can fire. Needs a "try expression, then recover what's left over" pattern (probably a `recover_with` on the trailing-expression alternative that re-emits an `Error` node and restarts the block-item loop).
- [x] Method-chain receivers — completed. Root cause was the HIR-Field path returning `Some(ResolvedTy::Error)` for the outer (parser-fused) Field, which short-circuited the CST-Dot fallback. Now `Error` is treated as a miss and the CST locator finds the real receiver.
- [ ] Param-type recovery (`func foo(x` mid-edit). Skipped in Phase 4 because it requires widening `ParameterData` to `colon: Option<Span>` + `ty: Option<TyVariant>`, with downstream blast across `kestrel-ast-builder`, name-res, and inference. Worth doing once Param data is otherwise touched, but not blocking — `declaration_recovery` keeps the rest of the file parsing.

## M4 — References, rename, document symbols
- [x] `lib/kestrel-lsp/src/references.rs` — free function `references_to(world, root, target)` + `local_references` (lives in the LSP crate, not `kestrel-name-res`, because it needs `LowerBody` + `InferBody` and those sit downstream of name-res in the dep graph). Walks every `Body` entity, scans HIR `Def` / `OverloadSet` / `ProtocolCall::protocol`, pattern `Variant` / `Struct`, and inference's `TypedBody.resolutions` for member-access resolutions. Spans clipped to identifier in the handler via `clip_to_identifier`.
- [x] **Type-position references** (`Foo` in `func bar(x: Foo)` etc.) — `lib/kestrel-lsp/src/types.rs` walks every `TyPath` in a file's CST and calls `ResolveTypePath` per running prefix; sites where the resolution lands on the target are recorded. Wired into hover, definition, and references handlers — no new compiler-side query needed.
- [x] `lib/kestrel-syntax-tree/src/utils.rs` — `get_name_span(node, file_id) -> Option<Span>` helper alongside `get_decl_span`. Reads the `Name` child's `Identifier` token. Used by rename for the edit range and document_symbols for `selectionRange`.
- [x] `handlers/references.rs` — dispatches by HIR shape (Def / Local / Field / MethodCall / Call / ImplicitMember / ProtocolCall), falls back to `enclosing_decl_at` when the cursor is on a declaration's identifier. Maps `ReferenceSite`s to LSP `Location`s; clips `MemberAccess` / `Pattern` spans to the trailing identifier so the editor highlights just the name. `include_declaration` toggle appends the decl's identifier span (or `hir.locals[id].span` for locals).
- [x] `handlers/document_symbols.rs` — walks every entity tagged `FileId(file_entity)`, treats those whose parent isn't in the same file as roots, recurses via `children_of`. `range` from `DeclSpan`, `selection_range` from `get_name_span(CstNode)`. Maps `NodeKind` → `SymbolKind` (Module/Struct/Enum/EnumCase/Protocol/Extension/Function/Initializer/Field/TypeAlias/Subscript). Hides `Import`, `Setter`, `Deinit`, `ParamDefault`, `TypeParameter` from the outline.
- [x] `handlers/rename.rs` — `prepare` returns `PrepareRenameResponse::RangeWithPlaceholder { range, placeholder }` using `identifier_for_target`; disqualifies stdlib (no `FilePath` ancestor), modules, anonymous decls, and `OverloadSet` ambiguities. `rename` validates `new_name` via `kestrel_lexer::lex` (must be a single `Token::Identifier`), runs `references_to` + `push_decl_site`, collision-checks via `ResolveName` at each use site's enclosing scope (intra-body for locals), then bundles `TextEdit`s into `WorkspaceEdit { changes: HashMap<Url, Vec<TextEdit>> }`.
- [x] Server advertises `referencesProvider`, `documentSymbolProvider`, `renameProvider { prepareProvider: true }`. Trait methods `references` / `document_symbol` / `prepare_rename` / `rename` delegate to handler modules.
- [x] Tests: 28 LSP unit tests green. New tests cover references_to (cross-file function calls, no-false-positives, local-scope only), `clip_to_identifier` (Direct vs MemberAccess), document_symbols (top-level + nested fields + import exclusion + selection_range narrower than range), rename (lexer-based identifier validation, keyword rejection, collision rejection, allowed unused name, WorkspaceEdit assembly with both decl and call sites).
- [ ] Multi-file rename smoke (real `flock.toml` workspace) — covered by single-file tests; needs a multi-file fixture in the Extension Development Host pass.

## M5 — Formatting, code actions, workspace discovery, polish
- [ ] `handlers/formatting.rs` — CST trivia-preserving formatter. Deferred: multi-day work on its own; not the highest-leverage M5 item once rename + diagnostics already work in the editor. Tracked separately.
- [x] `handlers/code_actions.rs` — first quick-fix shipped: **E002 (unreachable_code)** → "Remove unreachable code" deletes the diagnostic span (extending past the trailing newline so no blank line remains). Maps via `Diagnostic.code = NumberOrString::String(descriptor_id)`. Server advertises `codeActionProvider`. **Deferred fixes:** E203/E204/E205/E206 (let→var promotion) and E005 (stub uninitialized fields) — both need binding-site lookup beyond the diagnostic's primary span; will need either a structured "binding" label on the diagnostic or a CST/HIR walk in the handler. Pattern is in place for adding more.
- [x] `project.rs` extended for registry deps via `flock.lock`. Reads `[[package]]` entries with `source = "registry"`, looks them up at `<cache>/{name}/{version}/flock.toml`, recurses into the cached package. Path-source entries also handled. Cache root defaults to `~/.kestrel/packages` (matches flock's own default) and is overridden by `kestrel.flockCachePath`. New `CollectReport { sources, missing_cache }` return type so callers know what didn't resolve.
- [x] Cache misses logged via `client.log_message(WARNING, ...)` so the editor's "Output → Kestrel Language Server" panel shows: `Kestrel: registry dep <name>@<version> not in flock cache. Run \`flock build\` to fetch.`
- [x] `workspace/didChangeWatchedFiles` — for `flock.toml` / `flock.lock` changes, clear sources and re-walk the workspace folders. For `.ks` changes, re-read the affected file (or remove it on DELETED). Refreshes diagnostics after.
- [x] `kestrel.stdlibPath` wired through `initializationOptions` → `ServerState::stdlib_path` → `load_workspace` walks the dir for `.ks` files and adds them to the source map before workspace files (so user code can resolve std imports).
- [x] `kestrel.flockCachePath` added to `package.json`, plumbed through `initializationOptions` → `ServerState::flock_cache_path` → `project::collect_sources(manifest_path, cache_root)`.
- [x] CI: added `cargo build -p kestrel-lsp --release` step to the Rust job + a new `vscode-extension` job that runs `npm ci` + `npm run compile` against `editors/vscode/`.
- [ ] `vsce package` produces a `.vsix` cleanly (sideload-installable; publishing deferred). Not run in CI yet — needs `vsce` install step.

## M6 — Inlay hints, signature help, document highlight, call & type hierarchy
High-leverage editor surface that reuses inference + overload-set data already wired for hover / completion / references. Only `subtypes` in type hierarchy needs a new compiler-side index; everything else falls out of existing queries.

### Inlay hints
- [x] `handlers/inlay_hints.rs` — `textDocument/inlayHint` over a `Range`. **Type hints on `let` / `var` without an explicit annotation** shipped: walks every `Body` in the file, iterates `HirStmt::Let { ty: None, .. }`, skips synthesized desugaring temps (`$let_tmp`, `$iter`) by name prefix, looks up `TypedBody.local_types[local]`, and emits `InlayHint { position: end-of-BindingPattern, label: ": <ty>", kind: Type }`. Position is found via a CST walk for `SyntaxKind::BindingPattern` inside the matching `VariableDeclaration`.
- [x] Server advertises `inlayHintProvider`.
- [x] Tests: simple let, var binding, annotated let (no hint), destructured let (no hint via `$`-prefix filter).
- [ ] **Parameter-name hints at call sites** (deferred). For each `HirExpr::Call` / `HirExpr::MethodCall`, look up the resolved callee in `TypedBody.resolutions`, fetch its parameter names, and emit `InlayHint { position: arg-start, label: "<name>:", kind: Parameter, padding_right: true }`. Skip when the arg already has an explicit label, when the call is variadic past the named params, or when receiver and arg trivially match.
- [ ] Settings: `kestrel.inlayHints.types` / `kestrel.inlayHints.parameterNames` (defer until paramName lands).

### Signature help
- [x] `handlers/signature_help.rs` — `textDocument/signatureHelp`. Locate the smallest `HirExpr::Call` / `HirExpr::MethodCall` / `HirExpr::ProtocolCall` whose argument-list span contains the cursor. Active-parameter index comes from the cursor's position among the `Argument` children of the CST `ArgumentList` (the parser folds each `,` into the *following* `Argument`, so direct comma-counting under-counts).
  - **Single resolution**: pull the callee entity from the callee `HirExpr::Def` (or `TypedBody.resolutions` for method/protocol calls), render `SignatureInformation { label, parameters }` from `Callable` + `TypeAnnotation`. Param types are sourced via the AST `ty` span — `format_ty` is reserved for the inferred-type fallback path.
  - **Overload sets**: every candidate of `HirExpr::OverloadSet` is rendered as its own `SignatureInformation`. `active_signature` is set to whichever candidate `TypedBody.resolutions` settled on (overload resolution's pick), else 0.
- [x] Trigger characters: `(` opens, `,` advances active parameter. Server advertises `signatureHelpProvider { trigger_characters }`.
- [x] Tests in `handlers/signature_help.rs`: cursor right after `(` → active_parameter 0; after `,` → 1; cursor on the callee identifier → no popup; nested call picks the inner signature; two overloads of `add` → both surfaced; per-parameter `LabelOffsets` slice into the rendered label correctly.
- [ ] Follow-up: doc comments in `SignatureInformation.documentation` (reuse hover's `collect_doc_comments`); type-inference-driven param types so generic substitution shows up (currently we slice the AST source, so `func foo[T](x: T)` always renders `x: T`).

### Document highlight
- [ ] `handlers/document_highlight.rs` — `textDocument/documentHighlight`. Reuses the `references_to` + local-references machinery from M4 but scoped to the active file. Maps each `ReferenceSite` to `DocumentHighlight { range, kind }`:
  - `Read` for plain expression occurrences (HIR `Def` / `MethodCall` callee / `Field` access).
  - `Write` for the LHS of `HirStmt::Assign` (and compound-assign LHS).
  - `Text` for the declaration's identifier itself.
  Spans clipped via the existing `clip_to_identifier` helper.
- [ ] Server advertises `documentHighlightProvider`.
- [ ] Tests: cursor on a local → all reads + the `let` decl + any `=` LHS marked Write; cursor on a function name → every call site in the file marked Read; cursor on a struct field → field decl marked Text and `.field` accesses marked Read.

### Call hierarchy
LSP shape: `textDocument/prepareCallHierarchy` returns one or more `CallHierarchyItem`s for the symbol under the cursor; the editor then calls `callHierarchy/incomingCalls` and `callHierarchy/outgoingCalls` on each item as the user expands the tree in the panel.
- [ ] `handlers/call_hierarchy.rs` — `prepare`: dispatch by HIR shape at the cursor (same locator as references / rename); resolve to a callable entity (function, initializer, method, protocol method). Disqualify locals, modules, types. Build `CallHierarchyItem { name, kind, uri, range: DeclSpan, selection_range: get_name_span(...) }`.
- [ ] `incomingCalls`: filter `references_to(target)` to call sites only — HIR `Call` / `MethodCall` / `ProtocolCall` whose resolved callee is `target`. For each site, find the enclosing decl via `enclosing_decl_at`, group by that decl entity, return one `CallHierarchyIncomingCall { from: <enclosing item>, from_ranges: [call-site spans] }` per group.
- [ ] `outgoingCalls`: walk `target`'s `HirBody` once, collect every `Call` / `MethodCall` / `ProtocolCall`; for each, look up the callee in `TypedBody.resolutions`, group by callee entity, emit `CallHierarchyOutgoingCall { to: <callee item>, from_ranges: [call-site spans] }`.
- [ ] Server advertises `callHierarchyProvider`.
- [ ] Tests: `prepare` on a function name returns one item; `prepare` on an overload-set call returns N items; incoming on `foo` from a file with three call sites in two functions → 2 incoming entries with the right `from_ranges`; outgoing on a function calling `bar` and `baz.qux()` → 2 outgoing entries.

### Type hierarchy
LSP shape: `textDocument/prepareTypeHierarchy` → `typeHierarchy/supertypes` / `subtypes`. Same prepare-then-expand pattern as call hierarchy.
- [ ] `handlers/type_hierarchy.rs` — `prepare`: resolve cursor to a type entity (nominal or protocol). Build a `TypeHierarchyItem` with `kind` (Class/Struct/Enum/Interface), `range: DeclSpan`, `selection_range: get_name_span(...)`.
- [ ] `supertypes`: for a nominal, read its `ProtocolConformance` component — return one `TypeHierarchyItem` per conformed protocol. For a protocol, read its inherited-protocol list (`where Self: Foo` clauses on the protocol decl).
- [ ] `subtypes`: for a protocol, return every nominal that conforms. **Pilot with an O(n) scan over all nominal entities** filtering by `ProtocolConformance.contains(target)` — fine at current corpus size. If it shows up in profiles, promote to a `ConformersOf { protocol }` query in `kestrel-name-res` (cached, invalidated on any conformance change). For a nominal, return empty (no nominal subtyping in Kestrel).
- [ ] Server advertises `typeHierarchyProvider`.
- [ ] Tests: prepare on `struct Point` returns one item with kind Struct; supertypes lists every protocol `Point` conforms to; prepare on `protocol Iterator` → subtypes lists every conforming nominal in the workspace; protocol with inherited protocols lists them as supertypes.

## Cross-cutting / quality gates
- [x] Unit tests for `position.rs`
- [ ] Unit tests for `convert.rs`, `syntax.rs::node_at_offset`
- [ ] Integration test harness driving the LSP over in-memory stdio with fixtures from `lib/kestrel-test-suite/testdata/`
- [ ] `triage` full run green before merging any M2+ change that touches `kestrel-ast-builder` or `kestrel-name-res`
- [ ] No `eprintln!` / `println!` in compiler crates introduced for LSP debugging — use `debug_trace!` (per CLAUDE.md)
- [x] No two queries running concurrently against the same `World` (single fresh `Compiler` per pass; analysis runs in `spawn_blocking`)
- [x] Position math is the single source of truth in `position.rs`; no ad-hoc offset arithmetic elsewhere
