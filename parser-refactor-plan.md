# kestrel-parser Refactor Plan

Status as of this note: work is being done one small step at a time, with a
commit after each passing parser-focused step.

## Test Policy

- For parser-only refactor steps, run `cargo test -p kestrel-parser`.
- Do not run `triage` unless a step touches broader compiler behavior or the
  test-suite harness.
- Documentation-only steps do not require tests.

## Completed

### Step 1: Document Parser Contract

The parser architecture document already captures the target contract:

- parser owns syntax recognition only
- CST shape and diagnostics are downstream-facing
- Chumsky/internal data structs should not become downstream dependencies
- trivia is intended to become truly lossless
- operator precedence/associativity intentionally belongs to the later Pratt
  parser, not `kestrel-parser`

No new commit was made during this pass because the document was already at the
desired state in `HEAD`.

### Step 2: Add Characterization Tests

Commit: `e49555e8 test: characterize parser behavior`

Added focused parser characterization coverage for:

- inter-declaration trivia text preservation
- nested struct/enum declarations
- flat operator token preservation for the later Pratt parser

Verification: `cargo test -p kestrel-parser`

### Step 3: Remove Double Declaration Routing

Commit: `65f4717d refactor: use declaration router for single item parsing`

Changed `parse_declaration_item` to use the same Chumsky declaration router as
source-file parsing, removing the previous trial-parse ladder and temporary
event-sink copying.

Verification: `cargo test -p kestrel-parser`

### Step 4a: Move Module Declaration Ownership

Commit: `933664b8 refactor: move module declaration parsing into module`

Moved module declaration parsing/emission out of `common` and into
`module/mod.rs`.

`emit_module_path` remains shared for now because imports and module paths still
use it.

Verification: `cargo test -p kestrel-parser`

### Step 4b: Move Import Declaration Ownership

Commit: `d88f6deb refactor: move import declaration parsing into import`

Moved import declaration parser helpers and import declaration emission out of
`common` and into `import/mod.rs`.

Verification: `cargo test -p kestrel-parser`

### Step 4c: Move Type Alias Data And Emission

Commit: `ca93d3fb refactor: move type alias data into type_alias`

Moved type alias parse data and emission into `type_alias/mod.rs`.

`common` still references `TypeAliasDeclarationData` in shared body item enums,
but the type is now owned by `type_alias`.

Verification: `cargo test -p kestrel-parser`

### Step 4d: Move Field Declaration Data And Emission

Commit: `ecc224c4 refactor: move field declaration data into field`

Moved field declaration data structs (`FieldDeclarationData`, `ComputedBodyData`),
parser internals (`field_declaration_parser_internal`, `computed_body_parser`),
and emitters (`emit_field_declaration`, `emit_property_accessors`) into
`field/mod.rs`.

### Step 4e: Move Function Declaration Data And Emission

Commit: `965f8f9a refactor: move function declaration data into function`

Moved `FunctionDeclarationData`, `ReceiverModifier`,
`function_declaration_parser_internal`, `receiver_modifier_parser`, and
`emit_function_declaration` into `function/mod.rs`.

`FunctionBodyData`, `ParameterData`, and related emitters stay in `common`
because they are shared with initializer, subscript, and deinit declarations.

### Step 4f: Move Subscript Declaration Data And Emission

Commit: `a225b590 refactor: move subscript declaration data into subscript`

Moved `SubscriptDeclarationData`, `SubscriptBodyData`, the internal parsers,
and `emit_subscript_declaration` into `subscript/mod.rs`.

### Step 4h: Move Protocol Declaration Data And Emission

Commit: `57232b90 refactor: move protocol declaration data into protocol`

Moved `ProtocolDeclarationData`, `ProtocolBodyItem`, and
`emit_protocol_declaration` into `protocol/mod.rs`. The parser was already
colocated there.

### Step 4i: Move Extension Declaration Data And Emission

Commit: `5b1cd319 refactor: move extension declaration data into extension`

Moved `ExtensionDeclarationData`, `ExtensionBodyItem`, and
`emit_extension_declaration` into `extension/mod.rs`. The parser was already
colocated there.

### Step 4j: Move Struct/Enum Declaration Data And Emission

Commit: `9a27e97e refactor: move struct and enum declaration data into their modules`

Moved `StructDeclarationData` + `emit_struct_declaration` to `struct/mod.rs`.
Moved `EnumDeclarationData`, `EnumCaseDeclarationData`,
`EnumCaseParameterData`, and their emitters to `enum_decl/mod.rs`.

`type_decl.rs` remains the mutual-recursion coordinator. `TypeDeclarationBodyItem`
and `emit_type_declaration_body_item` stay in `common` as the shared dispatcher.

### Step 4g (skipped): Initializer / Deinitializer

No separate `initializer/` or `deinit/` directory exists and these declarations
are never surfaced as top-level `DeclarationItem` variants. Their data types
(`InitializerDeclarationData`, `DeinitDeclarationData`), parsers, and emitters
remain in `common`. Revisit only if their ownership becomes painful.

### Step 5: Shared Parse Context

Commit: `5711e662 refactor: introduce parse_and_emit! macro to reduce parser-entry boilerplate`

Added `parse_and_emit!` macro in `crate::input` that centralizes the repeated
`prepare_tokens → create_input → match parse` pattern. Applied to every
non-custom parser entry point (declarations, module, import, ty, stmt, block,
declaration_item, source file).

Parser wrappers with custom error-recovery shape (`expr`, `pattern`) keep their
explicit bodies for now.

### Step 11: Narrow Public API

Commit: `422a037a refactor: make common and type_decl modules crate-private`

`common` and `type_decl` are now `pub(crate)`. External consumers
(`kestrel-compiler`) only use top-level parse entry points and `ParseResult`,
so narrowing the internal modules matches their role as internal scaffolding.

### Step 6: Make Trivia Policy Explicit In Code

Added `SyntaxKind::Newline` so newlines are distinct from horizontal whitespace.
`Token::Newline` now maps to `SyntaxKind::Newline` instead of `Whitespace`.

Rewrote `TreeBuilder::emit_trivia_until` to re-lex each inter-token gap so
whitespace, newlines, line comments, and block comments are preserved as their
distinct kinds rather than lumped into `Whitespace`. Added trailing-trivia
emission at the outermost `FinishNode` so `tree.text()` round-trips the source.

Updated `is_trivia` in `kestrel-syntax-tree/utils.rs` to include `Newline`.

New/updated tests in `parser.rs` and `event.rs`:

- `trivia_kinds_are_distinct_between_declarations` (replaces the old
  characterization test that documented the previous lumped behavior)
- `trivia_round_trips_block_and_line_comments`
- `trailing_trivia_is_preserved_in_tree`
- `tree_builder_classifies_inter_token_trivia_by_kind`
- `tree_builder_emits_trailing_trivia_after_last_token`

Verification: `cargo test -p kestrel-parser`

### Step 12: Deliberate Top-Level Error Recovery

Added an explicit recovery anchor at the top-level declaration loop so a
malformed declaration no longer poisons the rest of the file. The anchor
predicate `is_declaration_starter` matches the keywords (and `@`) that can
begin a declaration; when `declaration_item_parser_internal` fails,
`declaration_recovery` skips leading trivia, consumes at least one non-trivia
token (to guarantee progress), then consumes tokens until the next declaration
starter or EOF. The recovered span becomes a `DeclarationItemData::Error`
variant that emits as an `Error` syntax node plus a diagnostic pinned at the
skipped range.

Fixed a related bug in `parse_and_emit!` that was discarding the parser's
output whenever any error was emitted. Chumsky's `into_result()` returns
`Err` even on recovered successes, so we now forward both the output and the
errors to the sink independently. This fix is a prerequisite for recovery to
surface in the tree.

Recovery does NOT fire on:

- trailing whitespace/comments after the last declaration (filter requires at
  least one non-trivia token)
- EOF (no token to consume)

Not yet covered (explicit anchors remain implicit):

- `}` inside block/body grammars
- `;` inside statement grammars
- recovery inside expressions and patterns

These can be added in a follow-up slice using the same pattern.

New tests in `parser.rs`:

- `recovery_preserves_declarations_around_garbage_region`
- `recovery_error_span_covers_skipped_garbage`
- `recovery_does_not_fire_on_trailing_trivia`

Verification: `cargo test -p kestrel-parser`

## Remaining Plan

### Step 7: Split `expr/mod.rs`

Break expression implementation into focused modules. Suggested layout:

- `expr/data.rs`
- `expr/atom.rs`
- `expr/postfix.rs`
- `expr/control.rs`
- `expr/closure.rs`
- `expr/operators.rs`
- `expr/emit.rs`
- `expr/mod.rs` as facade

Do not add parser-level precedence. Operator handling should preserve syntax
order for the later Pratt parser.

### Step 8: Clarify Block/Stmt/Expr Boundaries

Reduce duplicated inline statement/block parsing inside `expr`.

Target ownership:

- `block` owns block grammar
- `stmt` owns statement grammar
- `expr` calls into those through narrow recursive hooks

### Step 9: Tame Struct/Enum Mutual Recursion

Keep a unified recursive parser if needed, but make `type_decl.rs` a coordinator
rather than the owner of all type-body syntax.

Target ownership:

- enum cases live in `enum_decl`
- struct-specific body rules live in `struct`
- shared recursion glue stays in `type_decl`

### Step 10: Make Emitters Harder To Misuse

Either colocate emitters next to parser data, or introduce a small trait such as
`EmitSyntax`.

Goal: adding a syntax field should fail compilation or local tests unless
emission is handled.

## Acceptance Criteria

The refactor is in a good state when:

- `common` is small and boring
- no declaration is parsed in two independent ways
- declaration ownership is obvious from module names
- `expr/mod.rs` is a facade, not a large implementation file
- trivia behavior is documented, implemented, and tested
- operator syntax is preserved without parser-level precedence semantics
- adding syntax touches one obvious parser module plus downstream AST/HIR work,
  not a hidden chain of shared hubs
