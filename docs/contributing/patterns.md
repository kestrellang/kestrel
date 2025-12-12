# Kestrel Code Patterns

This document catalogs the conventions and patterns used throughout the Kestrel codebase.

## Naming Conventions

### Token Names (`kestrel-lexer`)
```rust
// Keywords: PascalCase matching the keyword
Module, Struct, Fn, Let, Var, If, Else, Return

// Visibility: Full word
Public, Private, Internal, Fileprivate

// Modifiers: Full word (Self_ has underscore to avoid Rust keyword)
Static, Mutating, Consuming, Self_

// Literals: Type name
Identifier, String, Integer, Float, Boolean

// Braces: L/R + Name
LParen, RParen      // ( )
LBrace, RBrace      // { }
LBracket, RBracket  // [ ]
LAngle, RAngle      // < >

// Punctuation: Descriptive
Dot, Comma, Colon, Semicolon, Arrow, FatArrow, DoubleColon

// Operators: Descriptive
Plus, Minus, Star, Slash, Equals, Bang
```

### SyntaxKind Names (`kestrel-syntax-tree`)
```rust
// Declarations: {Feature}Declaration
ModuleDeclaration, StructDeclaration, FunctionDeclaration, FieldDeclaration

// Bodies: {Feature}Body
StructBody, ProtocolBody

// Expressions: {Type}Expr
LiteralExpr, CallExpr, MethodCallExpr, FieldAccessExpr, BinaryExpr

// Statements: {Type}Stmt
ExpressionStmt, LetStmt, VarStmt, ReturnStmt

// Wrapper nodes: Descriptive
Name, Visibility, TypeAnnotation, ReturnType

// Lists: {Item}List or plural
ParameterList, Arguments, TypeArguments

// Types: {Type}Type
FunctionType, TupleType, ArrayType
```

### Parser Functions (`kestrel-parser`)
```rust
// Internal Chumsky parsers: {feature}_parser_internal()
fn module_declaration_parser_internal() -> impl Parser<...>
fn visibility_parser_internal() -> impl Parser<...>

// Event emitters: emit_{feature}()
fn emit_module_declaration(sink: &mut EventSink, ...)
fn emit_visibility(sink: &mut EventSink, ...)

// Public parsers: parse_{feature}()
pub fn parse_module_declaration(source: &str, tokens: I, sink: &mut EventSink)
pub fn parse_source_file(source: &str, tokens: I, sink: &mut EventSink)
```

### Semantic Types (`kestrel-semantic-tree`)
```rust
// Symbols: {Feature}Symbol
ModuleSymbol, StructSymbol, FunctionSymbol, FieldSymbol, LocalSymbol

// Behaviors: {Concern}Behavior
VisibilityBehavior, CallableBehavior, TypedBehavior, ExecutableBehavior
```

### Builders (`kestrel-semantic-tree-builder`)
```rust
// Builders: {Feature}Builder
ModuleBuilder, StructBuilder, FunctionBuilder, FieldBuilder
```

### Binders (`kestrel-semantic-tree-binder`)
```rust
// Binders: {Feature}Binder
ModuleBinder, StructBinder, FunctionBinder, FieldBinder
```

## Event-Driven Parser Pattern

The parser uses a three-layer architecture:

```rust
// Layer 1: Internal Chumsky parser - returns raw data
fn foo_parser_internal() -> impl Parser<Token, FooData, Error = Simple<Token>> + Clone {
    just(Token::Foo)
        .map_with_span(|_, span| span)
        .then(identifier_parser_internal())
        .map(|(keyword_span, name_span)| FooData { keyword_span, name_span })
}

// Layer 2: Event emitter - converts data to events
fn emit_foo(sink: &mut EventSink, data: &FooData) {
    sink.start_node(SyntaxKind::FooDeclaration);

    // Visibility (always emit, even if empty)
    sink.start_node(SyntaxKind::Visibility);
    sink.finish_node();

    sink.add_token(SyntaxKind::Foo, data.keyword_span.clone());

    // Name wrapper
    sink.start_node(SyntaxKind::Name);
    sink.add_token(SyntaxKind::Identifier, data.name_span.clone());
    sink.finish_node();

    sink.finish_node();
}

// Layer 3: Public parser - coordinates everything
pub fn parse_foo<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let end_pos = source.len();
    let stream = chumsky::Stream::from_iter(end_pos..end_pos, tokens);

    match foo_parser_internal().parse(stream) {
        Ok(data) => emit_foo(sink, &data),
        Err(errors) => {
            for error in errors {
                sink.error(format!("Parse error: {:?}", error));
            }
        }
    }
}
```

## Wrapper Node Pattern

Wrapper nodes provide uniform extraction across different declaration types.

### Name Wrapper
**Always required. Always has content.**

```rust
// Emit
sink.start_node(SyntaxKind::Name);
sink.add_token(SyntaxKind::Identifier, name_span);
sink.finish_node();

// Extract
pub fn extract_name(syntax: &SyntaxNode) -> Option<String> {
    let name_node = find_child(syntax, SyntaxKind::Name)?;
    name_node
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .find(|tok| tok.kind() == SyntaxKind::Identifier)
        .map(|tok| tok.text().to_string())
}
```

### Visibility Wrapper
**Always emitted, but may be empty if no modifier.**

```rust
// Emit - ALWAYS emit the wrapper, content is optional
sink.start_node(SyntaxKind::Visibility);
if let Some((vis_token, vis_span)) = visibility {
    let vis_kind = match vis_token {
        Token::Public => SyntaxKind::Public,
        Token::Private => SyntaxKind::Private,
        Token::Internal => SyntaxKind::Internal,
        Token::Fileprivate => SyntaxKind::Fileprivate,
        _ => unreachable!(),
    };
    sink.add_token(vis_kind, vis_span);
}
sink.finish_node(); // May be empty!

// Extract - returns None if wrapper is empty
pub fn extract_visibility(syntax: &SyntaxNode) -> Option<String> {
    let visibility_node = find_child(syntax, SyntaxKind::Visibility)?;
    visibility_node
        .children_with_tokens()
        .filter_map(|elem| elem.into_token())
        .next()
        .and_then(|tok| match tok.kind() {
            SyntaxKind::Public => Some("public"),
            SyntaxKind::Private => Some("private"),
            SyntaxKind::Internal => Some("internal"),
            SyntaxKind::Fileprivate => Some("fileprivate"),
            _ => None,
        })
        .map(String::from)
}
```

## Symbol Creation Pattern

```rust
impl Builder for FooBuilder {
    fn build_declaration(
        &self,
        syntax: &SyntaxNode,
        source: &str,
        parent: Option<&Arc<dyn Symbol<KestrelLanguage>>>,
        root: &Arc<dyn Symbol<KestrelLanguage>>,
    ) -> Option<Arc<dyn Symbol<KestrelLanguage>>> {
        // 1. Extract name
        let name_str = extract_name(syntax)?;
        let name_node = find_child(syntax, SyntaxKind::Name)?;
        let name_span = get_node_span(&name_node, source);
        let name = Spanned::new(name_str, name_span.clone());

        // 2. Get declaration span
        let full_span = get_node_span(syntax, source);

        // 3. Extract visibility
        let visibility_str = extract_visibility(syntax);
        let visibility_enum = visibility_str.as_deref().and_then(parse_visibility);
        let visibility_span = get_visibility_span(syntax, source).unwrap_or(name_span);
        let visibility_scope = find_visibility_scope(visibility_enum.as_ref(), parent, root);

        // 4. Create behavior
        let visibility_behavior = VisibilityBehavior::new(
            visibility_enum,
            visibility_span,
            visibility_scope,
        );

        // 5. Create symbol
        let symbol = FooSymbol::new(name, full_span, visibility_behavior);
        let symbol_arc: Arc<dyn Symbol<KestrelLanguage>> = Arc::new(symbol);

        // 6. Add to parent
        if let Some(parent) = parent {
            parent.metadata().add_child(&symbol_arc);
        }

        Some(symbol_arc)
    }
}
```

## Symbol Metadata Builder Pattern

```rust
let metadata = SymbolMetadataBuilder::new(KestrelSymbolKind::Function)
    .with_name(name.clone())
    .with_declaration_span(name.span.clone())
    .with_span(full_span)
    .with_behavior(Arc::new(visibility_behavior))
    .with_behavior(Arc::new(callable_behavior))
    .build();

FunctionSymbol { metadata }
```

## Diagnostic Reporting Pattern

```rust
fn report_error(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    diagnostics: &mut DiagnosticContext,
    config: &ValidationConfig,
) {
    let name = &symbol.metadata().name().value;
    let span = symbol.metadata().declaration_span().clone();

    let message = if config.debug_mode {
        format!("[{}] error message: '{}'", Self::NAME, name)
    } else {
        format!("error message: '{}'", name)
    };

    let diagnostic = kestrel_reporting::Diagnostic::error()
        .with_message(message)
        .with_labels(vec![
            kestrel_reporting::Label::primary(span.file_id, span.range())
                .with_message("additional context")
        ]);

    diagnostics.add_diagnostic(diagnostic);
}
```

## Analyzer Pattern

```rust
pub struct MyAnalyzer;

impl Analyzer for MyAnalyzer {
    fn name(&self) -> &'static str {
        "my_analyzer"
    }

    fn analyze(&mut self, model: &SemanticModel, ctx: &mut AnalysisContext) {
        // Walk `model.root()` and inspect symbols/behaviors.
        // Report diagnostics via `ctx.diagnostics`.
        let _ = (model, ctx);
    }
}
```

## Test Pattern

```rust
mod my_feature {
    use super::*;

    mod basic {
        use super::*;

        #[test]
        fn simple_case() {
            Test::new(r#"module Main
                struct Point { }
            "#)
            .expect(Compiles)
            .expect(Symbol::new("Point").is(SymbolKind::Struct));
        }
    }

    mod errors {
        use super::*;

        #[test]
        fn invalid_case() {
            Test::new(r#"module Main
                // invalid code
            "#)
            .expect(HasError("expected error substring"));
        }
    }

    mod visibility {
        use super::*;

        #[test]
        fn public_modifier() {
            Test::new(r#"module Main
                public struct Point { }
            "#)
            .expect(Compiles)
            .expect(Symbol::new("Point").has(Behavior::Visibility(Visibility::Public)));
        }
    }
}
```

## File Organization Pattern

Each parser feature gets its own directory:
```
lib/kestrel-parser/src/{feature}/
├── mod.rs      # Main declaration struct and public parser
├── path.rs     # Sub-types (if needed)
└── ...
```

Each build-phase builder gets its own file:
```
lib/kestrel-semantic-tree-builder/src/builders/{feature}.rs
```

Update mod.rs to export:
```rust
mod feature;
pub use feature::FeatureBuilder;
```

Each bind-phase binder gets its own file:
```
lib/kestrel-semantic-tree-binder/src/binders/{feature}.rs
```

Update `binders/mod.rs` to export:
```rust
mod feature;
pub use feature::FeatureBinder;
```

## Common Pitfalls

1. **Missing wrapper node** - Always emit `Name` and `Visibility` nodes
2. **Empty visibility not emitted** - Visibility node must exist even when empty
3. **Forgot to update kind_from_raw** - After adding SyntaxKind, update the match
4. **Forgot to register builder** - Update `builder_for(...)` in `lib/kestrel-semantic-tree-builder/src/lowerer.rs`
5. **Forgot to register binder** - Add to `DeclarationBinderRegistry::new()` in `lib/kestrel-semantic-tree-binder/src/declaration_binder.rs`
6. **Forgot to add to declaration_item parser** - Features won't parse in files
7. **Wrong event order** - start_node/finish_node must be balanced
8. **Storing data instead of syntax** - AST structs should wrap SyntaxNode
