# Kestrel Architecture

## Compilation Pipeline

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           COMPILATION PIPELINE                               │
└─────────────────────────────────────────────────────────────────────────────┘

Source Code ("module Main\nstruct Point { ... }")
       │
       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  PHASE 1: LEXING                                    [kestrel-lexer]         │
│  ────────────────                                                           │
│  Input:  Source string                                                      │
│  Output: Iterator<Spanned<Token>>                                           │
│  Library: Logos                                                             │
│                                                                             │
│  "module" → Token::Module                                                   │
│  "Main"   → Token::Identifier                                               │
│  "struct" → Token::Struct                                                   │
└─────────────────────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  PHASE 2: PARSING                                   [kestrel-parser]        │
│  ────────────────                                                           │
│  Input:  Tokens + Source                                                    │
│  Output: Events (StartNode, AddToken, FinishNode)                           │
│  Library: Chumsky                                                           │
│                                                                             │
│  Event-driven architecture:                                                 │
│    1. Internal Chumsky parser returns raw data (spans, tuples)              │
│    2. Emit functions convert data to events                                 │
│    3. Events collected in EventSink                                         │
└─────────────────────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  PHASE 3: SYNTAX TREE                               [kestrel-syntax-tree]   │
│  ────────────────────                                                       │
│  Input:  Events + Source                                                    │
│  Output: SyntaxNode (lossless CST)                                          │
│  Library: Rowan                                                             │
│                                                                             │
│  TreeBuilder converts events → GreenNode → SyntaxNode                       │
│  Preserves all source text (whitespace, comments, trivia)                   │
└─────────────────────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  PHASE 4: SEMANTIC ANALYSIS                                                 │
│  ──────────────────────────                                                 │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  4a. BUILD                           [kestrel-semantic-tree-builder]│    │
│  │  ─────────                                                          │    │
│  │  Builders extract symbols from syntax nodes                         │    │
│  │  Creates: ModuleSymbol, StructSymbol, FunctionSymbol, etc.          │    │
│  │  Attaches: Behaviors (Visibility, Callable, Typed)                  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                              │                                              │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  4b. BIND                             [kestrel-semantic-tree-binder]│    │
│  │  ────────                                                           │    │
│  │  Resolves type references to concrete types                         │    │
│  │  Validates imports, detects cycles                                  │    │
│  │  Body resolution for expressions/statements                         │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                              │                                              │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  4c. VALIDATE                      [kestrel-semantic-analyzers]     │    │
│  │  ───────────                                                        │    │
│  │  Validation passes check semantic constraints:                      │    │
│  │  - FunctionBodyPass: functions need bodies (except protocols)       │    │
│  │  - ProtocolMethodPass: protocol methods can't have bodies           │    │
│  │  - StaticContextPass: static only in struct/protocol                │    │
│  │  - DuplicateSymbolPass: no duplicate types/members                  │    │
│  │  - VisibilityConsistencyPass: public APIs consistency               │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  OUTPUT                                             [kestrel-compiler]      │
│  ──────                                                                     │
│  Compilation {                                                              │
│      semantic_model: SemanticModel, // Bound semantic model                 │
│      diagnostics: Vec<Diagnostic>,  // Errors and warnings                  │
│  }                                                                          │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Semantic Model Mutation Points

Kestrel keeps semantic analysis split into **BUILD**, **BIND**, and **VALIDATE** phases.
To preserve clean boundaries (and keep the query system free to become incremental later),
each phase has a clear “what it may mutate” rule:

- **BUILD (`kestrel-semantic-tree-builder`)**: creates symbols and the initial `SemanticModel`.
  - Allowed mutations: symbol creation, parent/child relationships, initial symbol metadata,
    `syntax_map` entries, and `sources` registration.
  - Not allowed: type/value resolution, cross-file binding, or any analysis that depends on
    already-resolved types/paths.

- **BIND (`kestrel-semantic-tree-binder`)**: resolves references and enriches the model.
  - Allowed mutations: attach/compute derived semantic information (e.g. types, callable
    signatures, executable bodies), populate resolution/registry structures, and link symbols
    across files/modules.
  - Not allowed: emitting new symbols that change the program surface area (those belong in BUILD).

- **VALIDATE (`kestrel-semantic-analyzers`)**: read-only checks over the bound model.
  - Allowed mutations: diagnostics accumulation only.
  - Not allowed: mutating the model or changing resolution results.

Guideline: if a component needs semantic information, prefer going through the `kestrel-semantic-model`
query layer rather than ad-hoc traversals, even if the query currently computes eagerly.

## Crate Dependencies

```
kestrel-compiler
  ├─ kestrel-semantic-tree-builder   (BUILD/lowering)
  ├─ kestrel-semantic-tree-binder    (BIND)
  ├─ kestrel-semantic-analyzers      (VALIDATE)
  ├─ kestrel-parser / kestrel-lexer / kestrel-syntax-tree
  ├─ kestrel-semantic-model / kestrel-semantic-tree / semantic-tree
  └─ kestrel-reporting / kestrel-span
```

## Key Types by Phase

### Phase 1: Lexing
```rust
// kestrel-lexer
Token              // Enum: Identifier, Module, Struct, LBrace, ...
Spanned<Token>     // Token + Span (Range<usize>)

// Usage
let tokens: Vec<Spanned<Token>> = lex(source).filter_map(|t| t.ok()).collect();
```

### Phase 2: Parsing
```rust
// kestrel-parser
Event              // StartNode(SyntaxKind), AddToken(kind, span), FinishNode, Error
EventSink          // Collects events during parsing
TreeBuilder        // Converts events to SyntaxNode

// Pattern: internal parser → emit → public parse function
fn foo_parser_internal() -> impl Parser<Token, RawData, ...>
fn emit_foo(sink: &mut EventSink, data: RawData)
pub fn parse_foo(source: &str, tokens: I, sink: &mut EventSink)
```

### Phase 3: Syntax Tree
```rust
// kestrel-syntax-tree
SyntaxKind         // Enum: tokens (Identifier, Module) + nodes (StructDeclaration, Name)
SyntaxNode         // Rowan node with children
SyntaxToken        // Rowan token with text

// Tree structure
StructDeclaration
├── Visibility      // Wrapper (may be empty)
│   └── Public      // Token (optional)
├── Struct          // Token
├── Name            // Wrapper (always has content)
│   └── Identifier  // Token
└── StructBody
    └── ...
```

### Phase 4: Semantic Analysis
```rust
// kestrel-semantic-tree
Symbol<KestrelLanguage>     // Trait for all symbols
SymbolMetadata              // Name, span, children, behaviors
KestrelSymbolKind           // Module, Struct, Function, Field, ...

// Specific symbols
ModuleSymbol, StructSymbol, FunctionSymbol, FieldSymbol,
ProtocolSymbol, TypeAliasSymbol, ImportSymbol, LocalSymbol

// Behaviors (attached to symbols)
VisibilityBehavior          // Access control
CallableBehavior            // Function signatures
TypedBehavior               // Type information
ExecutableBehavior          // Code bodies

// kestrel-semantic-tree-builder (BUILD)
Builder                     // Trait: builds symbol from syntax
SemanticModelBuilder         // Lowers syntax trees to a SemanticModel

// kestrel-semantic-tree-binder (BIND)
DeclarationBinder           // Trait: binds a symbol using its syntax node
DeclarationBinderRegistry   // Maps SyntaxKind → DeclarationBinder
TypeResolver                // Resolves types (during binding/body resolution)
BodyResolver                // Resolves expressions/statements
```

## File Organization

```
lib/kestrel-lexer/
└── src/lib.rs              # Single file: Token enum + lex()

lib/kestrel-parser/
└── src/
    ├── lib.rs              # Re-exports
    ├── event.rs            # Event, EventSink, TreeBuilder
    ├── parser.rs           # High-level Parser API
    ├── declaration_item/   # Top-level declarations
    │   └── mod.rs
    ├── module/             # Module-specific parsing
    │   └── mod.rs
    ├── struct/             # Struct-specific parsing
    │   └── mod.rs
    ├── function/           # Function-specific parsing
    │   └── mod.rs
    ├── expr/               # Expression parsing
    │   └── mod.rs
    └── common/             # Shared parser utilities
        ├── data.rs
        ├── emitters.rs
        └── parsers.rs

lib/kestrel-syntax-tree/
└── src/lib.rs              # SyntaxKind enum + KestrelLanguage

lib/kestrel-semantic-tree/
└── src/
    ├── lib.rs              # Re-exports
    ├── language.rs         # KestrelLanguage definition
    ├── symbol/
    │   ├── mod.rs
    │   ├── kind.rs         # KestrelSymbolKind enum
    │   ├── module.rs       # ModuleSymbol
    │   ├── struct.rs       # StructSymbol
    │   ├── function.rs     # FunctionSymbol
    │   └── ...
    ├── behavior/
    │   ├── mod.rs
    │   ├── visibility.rs
    │   ├── callable.rs
    │   └── ...
    ├── ty/                 # Type system
    │   └── mod.rs
    ├── expr.rs             # Expression semantics
    └── stmt.rs             # Statement semantics

lib/kestrel-semantic-tree-builder/
└── src/
    ├── lib.rs              # Public API: build(...), SemanticModelBuilder
    ├── lowerer.rs          # SyntaxTree -> SemanticModel lowering driver
    ├── builder.rs          # Builder trait
    ├── builders/
    │   ├── mod.rs
    │   ├── module.rs
    │   ├── struct.rs
    │   ├── function.rs
    │   └── ...

lib/kestrel-semantic-tree-binder/
└── src/
    ├── lib.rs              # Public API: SemanticBinder
    ├── declaration_binder.rs# DeclarationBinder + registry
    ├── binders/            # Per-declaration binding
    ├── resolution/         # Binder orchestration + type resolution
    ├── body_resolver/      # Expression/statement resolution
    └── diagnostics/        # Bind-time diagnostics

lib/kestrel-semantic-analyzers/
└── src/                    # Post-bind analyzers (VALIDATE)

lib/kestrel-test-suite/
└── src/lib.rs              # Test fluent API
└── tests/
    ├── body_resolution.rs  # Expression/statement tests
    ├── functions.rs
    ├── structs.rs
    ├── protocols.rs
    └── ...
```

## Data Flow Example

Adding `5.toString()` (primitive method call):

```
1. LEXER
   "5.toString()" → [Integer(5), Dot, Identifier(toString), LParen, RParen]

2. PARSER
   Internal parser extracts: receiver_span, dot_span, method_span, args_spans
   Emitter produces events:
     StartNode(MethodCallExpr)
       AddToken(Integer, 0..1)
       AddToken(Dot, 1..2)
       AddToken(Identifier, 2..10)
       AddToken(LParen, 10..11)
       AddToken(RParen, 11..12)
     FinishNode

3. SYNTAX TREE
   MethodCallExpr
   ├── Integer "5"
   ├── Dot "."
   ├── Identifier "toString"
   ├── LParen "("
   └── RParen ")"

4. SEMANTIC ANALYSIS (body_resolver.rs)
   - Recognize Integer literal → Expr::Integer(5)
   - Look up "toString" method on Int type from prelude
   - Resolve to: Expr::MethodCall { receiver: Int, method: toString, args: [] }
   - Return type: String
```
