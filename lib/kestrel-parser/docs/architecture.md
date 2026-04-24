# kestrel-parser Architecture

`kestrel-parser` is the syntax parser for the Kestrel language. It converts a
token stream into a concrete syntax tree (CST) using Chumsky parser combinators
and an event-driven tree-building layer inspired by rust-analyzer.

This document describes the target contract for the crate as the parser is
reworked. Some implementation details still reflect the current transitional
architecture and are called out below.

## Pipeline Position

```
Source Text → Tokens → Parser → CST (rowan) → AST Build → Name Res → HIR → Type Infer
                        ^^^
                     this crate
```

## Target Contract

The parser owns syntax recognition only. It should:

- accept lexer tokens and source text for one file
- emit a concrete syntax tree with stable `SyntaxKind` nodes and tokens
- preserve source token order and source spans
- report syntax errors with useful spans and recovery where practical
- avoid semantic decisions that belong to AST build, name resolution, HIR, type
  inference, or later lowering passes

Downstream crates may depend on CST shape and parser diagnostics, but should not
depend on parser-internal Chumsky combinators or temporary parse-data structs.

### Trivia

The target CST contract is lossless with respect to source text. Whitespace,
newlines, line comments, and block comments should be preserved as trivia tokens
with their distinct token kinds.

Current limitation: `TreeBuilder` currently inserts skipped trivia as a single
`Whitespace` token before emitted syntax tokens, and does not explicitly model
trailing trivia after the final emitted token. Reworking trivia preservation is a
planned architecture step.

### Operators

The parser intentionally does not own operator precedence or associativity. It
recognizes operator tokens and preserves expression/operator order in syntax.
Operator binding is handled later by the Pratt parser in the downstream
pipeline, which leaves room for operators to be defined by code in the future.

Parser tests for operators should assert syntax preservation, not semantic
grouping.

## Two-Phase Architecture

Parsing and tree building are separate concerns:

```
Tokens → Chumsky Parsers → Data Structs → Emitters → Events → TreeBuilder → SyntaxNode
         ^^^^^^^^^^^^^^^^                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
         Phase 1: parse                    Phase 2: emit + build
```

**Phase 1 — Parsing**: Chumsky parser combinators match tokens and currently
produce intermediate data structures (e.g., `FunctionDeclarationData`,
`ExprVariant`).

**Phase 2 — Emission & Tree Building**: Emitter functions walk data structures
and push events (`StartNode`, `AddToken`, `FinishNode`, `Error`) into an
`EventSink`. The `TreeBuilder` then consumes these events with the original
source text, inserts trivia, and builds rowan green trees.

Current limitation: the data-then-emit split creates multiple representations of
the same grammar. A future rework should either make the intermediate data layer
small and local to each parser module, or remove it where direct event/CST
construction is clearer.

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `Event` | `event.rs` | Four variants: `StartNode(SyntaxKind)`, `AddToken(SyntaxKind, Span)`, `FinishNode`, `Error` |
| `EventSink` | `event.rs` | Collects events during parsing |
| `TreeBuilder` | `event.rs` | Consumes events + source text → `SyntaxNode` |
| `Parser` | `parser.rs` | High-level wrapper: creates sink, runs parse, extracts errors, builds tree |
| `ParseResult` | `parser.rs` | Result type with tree + accumulated errors |
| `ParseError` | `parser.rs` | User-friendly error with message, span, and fix suggestions |

## Error Recovery

The target parser should continue after syntax errors:

- Rich errors from chumsky track expectations for helpful messages
- The tree builder produces a valid CST even with `Error` events interspersed
- `suggest_fix()` provides context-aware fix suggestions

Current limitation: recovery is mostly implicit through list parsing and parser
failure behavior. Planned recovery work should add explicit recovery anchors for
declaration starters, `}`, semicolons, and other syntax boundaries.

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | Public API, convenience `*_from_source()` functions |
| `parser.rs` | `Parser` struct, `ParseError`, user-friendly formatting |
| `event.rs` | `Event`, `EventSink`, `TreeBuilder` (event → CST conversion) |
| `input.rs` | Chumsky integration: `ParserInput`, `ParserExtra` type aliases |
| `common/parsers.rs` | Shared parser helpers and currently some declaration parsers |
| `common/emitters.rs` | Shared emitters and currently many declaration emitters |
| `common/data.rs` | Shared parse data and currently many declaration data structs |
| `declaration_item/` | Router dispatching to declaration-specific parsers |
| `type_decl.rs` | Unified struct/enum body parser (handles mutual recursion) |

Target direction: `common` should become small and boring. Declaration modules
should own their data structs, parsers, emitters, and CST wrappers. `common`
should keep only reusable syntax fragments such as token helpers, identifiers,
trivia, visibility, and separated-list utilities.

### Declaration modules

Each follows the pattern: data type + parser + emitter.

| Module | Parses |
|--------|--------|
| `module/` | `module A.B.C` declarations |
| `import/` | `import A.B.C.(X, Y)` with aliases and wildcards |
| `struct/`, `enum_decl/` | Type declarations (delegates to `type_decl.rs`) |
| `protocol/` | Protocol declarations with associated types |
| `extension/` | Extend declarations |
| `function/` | Function declarations with generics |
| `field/` | Field declarations (var/let, computed properties) |
| `subscript/` | Subscript declarations |
| `type_alias/` | Type aliases and associated type bindings |

### Expression and type modules

| Module | Parses |
|--------|--------|
| `expr/` | Expressions: literals, calls, operators, closures, control flow |
| `ty/` | Type expressions: paths, tuples, functions, arrays, generics |
| `pattern/` | Patterns: wildcards, bindings, tuples, enums, structs |
| `block/` | Code blocks: statements + trailing expression |
| `stmt/` | Statements: let/var bindings |
| `attribute/` | Attributes: `@name` or `@name(args)` |
| `type_param/` | Type parameters, where clauses, bounds |

## Dependencies

| Crate | Usage |
|-------|-------|
| `chumsky` | Parser combinator framework |
| `stacker` | Stack growth for deeply nested types |
| `kestrel-lexer` | `Token` enum as parser input |
| `kestrel-syntax-tree` | `SyntaxKind`, `SyntaxNode`, `GreenNodeBuilder` |
| `kestrel-span` | `Span` for source locations |
