# kestrel-syntax-tree Architecture

Lossless concrete syntax tree (CST) representation for the Kestrel language. Wraps the `rowan` library to provide a typed, immutable tree that preserves all source details including whitespace and comments.

## Pipeline Position

```
Source Text → Tokens → CST (rowan) → AST Build → Name Res → HIR → Type Infer
                        ^^^
                     this crate
```

The parser produces `SyntaxNode` trees using this crate's types. The AST builder walks these trees to extract declaration entities. CST subtrees are also stored in `CstNode` and `Valued` components for deferred expression processing.

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `SyntaxKind` | `lib.rs` | Enum with ~200 variants — all node and token types in the language |
| `SyntaxNode` | `lib.rs` | Type alias for `rowan::SyntaxNode<KestrelLanguage>` — a CST node |
| `SyntaxToken` | `lib.rs` | Type alias for `rowan::SyntaxToken<KestrelLanguage>` — a CST leaf |
| `SyntaxElement` | `lib.rs` | Enum wrapping either `SyntaxNode` or `SyntaxToken` |
| `KestrelLanguage` | `lib.rs` | Implements rowan's `Language` trait (SyntaxKind ↔ raw u16 conversion) |
| `GreenNodeBuilder` | re-export | Rowan's builder for constructing immutable green trees |

## SyntaxKind Categories

| Category | Examples | Count |
|----------|----------|-------|
| Declarations | `StructDeclaration`, `EnumDeclaration`, `FunctionDeclaration`, ... | ~20 |
| Expressions | `CallExpression`, `BinaryExpression`, `IfExpression`, ... | ~25 |
| Patterns | `BindingPattern`, `TuplePattern`, `EnumPattern`, ... | ~15 |
| Types | `PathType`, `TupleType`, `FunctionType`, `OptionalType`, ... | ~12 |
| Tokens | `Identifier`, `Integer`, `Plus`, `Func`, ... | ~70 |
| Trivia | `Whitespace`, `Newline`, `LineComment`, `BlockComment` | 4 |
| Generics | `TypeParameter`, `TypeParameterList`, `WhereClause`, ... | ~8 |
| Structure | `Name`, `ConformanceList`, `CodeBlock`, `Statement`, ... | ~15 |

## Lossless Design

The CST preserves all source text:

- **Whitespace and newlines** stored as trivia nodes
- **Comments** preserved between meaningful tokens
- **Every literal character** round-trips through the tree

This enables IDE features (formatting, refactoring) and accurate error recovery.

## Module Map

| File | Responsibility |
|------|---------------|
| `kinds.txt` | Every `SyntaxKind`, append-only, with token spellings (source of `src/generated/kinds.rs`) |
| `kestrel.ungram` | The tree's shape: one rule per node kind (source of the typed views and validator rules) |
| `lib.rs` | `KestrelLanguage`, kind predicates (`is_trivia`, `is_type`), rowan type aliases |
| `ast/` | Typed views: `AstNode`, `AstChildren`, `AstPtr` (a `Send` kind+range handle), generated node structs/enums, `ext.rs` conveniences (`HasName`, `HasVisibility`, … traits) |
| `validate/` | `validate()`: checks a tree against the grammar's rules (parser contract) |
| `utils.rs` | Span helpers over untyped declaration nodes (`get_decl_span`, `get_name_span`, `get_node_span`) |
| `tests/sourcegen.rs` | Generates and freshness-checks the three generated files |

## Dependencies

| Crate | Usage |
|-------|-------|
| `rowan` | Green/red tree infrastructure for the lossless CST |
| `kestrel-lexer` | `Token` enum — conversion from lexer tokens to `SyntaxKind` |
| `kestrel-span` | `Span` for source location tracking |
