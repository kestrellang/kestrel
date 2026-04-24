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

## Remaining Plan

### Step 4g (skipped): Initializer / Deinitializer

No separate `initializer/` or `deinit/` directory exists and these declarations
are never surfaced as top-level `DeclarationItem` variants. Their data types
(`InitializerDeclarationData`, `DeinitDeclarationData`), parsers, and emitters
remain in `common`. Revisit only if their ownership becomes painful.

### Step 5: Introduce A Shared Parse Context

Centralize repeated parser entry boilerplate:

- token preparation
- input creation
- parse error forwarding
- file id handling
- parse-to-events pattern

This should remove repetitive `prepare_tokens/create_input/match parse` code
from declaration modules.

### Step 6: Make Trivia Policy Explicit In Code

Implement the documented target:

- preserve whitespace, newline, line comment, and block comment trivia as
  distinct token kinds
- preserve trailing trivia after the last emitted syntax token
- add tests around source text round-tripping and trivia kinds

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

### Step 11: Narrow Public API

Revisit `lib.rs` exports.

Likely public surface:

- source-file parse entry point
- specific parse entry points used by tests/tools
- CST wrapper types

Parser combinators and temporary parse data should stay crate-private.

### Step 12: Improve Error Recovery Deliberately

Add explicit recovery anchors for:

- declaration starters
- `}`
- semicolons
- other syntax boundaries where recovery is useful

Avoid relying on list parsing and parser failure order as implicit recovery.

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
