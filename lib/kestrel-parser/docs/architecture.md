# kestrel-parser Architecture

Recursive descent parser for the Kestrel language. Converts a token stream into a lossless CST using an event-driven architecture inspired by rust-analyzer: parsers emit events through a sink, which a tree builder converts into rowan `SyntaxNode` trees.

## Pipeline Position

```
Source Text → Tokens → Parser → CST (rowan) → AST Build → Name Res → HIR → Type Infer
                        ^^^
                     this crate
```

## Two-Phase Architecture

Parsing and tree building are separate concerns:

```
Tokens → Chumsky Parsers → Data Structs → Emitters → Events → TreeBuilder → SyntaxNode
         ^^^^^^^^^^^^^^^^                  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
         Phase 1: parse                    Phase 2: emit + build
```

**Phase 1 — Parsing**: Chumsky parser combinators match tokens and produce intermediate data structures (e.g., `FunctionDeclarationData`, `ExprVariant`).

**Phase 2 — Emission & Tree Building**: Emitter functions walk data structures and push events (`StartNode`, `AddToken`, `FinishNode`, `Error`) into an `EventSink`. The `TreeBuilder` then consumes these events with the original source text, inserts trivia (whitespace/comments), and builds rowan green trees.

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `Event` | `event.rs` | Four variants: `StartNode(SyntaxKind)`, `AddToken(SyntaxKind, Span)`, `FinishNode`, `Error` |
| `EventSink` | `event.rs` | Collects events during parsing |
| `TreeBuilder` | `event.rs` | Consumes events + source text → `SyntaxNode` (inserts trivia) |
| `Parser` | `parser.rs` | High-level wrapper: creates sink, runs parse, extracts errors, builds tree |
| `ParseResult` | `parser.rs` | Result type with tree + accumulated errors |
| `ParseError` | `parser.rs` | User-friendly error with message, span, and fix suggestions |

## Error Recovery

The parser continues after syntax errors:

- `.repeated()` on lists allows recovery after malformed items
- Rich errors from chumsky track expectations for helpful messages
- The tree builder produces a valid CST even with `Error` events interspersed
- `suggest_fix()` provides context-aware fix suggestions

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | Public API, convenience `*_from_source()` functions |
| `parser.rs` | `Parser` struct, `ParseError`, user-friendly formatting |
| `event.rs` | `Event`, `EventSink`, `TreeBuilder` (event → CST conversion) |
| `input.rs` | Chumsky integration: `ParserInput`, `ParserExtra` type aliases |
| `common/parsers.rs` | Shared parsers: module paths, visibility, identifiers, type params, where clauses |
| `common/emitters.rs` | Shared emitters: attributes, type parameters, declarations |
| `common/data.rs` | Shared data types: `ParameterData`, `FunctionDeclarationData`, etc. |
| `declaration_item/` | Router dispatching to declaration-specific parsers |
| `type_decl.rs` | Unified struct/enum body parser (handles mutual recursion) |

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
