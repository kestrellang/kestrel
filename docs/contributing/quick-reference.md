# Quick Reference

## Key File Locations

| Task | File Path |
|------|-----------|
| Add token/keyword | `lib/kestrel-lexer/src/lib.rs` |
| Add syntax node kind | `lib/kestrel-syntax-tree/src/lib.rs` |
| Add parser for feature | `lib/kestrel-parser/src/{feature}/mod.rs` |
| Add to declaration items | `lib/kestrel-parser/src/declaration_item/mod.rs` |
| Shared parser utilities | `lib/kestrel-parser/src/common/` |
| Expression parsing | `lib/kestrel-parser/src/expr/mod.rs` |
| Statement parsing | `lib/kestrel-parser/src/stmt/mod.rs` |
| Add semantic symbol | `lib/kestrel-semantic-tree/src/symbol/{name}.rs` |
| Add symbol kind | `lib/kestrel-semantic-tree/src/symbol/kind.rs` |
| Add behavior | `lib/kestrel-semantic-tree/src/behavior/{name}.rs` |
| Add builder (BUILD) | `lib/kestrel-semantic-tree-builder/src/builders/{name}.rs` |
| Register builder (BUILD) | `lib/kestrel-semantic-tree-builder/src/lowerer.rs` |
| Add binder (BIND) | `lib/kestrel-semantic-tree-binder/src/binders/{name}.rs` |
| Register binder (BIND) | `lib/kestrel-semantic-tree-binder/src/declaration_binder.rs` |
| Body resolution (BIND) | `lib/kestrel-semantic-tree-binder/src/body_resolver/mod.rs` |
| Type resolution (BIND) | `lib/kestrel-semantic-tree-binder/src/resolution/type_resolver.rs` |
| Add analyzer (VALIDATE) | `lib/kestrel-semantic-analyzers/src/analyzers/{name}/mod.rs` |
| Register analyzer (VALIDATE) | `lib/kestrel-semantic-analyzers/src/lib.rs` |
| Primitive types | `lib/kestrel-prelude/src/lib.rs` |
| Add integration test | `lib/kestrel-test-suite/tests/{name}.rs` |
| Test utilities | `lib/kestrel-test-suite/src/lib.rs` |

## Common Imports

### Lexer (`kestrel-lexer`)
```rust
use logos::Logos;
use kestrel_span::{Span, Spanned};
```

### Parser (`kestrel-parser`)
```rust
use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxKind;
use crate::event::{Event, EventSink, TreeBuilder};
```

### Syntax Tree (`kestrel-syntax-tree`)
```rust
use rowan::{GreenNode, GreenNodeBuilder, Language, SyntaxNode as RowanSyntaxNode};
```

### Semantic Tree (`kestrel-semantic-tree`)
```rust
use std::sync::Arc;
use kestrel_span::{Name, Span, Spanned};
use semantic_tree::symbol::{Symbol, SymbolMetadata, SymbolMetadataBuilder};
use crate::behavior::{KestrelBehavior, KestrelBehaviorKind};
use crate::language::KestrelLanguage;
use crate::symbol::kind::KestrelSymbolKind;
```

### Semantic Tree Builder (BUILD) (`kestrel-semantic-tree-builder`)
```rust
use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_tree_builder::builder::Builder;
use kestrel_syntax_tree::SyntaxNode;
```

### Semantic Tree Binder (BIND) (`kestrel-semantic-tree-binder`)
```rust
use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_model::SemanticModel;
use kestrel_semantic_tree_binder::SemanticBinder;
```

### Semantic Analyzers (VALIDATE) (`kestrel-semantic-analyzers`)
```rust
use kestrel_semantic_analyzers::{AnalysisContext, Analyzer, default_analyzers, run_all};
```

### Tests (`kestrel-test-suite`)
```rust
use kestrel_test_suite::{Test, Compiles, HasError, Symbol, SymbolKind, Behavior, Visibility};
```

## Token Categories (in order)

```rust
pub enum Token {
    // 1. Literals
    Identifier, String, Integer, Float, Boolean,

    // 2. Declaration Keywords
    Fn, Import, Let, Module, Protocol, Struct, Type, Var, ...

    // 3. Visibility Keywords
    Public, Private, Internal, Fileprivate,

    // 4. Statement Keywords
    Else, For, If, Match, Return, While, ...

    // 5. Modifier Keywords
    Consuming, Mutating, Self_, Static, Where, ...

    // 6. Braces
    LParen, RParen, LBrace, RBrace, LBracket, RBracket, LAngle, RAngle,

    // 7. Punctuation
    Semicolon, Comma, Dot, Colon, DoubleColon, Arrow, FatArrow,

    // 8. Operators
    Equals, Plus, Minus, Star, Slash, ...
}
```

## SyntaxKind Categories

```rust
pub enum SyntaxKind {
    // === TOKENS (map 1:1 from Token) ===
    Identifier, String, Integer, Float, Boolean,
    Fn, Module, Struct, Protocol, ...
    LParen, RParen, LBrace, RBrace, ...

    // === SYNTAX NODES (non-terminals) ===
    // Top-level
    Root, SourceFile, DeclarationItem,

    // Declarations
    ModuleDeclaration, ModulePath,
    StructDeclaration, StructBody,
    ProtocolDeclaration, ProtocolBody,
    FunctionDeclaration, ParameterList, Parameter,
    FieldDeclaration,
    ImportDeclaration,
    TypeAliasDeclaration,

    // Wrapper nodes (used for uniform extraction)
    Name,           // Wraps identifier for declarations
    Visibility,     // Wraps visibility token (may be empty)
    TypeAnnotation, // Wraps type expression
    ReturnType,     // Wraps return type

    // Expressions
    LiteralExpr, GroupingExpr, TupleExpr, ArrayExpr,
    IdentifierExpr, CallExpr, MethodCallExpr,
    FieldAccessExpr, BinaryExpr, UnaryExpr,

    // Statements
    ExpressionStmt, LetStmt, VarStmt, ReturnStmt,

    // Types
    TypePath, TypeArguments,
    FunctionType, TupleType, ArrayType,

    // Misc
    Block, Arguments, WhereClause, TypeParameter,
}
```

## Test API Cheat Sheet

```rust
// Basic compilation test
Test::new("module Main\nstruct Point { }")
    .expect(Compiles);

// Expect error
Test::new("module Main\nfn f() { }")  // no body
    .expect(HasError("function 'f' must have a body"));

// Check symbol exists
Test::new("module Main\npublic struct Point { }")
    .expect(Compiles)
    .expect(Symbol::new("Point").is(SymbolKind::Struct));

// Check symbol has behavior
Test::new("module Main\npublic struct Point { }")
    .expect(Compiles)
    .expect(Symbol::new("Point").has(Behavior::Visibility(Visibility::Public)));

// Multi-file test
Test::with_files(&[
    ("main.ks", "module Main\nimport Other"),
    ("other.ks", "module Other\npublic struct Foo { }"),
])
.expect(Compiles);
```

## Commands

```bash
# Run all tests
cargo test

# Run specific crate tests
cargo test -p kestrel-lexer
cargo test -p kestrel-parser
cargo test -p kestrel-syntax-tree
cargo test -p kestrel-semantic-tree
cargo test -p kestrel-semantic-tree-builder
cargo test -p kestrel-semantic-tree-binder
cargo test -p kestrel-semantic-analyzers
cargo test -p kestrel-test-suite

# Run specific test file
cargo test -p kestrel-test-suite --test body_resolution
cargo test -p kestrel-test-suite --test functions

# Run specific test by name
cargo test -p kestrel-test-suite call_instance_method

# Run main CLI
cargo run

# Check compilation
cargo check

# Format code
cargo fmt

# Lint
cargo clippy
```

## Symbol Kind Reference

| Kind | Description | Parent Types |
|------|-------------|--------------|
| `SourceFile` | File in compilation | Root |
| `Module` | Module declaration | SourceFile |
| `Struct` | Struct type | SourceFile, Struct |
| `Protocol` | Protocol type | SourceFile |
| `Function` | Function/method | SourceFile, Struct, Protocol |
| `Field` | Field/property | Struct |
| `TypeAlias` | Type alias | SourceFile |
| `Import` | Import statement | SourceFile |
| `TypeParameter` | Generic param | Struct, Function, Protocol, TypeAlias |
| `Local` | Local variable | Function body |

## Behavior Reference

| Behavior | Attached To | Purpose |
|----------|-------------|---------|
| `VisibilityBehavior` | All declarations | Access control (public/private/etc) |
| `CallableBehavior` | Functions | Signature, parameters, return type |
| `TypedBehavior` | Fields, locals | Type information |
| `ExecutableBehavior` | Functions | Body expressions |
| `MemberAccessBehavior` | Fields | Field access info |
| `ConformancesBehavior` | Structs | Protocol conformances |
| `ValueBehavior` | Fields, locals | Value category (let/var) |
