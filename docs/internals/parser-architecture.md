# Parser Architecture: Chumsky + Rowan Integration

## Current Approach (Two-Stage)

Currently, the parser works in two stages:

```rust
// Stage 1: Parse to get spans
let (module_span, path_segments) = module_declaration_parser().parse(tokens)?;

// Stage 2: Build syntax tree from spans
let decl = parse_module_declaration(source, module_span, path_segments, full_span);
```

**Pros:**
- Simple and clear separation
- Easy to understand
- Works well for small parsers

**Cons:**
- Two-pass approach (less efficient)
- Requires manual synchronization between parser and tree builder
- Can't handle complex recursive structures easily

## Better Approach: Integrated Tree Building

### Option 1: Parser Wrapper Pattern

Create a wrapper that builds trees during parsing:

```rust
struct TreeBuilder<'src> {
    source: &'src str,
    builder: GreenNodeBuilder<'static>,
}

impl<'src> TreeBuilder<'src> {
    fn token(&mut self, kind: SyntaxKind, span: Span) {
        let text = &self.source[span];
        self.builder.token(kind.into(), text);
    }

    fn start_node(&mut self, kind: SyntaxKind) {
        self.builder.start_node(kind.into());
    }

    fn finish_node(&mut self) {
        self.builder.finish_node();
    }
}

// Parser that builds tree inline
fn module_path_parser<'src>(
    builder: &mut TreeBuilder<'src>
) -> impl Parser<Token, (), Error = Simple<Token>> {
    // Parser logic that calls builder methods
}
```

### Option 2: Return GreenNode Directly

Have parsers return `GreenNode` instead of spans:

```rust
fn module_path_parser(source: &str)
    -> impl Parser<Token, GreenNode, Error = Simple<Token>>
{
    filter_map(|span, token| match token {
        Token::Identifier => Ok(span),
        _ => Err(...)
    })
    .separated_by(just(Token::Dot))
    .at_least(1)
    .map(move |segments| {
        let mut builder = GreenNodeBuilder::new();
        builder.start_node(SyntaxKind::ModulePath.into());
        // Build tree from segments
        builder.finish_node();
        builder.finish()
    })
}
```

### Option 3: Hybrid Approach (Recommended)

Keep AST structs but build trees eagerly:

```rust
pub struct ModulePath {
    pub syntax: SyntaxNode,
}

impl ModulePath {
    fn new(source: &str, segments: Vec<Span>) -> Self {
        let mut builder = GreenNodeBuilder::new();
        builder.start_node(SyntaxKind::ModulePath.into());

        for (i, span) in segments.iter().enumerate() {
            if i > 0 {
                builder.token(SyntaxKind::Dot.into(), ".");
            }
            builder.token(SyntaxKind::Identifier.into(), &source[span.clone()]);
        }

        builder.finish_node();
        let syntax = SyntaxNode::new_root(builder.finish());

        Self { syntax }
    }

    // Accessor methods derive from syntax tree
    pub fn segments(&self) -> impl Iterator<Item = SyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .filter(|tok| tok.kind() == SyntaxKind::Identifier)
    }
}

fn module_path_parser(source: &str)
    -> impl Parser<Token, ModulePath, Error = Simple<Token>>
{
    // Parse spans, then immediately build ModulePath
    filter_map(...)
        .separated_by(just(Token::Dot))
        .at_least(1)
        .map(move |segments| ModulePath::new(source, segments))
}
```

## SyntaxKind Design

### Current: Separate Token and SyntaxKind

```rust
// In kestrel-lexer
pub enum Token { Identifier, Module, ... }

// In kestrel-syntax-tree
pub enum SyntaxKind {
    Identifier, Module, ...,  // Tokens
    ModulePath, ...,           // Nodes
}
```

**Problem:** Duplication and manual mapping

### Better: Make Token convertible to SyntaxKind

```rust
// In kestrel-lexer - just tokens
pub enum Token {
    Identifier, Module, Dot, ...
}

// In kestrel-syntax-tree - tokens + nodes
#[repr(u16)]
pub enum SyntaxKind {
    // Tokens (0-999) - match Token discriminants
    Identifier = 0,
    String = 1,
    // ... match Token enum

    // Nodes (1000+)
    Root = 1000,
    ModulePath = 1001,
    ModuleDeclaration = 1002,
}

impl From<Token> for SyntaxKind {
    fn from(token: Token) -> Self {
        // Safe because discriminants match
        unsafe { std::mem::transmute(token as u16) }
    }
}
```

## Recommendation

For Kestrel, I recommend **Option 3 (Hybrid)** because:

1. ✅ AST structs provide convenient typed access
2. ✅ Syntax tree built eagerly (no two-pass)
3. ✅ Can still access full lossless tree via `.syntax`
4. ✅ Works well with Chumsky's combinator style
5. ✅ Easy to add accessor methods that query the syntax tree

The key insight: **Don't store spans separately**. Store the syntax tree and derive everything from it.
