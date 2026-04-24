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

### Step 7a: Split Expression Data And Emit

Extracted the two mechanically-separable halves of `expr/mod.rs` into sibling
modules:

- `expr/data.rs` (310 lines) — the 12 public data types (`ExprVariant`,
  `PathSegmentData`, `TypeArgsData`, `CallArg`, `ArgumentListData`,
  `MatchArmData`, `MatchGuardData`, `LabelData`, `ClosureParamsData`,
  `ClosureParamData`, `ElseClause`, `IfCondition`)
- `expr/emit.rs` (990 lines) — every `emit_*_expr` function, the
  `emit_expr_variant` dispatcher, and the interpolation-detection helpers
  (`string_contains_interpolation`, `maybe_convert_to_interpolated`)

`expr/mod.rs` now re-exports these so existing external imports keep working.
Net shrink: 3199 → 1943 lines in `expr/mod.rs`.

Verification: `cargo test -p kestrel-parser`

### Step 7b: Extract Expr Atoms, Operators, and Simple Control-Flow

Pulled the three chunks of `expr_parser` that don't need shared sub-parser
plumbing into their own modules:

- `expr/atom.rs` (107 lines) — `full_type_args_parser`, `literal_parser`
  (integer/float/string/raw-string/char/bool/null combined), `path_segment_parser`,
  `path_parser`.
- `expr/operators.rs` (78 lines) — `unary_op_parser`, `binary_op_parser`,
  `compound_assign_op_parser` (pure token-level recognisers).
- `expr/control.rs` (101 lines) — `break_parser`, `continue_parser`,
  `return_parser(expr)`, `throw_parser(expr)`, `try_keyword_parser`,
  `label_parser`.

`return` and `throw` take the recursive `expr` handle as a generic `impl
Parser<...> + Clone` parameter, which is the factory pattern for sub-parsers
that need the recursion handle.

`expr/mod.rs`: 1943 → 1788 lines (−44% from pre-Step 7 baseline of 3199).

Remaining under Step 7 (deferred as Step 7c, not pursued in this pass): the
postfix section (arg-list / member-access / tuple-index / postfix-bang,
built into primary via `attach_trailing_closures`) and the closure section
(`build_closure_expr` and trailing-closure argument). Both capture many
other shared sub-parsers defined inside the `recursive(|expr| ...)` closure
(`inline_code_block`, `inline_var_decl`, `condition_binary`, pattern and
statement parsers) and would require threading those through as parameters
or building a shared context struct. That's a larger structural rewrite.

Verification: `cargo test -p kestrel-parser`

### Step 10: Make Emitters Harder To Misuse

Added an `EmitSyntax` trait in `event.rs` so every parser-data type can be
emitted through a single uniform method (`data.emit(sink)`). The trait is
paired with a stronger discipline in each implementor: every emitter for the
declaration-level data structs now destructures its `Data` argument with a
non-exhaustive-free pattern (no `..` rest). Adding a field to any of these
structs is therefore a compile error until the field is handled in emission.

Converted emitters for:

- `StructDeclarationData`
- `FunctionDeclarationData`
- `FieldDeclarationData`
- `EnumDeclarationData`
- `EnumCaseDeclarationData`
- `ExtensionDeclarationData`
- `ProtocolDeclarationData`
- `SubscriptDeclarationData`
- `TypeAliasDeclarationData`
- `InitializerDeclarationData`
- `DeinitDeclarationData`
- `VariableDeclarationData`
- `DeinitStatementData`

Enum dispatchers (`emit_declaration_item`, `emit_type_declaration_body_item`,
`emit_stmt_variant`, `emit_expr_variant`, `emit_ty_variant`) already use
exhaustive `match`, so variant additions are already compile errors. The
destructuring discipline closes the complementary gap for struct-field
additions.

Added `emit_syntax_impl_matches_free_function` smoke test in
`struct/mod.rs` that verifies the trait impl produces the same tree as the
free function, locking in the trait contract.

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

### Step 7c: Postfix And Closure Extraction

Extracted the final two large sections of `expr_parser` into their own
modules, threading the recursive `expr` handle (and, for closures, the inline
`let`/`var` parser) through as explicit generic parameters:

- `expr/postfix.rs` (215 lines) — owns `PostfixOp`, the argument/arg-list/
  member-access/postfix-bang parsers, the combined `postfix_op_parser`, and
  the pure `fold_postfix_ops` helper. Also exposes `argument_parser` so
  `implicit_member_access` can share the labeled-vs-unlabeled logic.
- `expr/closure.rs` (293 lines) — owns `closure_parser` (the full
  `{ params in body }` parser with guard-let/inline-stmt handling) and
  `trailing_closure_arg_parser`. The factory takes `expr` and
  `inline_var_decl` (an `impl Parser ... StmtVariant`) as generics.

`is_inline_statement_like` is now `pub(super)` so both modules can import it.

`expr/mod.rs` final size: 1405 lines (3199 → 1405 across Steps 7a, 7b, 7c —
a 56% reduction). All 7 submodules suggested by the original plan now exist:

| File | Lines | Role |
| --- | --- | --- |
| `expr/data.rs` | 310 | Public data types |
| `expr/emit.rs` | 990 | All emit functions |
| `expr/atom.rs` | 107 | Literals, paths, type args |
| `expr/postfix.rs` | 215 | Calls, member access, postfix ops |
| `expr/operators.rs` | 78 | Unary/binary/compound-assign tokens |
| `expr/control.rs` | 101 | break/continue/return/throw/try/label |
| `expr/closure.rs` | 293 | Closures + trailing closures |
| `expr/mod.rs` | 1405 | Facade + recursion glue |

Verification: `cargo test -p kestrel-parser`

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
