# Pattern Matching Implementation Plan

This document provides a detailed, step-by-step plan for implementing pattern matching in the Kestrel compiler. The full specification is in `docs/pattern-matching.md`.

## Overview

Pattern matching enables destructuring values and controlling flow based on their shape. Implementation is divided into phases:

1. **Phase 1**: Pattern types (syntax, semantic tree, parsing, binding)
2. **Phase 2**: Replace let/var binding name with pattern
3. **Phase 3**: Create `kestrel-semantic-pattern-matching` crate (irrefutability analysis)
4. **Phase 4**: Match expressions + exhaustiveness analysis
5. **Phase 5**: If-let expressions
6. **Phase 6**: Guard-let statements
7. **Phase 7**: While-let expressions

---

## Pattern Matching Algorithms

### Maranget's Usefulness Algorithm

The implementation uses Luc Maranget's pattern matrix algorithm for exhaustiveness and usefulness checking, as described in "Warnings for pattern matching" (JFP 2007). This algorithm is used by OCaml, Haskell, and Rust.

**Key concepts:**

1. **Pattern Matrix**: Patterns are represented as a matrix where each row is a match arm and each column corresponds to a component of the scrutinee type.

2. **Usefulness**: A pattern `p` is *useful* with respect to a pattern matrix `P` if there exists a value that:
   - Matches `p`
   - Does NOT match any row in `P`

3. **Exhaustiveness**: A pattern matrix is exhaustive if the wildcard pattern `_` is NOT useful (meaning all values are covered).

4. **Redundancy**: A pattern arm is redundant if it is NOT useful with respect to the preceding arms.

**Algorithm pseudocode:**

```
useful(P, q) where P is the existing pattern matrix, q is the new pattern:
  if P is empty:
    return true  # q matches everything that falls through
  
  if q has no columns:
    return P has no rows  # base case
  
  let c = first constructor in first column of q
  
  if c is a wildcard:
    # Check if useful for any possible constructor
    return exists constructor k in type: useful(specialize(P, k), specialize(q, k))
  else:
    # Check usefulness for this specific constructor
    return useful(specialize(P, c), specialize(q, c))

specialize(P, c):
  # Remove rows that don't match c, expand those that do
  for each row r in P:
    if first pattern of r is c(...):
      yield row with c's subpatterns + rest of r
    elif first pattern of r is wildcard:
      yield row with (arity of c) wildcards + rest of r
    # else: discard row (doesn't match c)
```

### Guards and Exhaustiveness

**Important**: Guards do NOT affect the pattern matrix. The algorithm treats patterns with guards as follows:

1. Build the pattern matrix using only the pattern structure (ignoring guards entirely)
2. For exhaustiveness, assume any guard may fail at runtime
3. This means a pattern with a guard does NOT "cover" its cases for exhaustiveness purposes
4. For usefulness/redundancy, a guarded arm is always considered "useful" since the guard might fail and the pattern might need to match again

```rust
// Example: This is NOT exhaustive because the guard might fail
match opt {
    .Some(n) if n > 0 => "positive",  // Guard might fail!
    .None => "nothing"
}
// Missing: .Some(n) where n <= 0

// Correct version:
match opt {
    .Some(n) if n > 0 => "positive",
    .Some(_) => "non-positive",  // Fallback for when guard fails
    .None => "nothing"
}
```

### Irrefutability Check

A pattern is *irrefutable* if it matches all possible values of its type. This is a simpler check than full exhaustiveness:

```
irrefutable(pattern, type):
  match pattern:
    Wildcard => true
    Binding(_) => true
    Tuple(ps) => all(irrefutable(p, t) for (p, t) in zip(ps, tuple_types))
    Enum(case, _) => type has exactly one case AND subpatterns are irrefutable
    Struct { .. } => all named fields are irrefutable (with .. for rest)
    Literal(_) => false  # never irrefutable
    Range(_) => false
    Or(ps) => false  # or-patterns are refutable by nature (could be simplified but conservative)
```

---

## Phase 1: Pattern Types

**Goal**: Define pattern syntax and semantic representation for basic patterns.

### Phase 1a: Simple Patterns

Start with a subset of patterns:
- Wildcard (`_`)
- Binding (`name`, `var name`)
- Tuple (`(a, b)`)
- Literal (int, string, char, bool)
- Enum variant (`.Case`, `.Case(x)`)

### Files to Modify

#### 1. Lexer: `lib/kestrel-lexer/src/lib.rs`

Add the `Guard` keyword token (required for Phase 6, but should be reserved from the start):

```rust
#[token("guard")]
Guard,
```

Note: `Token::Match` already exists in the lexer.

#### 2. Syntax Tree: `lib/kestrel-syntax-tree/src/lib.rs`

Add new `SyntaxKind` variants:

```rust
// Pattern nodes (add after existing nodes)
Pattern,              // Root pattern node
WildcardPattern,      // _
BindingPattern,       // name or var name  
TuplePattern,         // (p1, p2, ...)
TuplePatternElement,  // Single element in tuple pattern
LiteralPattern,       // 42, "hello", 'c', true
EnumPattern,          // .Case or .Case(args)
EnumPatternArg,       // Single arg in enum pattern: label or label: pattern
OrPattern,            // pattern or pattern (Phase 1b)
AtPattern,            // name @ pattern (Phase 1b)
RestPattern,          // .. or ..name (Phase 1b)
RangePattern,         // 1..10, 'a'..='z' (Phase 1b)
StructPattern,        // Point { x, y } (Phase 1b)
ArrayPattern,         // [a, b, ..rest] (Phase 1b)
ErrorPattern,         // Error recovery placeholder
```

#### 3. Parser: Create `lib/kestrel-parser/src/pattern/mod.rs`

```rust
//! Pattern parsing for Kestrel
//!
//! Supports patterns in let/var bindings, match arms, if-let, etc.
//!
//! Grammar (Phase 1a):
//!   pattern := wildcard | binding | tuple | literal | enum
//!   wildcard := "_"
//!   binding := "var"? IDENT
//!   tuple := "(" pattern ("," pattern)* ")"
//!   literal := INTEGER | STRING | CHAR | "true" | "false"
//!   enum := "." IDENT ("(" enum_args ")")?
//!   enum_args := enum_arg ("," enum_arg)*
//!   enum_arg := IDENT (":" pattern)?
//!
//! Grammar (Phase 1b adds):
//!   or_pattern := pattern ("or" pattern)*
//!   at_pattern := IDENT "@" pattern
//!   rest := ".." IDENT?
//!   range := range_bound (".." | "..=") range_bound
//!   struct := IDENT "{" struct_fields? "}"
//!   array := "[" array_elements? "]"

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxKind;

use crate::event::EventSink;
use crate::input::{ParserExtra, ParserInput, to_kestrel_span};

/// Parsed pattern data
#[derive(Debug, Clone)]
pub enum PatternVariant {
    /// Wildcard: _
    Wildcard(Span),
    
    /// Binding: name or var name
    Binding {
        var_span: Option<Span>,
        name_span: Span,
    },
    
    /// Tuple: (p1, p2, ...)
    Tuple {
        lparen: Span,
        elements: Vec<PatternVariant>,
        commas: Vec<Span>,
        rparen: Span,
    },
    
    /// Literal: 42, "hello", 'c', true/false, 3.14 (floats parsed but rejected in semantic analysis)
    Literal(LiteralPatternData),
    
    /// Enum variant: .Case or .Case(args)
    Enum(EnumPatternData),
    
    /// Error recovery placeholder
    Error(Span),
}

#[derive(Debug, Clone)]
pub struct LiteralPatternData {
    pub span: Span,
    pub kind: LiteralPatternKind,
}

/// Literal kinds in patterns - reuses concepts from expr::LiteralValue
/// but keeps Float separate for error reporting (E0524)
#[derive(Debug, Clone)]
pub enum LiteralPatternKind {
    Integer,
    Float,   // Parsed but will emit E0524 during semantic analysis
    String,
    Char,
    Bool(bool),
}

#[derive(Debug, Clone)]
pub struct EnumPatternData {
    pub dot_span: Span,
    pub case_span: Span,
    pub args: Option<EnumPatternArgs>,
}

#[derive(Debug, Clone)]
pub struct EnumPatternArgs {
    pub lparen: Span,
    pub args: Vec<EnumPatternArg>,
    pub commas: Vec<Span>,
    pub rparen: Span,
}

#[derive(Debug, Clone)]
pub struct EnumPatternArg {
    pub label_span: Span,
    pub colon: Option<Span>,
    pub pattern: Option<Box<PatternVariant>>,
}
```

**Parser implementation with error recovery:**

```rust
/// Main pattern parser (Phase 1a: simple patterns only)
/// For Phase 1b, this becomes the base and or_pattern wraps it
pub fn pattern_parser<'tokens>() 
    -> impl Parser<'tokens, ParserInput<'tokens>, PatternVariant, ParserExtra<'tokens>> + Clone 
{
    recursive(|pattern| {
        let wildcard = just(Token::Underscore)
            .map_with(|_, e| PatternVariant::Wildcard(to_kestrel_span(e.span())));
        
        let binding = just(Token::Var)
            .map_with(|_, e| Some(to_kestrel_span(e.span())))
            .or_not()
            .flatten()
            .then(select! { Token::Identifier = e => to_kestrel_span(e.span()) })
            .map(|(var_span, name_span)| PatternVariant::Binding { var_span, name_span });
        
        let tuple = tuple_pattern_parser(pattern.clone());
        let literal = literal_pattern_parser();
        let enum_pat = enum_pattern_parser(pattern.clone());
        
        // Order matters: try more specific patterns first
        choice((
            wildcard,
            literal,
            enum_pat,
            tuple,
            binding,  // binding last (catches identifiers)
        ))
        // Error recovery: skip to synchronization tokens
        .recover_with(via_parser(
            any()
                .repeated()
                .at_most(10)  // Don't consume too much
                .then(one_of([
                    Token::FatArrow,   // Match arm separator
                    Token::Comma,      // Pattern separator  
                    Token::RParen,     // Tuple/call end
                    Token::RBrace,     // Block/struct end
                    Token::Equals,     // Let binding
                    Token::RBracket,   // Array end
                ]).rewind())
                .map_with(|_, e| PatternVariant::Error(to_kestrel_span(e.span())))
        ))
    })
}

/// Literal pattern parser - includes Float for error detection
fn literal_pattern_parser<'tokens>() 
    -> impl Parser<'tokens, ParserInput<'tokens>, PatternVariant, ParserExtra<'tokens>> + Clone 
{
    select! {
        Token::Integer = e => LiteralPatternData {
            span: to_kestrel_span(e.span()),
            kind: LiteralPatternKind::Integer,
        },
        Token::Float = e => LiteralPatternData {
            span: to_kestrel_span(e.span()),
            kind: LiteralPatternKind::Float,  // Will error in semantic phase
        },
        Token::String = e => LiteralPatternData {
            span: to_kestrel_span(e.span()),
            kind: LiteralPatternKind::String,
        },
        Token::Char = e => LiteralPatternData {
            span: to_kestrel_span(e.span()),
            kind: LiteralPatternKind::Char,
        },
        Token::Boolean = e => {
            let text = e.slice();
            LiteralPatternData {
                span: to_kestrel_span(e.span()),
                kind: LiteralPatternKind::Bool(text == "true"),
            }
        },
    }
    .map(PatternVariant::Literal)
}
```

**Or-pattern parser (Phase 1b):**

```rust
/// Or-pattern: pattern ("or" pattern)*
/// This wraps the base pattern parser and has lower precedence than @
pub fn or_pattern_parser<'tokens>(
    base_pattern: impl Parser<'tokens, ParserInput<'tokens>, PatternVariant, ParserExtra<'tokens>> + Clone
) -> impl Parser<'tokens, ParserInput<'tokens>, PatternVariant, ParserExtra<'tokens>> + Clone 
{
    base_pattern.clone()
        .then(
            just(Token::Or)
                .map_with(|_, e| to_kestrel_span(e.span()))
                .then(base_pattern)
                .repeated()
                .collect::<Vec<_>>()
        )
        .map(|(first, rest)| {
            if rest.is_empty() {
                first
            } else {
                PatternVariant::Or {
                    first: Box::new(first),
                    rest: rest.into_iter().map(|(or_span, pat)| (or_span, Box::new(pat))).collect(),
                }
            }
        })
}

/// @-pattern: IDENT "@" pattern
/// Has higher precedence than "or"
/// Left side MUST be a simple binding (validated in semantic phase)
pub fn at_pattern_parser<'tokens>(
    inner: impl Parser<'tokens, ParserInput<'tokens>, PatternVariant, ParserExtra<'tokens>> + Clone
) -> impl Parser<'tokens, ParserInput<'tokens>, PatternVariant, ParserExtra<'tokens>> + Clone 
{
    select! { Token::Identifier = e => to_kestrel_span(e.span()) }
        .then(just(Token::At).map_with(|_, e| to_kestrel_span(e.span())))
        .then(inner.clone())
        .map(|((name_span, at_span), pattern)| {
            PatternVariant::At {
                name_span,
                at_span,
                pattern: Box::new(pattern),
            }
        })
        .or(inner)
}

/// Full pattern parser with correct precedence (Phase 1b):
///   or_pattern > at_pattern > base_pattern
pub fn full_pattern_parser<'tokens>() 
    -> impl Parser<'tokens, ParserInput<'tokens>, PatternVariant, ParserExtra<'tokens>> + Clone 
{
    recursive(|pattern| {
        let base = base_pattern_parser(pattern.clone());
        let with_at = at_pattern_parser(base);
        or_pattern_parser(with_at)
    })
}
```

#### 4. Semantic Tree: `lib/kestrel-semantic-tree/src/pattern.rs`

Extend the existing `PatternKind` enum. Note: We reuse `LiteralValue` from `expr.rs` which already includes Float.

```rust
use crate::expr::LiteralValue;  // Reuse existing literal value type

/// Represents the kind of pattern.
#[derive(Debug, Clone)]
pub enum PatternKind {
    /// Simple local binding: `let x` or `var x`
    Local {
        local_id: LocalId,
        mutability: Mutability,
        name: String,
    },
    
    /// Wildcard pattern: `_`
    Wildcard,
    
    /// Tuple pattern: `(p1, p2, ...)`
    Tuple {
        elements: Vec<Pattern>,
    },
    
    /// Literal pattern: `42`, `"hello"`, `'c'`, `true`
    /// Note: Float literals are parsed but rejected with E0524
    Literal {
        value: LiteralValue,  // Reuses expr::LiteralValue
    },
    
    /// Enum variant pattern: `.Case` or `.Case(x, y)`
    /// case_id is resolved during type inference when scrutinee type is known
    EnumVariant {
        /// Resolved case symbol (None until type inference resolves it)
        case_id: Option<SymbolId>,
        /// Case name as written in source
        case_name: String,
        /// Bindings for associated values
        bindings: Vec<EnumPatternBinding>,
    },
    
    /// Or-pattern: `a or b or c` (Phase 1b)
    Or {
        alternatives: Vec<Pattern>,
    },
    
    /// @-pattern: `x @ .Some(_)` (Phase 1b)
    At {
        binding: LocalId,
        name: String,
        mutability: Mutability,
        subpattern: Box<Pattern>,
    },
    
    /// Rest pattern in tuple: `..` (Phase 1b)
    /// Note: In tuples, `..` ignores elements. In arrays, `..name` binds to slice.
    TupleRest,
    
    /// Range pattern: `1..=10`, `'a'..'z'` (Phase 1b)
    Range {
        start: LiteralValue,
        end: LiteralValue,
        inclusive: bool,  // true for ..=, false for ..
    },
    
    /// Struct pattern: `Point { x, y: 0, .. }` (Phase 1b)
    Struct {
        struct_id: SymbolId,
        fields: Vec<StructPatternField>,
        has_rest: bool,  // true if `..` present
    },
    
    /// Array pattern: `[first, ..rest, last]` (Phase 1b)
    Array {
        elements: Vec<ArrayPatternElement>,
    },
    
    /// Error pattern (poison value)
    Error,
}

/// A binding in an enum pattern
#[derive(Debug, Clone)]
pub struct EnumPatternBinding {
    /// Label name (must match enum case parameter)
    pub label: String,
    /// Span of the label in source
    pub label_span: Span,
    /// The sub-pattern for this binding
    pub pattern: Pattern,
}

/// A field in a struct pattern
#[derive(Debug, Clone)]
pub struct StructPatternField {
    /// Field name
    pub name: String,
    /// Span of field name
    pub name_span: Span,
    /// Pattern to match against field value
    pub pattern: Pattern,
}

/// An element in an array pattern
#[derive(Debug, Clone)]
pub enum ArrayPatternElement {
    /// Regular pattern
    Pattern(Pattern),
    /// Rest pattern: `..` or `..name`
    Rest {
        binding: Option<(LocalId, String)>,  // None for `..`, Some for `..name`
        span: Span,
    },
}
```

Add constructors:

```rust
impl Pattern {
    /// Create a wildcard pattern
    pub fn wildcard(ty: Ty, span: Span) -> Self {
        Pattern {
            kind: PatternKind::Wildcard,
            ty,
            span,
        }
    }
    
    /// Create a tuple pattern
    pub fn tuple(elements: Vec<Pattern>, ty: Ty, span: Span) -> Self {
        Pattern {
            kind: PatternKind::Tuple { elements },
            ty,
            span,
        }
    }
    
    /// Create a literal pattern
    pub fn literal(value: LiteralValue, ty: Ty, span: Span) -> Self {
        Pattern {
            kind: PatternKind::Literal { value },
            ty,
            span,
        }
    }
    
    /// Create an enum variant pattern (unresolved)
    pub fn enum_variant_unresolved(
        case_name: String,
        bindings: Vec<EnumPatternBinding>, 
        ty: Ty, 
        span: Span
    ) -> Self {
        Pattern {
            kind: PatternKind::EnumVariant { case_id: None, case_name, bindings },
            ty,
            span,
        }
    }
    
    /// Create an enum variant pattern (resolved)
    pub fn enum_variant(
        case_id: SymbolId,
        case_name: String, 
        bindings: Vec<EnumPatternBinding>, 
        ty: Ty, 
        span: Span
    ) -> Self {
        Pattern {
            kind: PatternKind::EnumVariant { case_id: Some(case_id), case_name, bindings },
            ty,
            span,
        }
    }
    
    /// Create an or-pattern
    pub fn or(alternatives: Vec<Pattern>, ty: Ty, span: Span) -> Self {
        Pattern {
            kind: PatternKind::Or { alternatives },
            ty,
            span,
        }
    }
    
    /// Create an @-pattern
    pub fn at(
        binding: LocalId,
        name: String,
        mutability: Mutability,
        subpattern: Box<Pattern>,
        ty: Ty,
        span: Span,
    ) -> Self {
        Pattern {
            kind: PatternKind::At { binding, name, mutability, subpattern },
            ty,
            span,
        }
    }
}
```

#### 5. Binder: Create `lib/kestrel-semantic-tree-binder/src/body_resolver/patterns.rs`

```rust
//! Pattern resolution for the body resolver.
//!
//! Patterns are resolved in two stages:
//! 1. Binder: Create pattern structure with unresolved enum cases
//! 2. Type inference: Resolve enum cases when scrutinee type is known

use kestrel_semantic_tree::expr::LiteralValue;
use kestrel_semantic_tree::pattern::{Pattern, PatternKind, Mutability, EnumPatternBinding};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use super::context::BodyResolutionContext;

/// Resolve a pattern from syntax to semantic form.
pub fn resolve_pattern(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Pattern {
    let span = get_node_span(node, ctx.file_id);
    
    match node.kind() {
        SyntaxKind::WildcardPattern => resolve_wildcard_pattern(node, ctx),
        SyntaxKind::BindingPattern => resolve_binding_pattern(node, ctx),
        SyntaxKind::TuplePattern => resolve_tuple_pattern(node, ctx),
        SyntaxKind::LiteralPattern => resolve_literal_pattern(node, ctx),
        SyntaxKind::EnumPattern => resolve_enum_pattern(node, ctx),
        SyntaxKind::OrPattern => resolve_or_pattern(node, ctx),
        SyntaxKind::AtPattern => resolve_at_pattern(node, ctx),
        SyntaxKind::ErrorPattern => Pattern::error(span),
        _ => Pattern::error(span),
    }
}

fn resolve_wildcard_pattern(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Pattern {
    let span = get_node_span(node, ctx.file_id);
    // Type will be inferred later
    Pattern::wildcard(Ty::infer(span.clone()), span)
}

fn resolve_binding_pattern(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Pattern {
    let span = get_node_span(node, ctx.file_id);
    
    // Check for `var` keyword
    let is_mutable = node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .any(|t| t.kind() == SyntaxKind::Var);
    
    // Extract identifier
    let name = node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| t.text().to_string())
        .unwrap_or_else(|| "?".to_string());
    
    // Create local binding
    let mutability = if is_mutable { Mutability::Mutable } else { Mutability::Immutable };
    let ty = Ty::infer(span.clone());
    let local_id = ctx.local_scope.bind(name.clone(), ty.clone(), is_mutable, span.clone());
    
    Pattern::local(local_id, mutability, name, ty, span)
}

fn resolve_literal_pattern(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Pattern {
    let span = get_node_span(node, ctx.file_id);
    
    // Extract the literal token
    let token = node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .next();
    
    let Some(token) = token else {
        return Pattern::error(span);
    };
    
    let (value, ty) = match token.kind() {
        SyntaxKind::Integer => {
            let text = token.text();
            let n = parse_integer(text).unwrap_or(0);
            (LiteralValue::Integer(n), Ty::int(span.clone()))
        }
        SyntaxKind::Float => {
            // Parse float but emit error during semantic analysis (E0524)
            let text = token.text();
            let f = text.parse::<f64>().unwrap_or(0.0);
            (LiteralValue::Float(f), Ty::float(span.clone()))
        }
        SyntaxKind::String => {
            let text = parse_string_literal(token.text());
            (LiteralValue::String(text), Ty::string(span.clone()))
        }
        SyntaxKind::Char => {
            let c = parse_char_literal(token.text());
            (LiteralValue::Char(c), Ty::char(span.clone()))
        }
        SyntaxKind::Boolean => {
            let b = token.text() == "true";
            (LiteralValue::Bool(b), Ty::bool(span.clone()))
        }
        _ => return Pattern::error(span),
    };
    
    Pattern::literal(value, ty, span)
}

fn resolve_enum_pattern(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Pattern {
    let span = get_node_span(node, ctx.file_id);
    
    // Extract case name (after the dot)
    let case_name = node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| t.text().to_string())
        .unwrap_or_else(|| "?".to_string());
    
    // Resolve bindings if present
    let bindings = node.children()
        .filter(|c| c.kind() == SyntaxKind::EnumPatternArg)
        .map(|arg_node| resolve_enum_pattern_arg(&arg_node, ctx))
        .collect();
    
    // Type will be inferred from context (scrutinee type)
    // case_id will be resolved during type inference
    let ty = Ty::infer(span.clone());
    
    Pattern::enum_variant_unresolved(case_name, bindings, ty, span)
}

fn resolve_enum_pattern_arg(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> EnumPatternBinding {
    let span = get_node_span(node, ctx.file_id);
    
    // Extract label
    let label = node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| t.text().to_string())
        .unwrap_or_else(|| "?".to_string());
    
    let label_span = /* extract label span */;
    
    // Check for explicit pattern after colon
    let pattern = node.children()
        .find(|c| matches!(c.kind(), SyntaxKind::Pattern | /* other pattern kinds */))
        .map(|p| resolve_pattern(&p, ctx))
        .unwrap_or_else(|| {
            // Shorthand: label becomes binding name
            let ty = Ty::infer(span.clone());
            let local_id = ctx.local_scope.bind(label.clone(), ty.clone(), false, span.clone());
            Pattern::local(local_id, Mutability::Immutable, label.clone(), ty, span.clone())
        });
    
    EnumPatternBinding { label, label_span, pattern }
}

fn resolve_at_pattern(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Pattern {
    let span = get_node_span(node, ctx.file_id);
    
    // Extract binding name (before @)
    let name = node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| t.text().to_string())
        .unwrap_or_else(|| "?".to_string());
    
    // Check for var keyword before the name
    let is_mutable = /* check for var */;
    let mutability = if is_mutable { Mutability::Mutable } else { Mutability::Immutable };
    
    // Resolve subpattern
    let subpattern = node.children()
        .find(|c| is_pattern_node(c))
        .map(|p| resolve_pattern(&p, ctx))
        .unwrap_or_else(|| Pattern::error(span.clone()));
    
    // Create binding for the @ pattern
    let ty = Ty::infer(span.clone());
    let local_id = ctx.local_scope.bind(name.clone(), ty.clone(), is_mutable, span.clone());
    
    Pattern::at(local_id, name, mutability, Box::new(subpattern), ty, span)
}

fn resolve_or_pattern(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Pattern {
    let span = get_node_span(node, ctx.file_id);
    
    // Collect all alternatives
    let alternatives: Vec<Pattern> = node.children()
        .filter(|c| is_pattern_node(c))
        .map(|p| resolve_pattern(&p, ctx))
        .collect();
    
    if alternatives.is_empty() {
        return Pattern::error(span);
    }
    
    // Type is inferred (all alternatives must have same type)
    let ty = Ty::infer(span.clone());
    
    Pattern::or(alternatives, ty, span)
}
```

### Phase 1b: Complex Patterns

Add in subsequent iterations:
- Range patterns (`1..=10`, `'a'..='z'`)
- Struct patterns (`Point { x, y }`)
- Array patterns (`[first, ..rest]`)
- Rest patterns (`..`, `..name`)
- Or-patterns (`a or b`)
- @-patterns (`x @ .Some(_)`)

### @-pattern Validation (E0519)

During semantic analysis, validate that @-patterns have correct structure:

```rust
fn validate_at_pattern(pattern: &Pattern, ctx: &mut AnalysisContext) {
    if let PatternKind::At { subpattern, .. } = &pattern.kind {
        // The left side is already constrained to be a binding by the parser
        // Validate no nested @ patterns: x @ y @ pattern is invalid
        if matches!(subpattern.kind, PatternKind::At { .. }) {
            ctx.emit(diagnostics::InvalidAtPattern {
                span: pattern.span.clone(),
                reason: "nested @ patterns are not allowed",
            });
        }
    }
}
```

---

## Phase 2: Replace let/var Binding with Pattern

**Goal**: Modify variable declarations to accept full patterns instead of just identifiers.

### Type Annotation Constraint

When a type annotation is present on a pattern binding, it adds a constraint:

```rust
// let (x, y): (Int, String) = expr
// Constraints:
// 1. pattern.ty = (Int, String)  -- from annotation
// 2. pattern.ty = expr.ty        -- from initializer
// 3. x.ty = Int, y.ty = String   -- from tuple structure
```

### Files to Modify

#### 1. Parser: `lib/kestrel-parser/src/stmt/mod.rs`

Change `VariableDeclarationData` to use a pattern:

```rust
/// Raw parsed data for a variable declaration
#[derive(Debug, Clone)]
pub struct VariableDeclarationData {
    /// Span of let/var keyword
    pub mutability_span: Span,
    /// Whether this is var (affects default mutability in pattern)
    pub is_mutable: bool,
    /// The pattern (was: name_span)
    pub pattern: PatternVariant,
    /// Optional type annotation: (colon_span, type)
    pub type_annotation: Option<(Span, TyVariant)>,
    /// Optional initializer: (equals_span, expression)
    pub initializer: Option<(Span, ExprVariant)>,
    /// Semicolon span
    pub semicolon: Span,
}
```

Update the parser:

```rust
fn variable_declaration_parser<'tokens>() -> impl Parser<...> {
    skip_trivia()
        .ignore_then(
            just(Token::Let)
                .map_with(|_, e| (to_kestrel_span(e.span()), false))
                .or(just(Token::Var).map_with(|_, e| (to_kestrel_span(e.span()), true))),
        )
        .then(pattern_parser())  // Changed from identifier
        .then(/* type annotation */)
        .then(/* initializer */)
        .then(/* semicolon */)
        .map(/* construct VariableDeclarationData */)
}
```

#### 2. Binder: `lib/kestrel-semantic-tree-binder/src/body_resolver/statements.rs`

Update `resolve_variable_declaration`:

```rust
pub fn resolve_variable_declaration(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Option<Statement> {
    // ... extract pattern node instead of name
    
    let pattern_node = node.children()
        .find(|c| c.kind() == SyntaxKind::Pattern)?;
    
    let pattern = resolve_pattern(&pattern_node, ctx);
    
    // Resolve type annotation if present
    let type_annotation = /* ... */;
    
    // Resolve initializer if present
    let value = /* ... */;
    
    // Create binding statement
    Some(Statement::binding(pattern, type_annotation, value, span))
}
```

#### 3. Type Inference

Add constraint for type annotation:

```rust
StatementKind::Binding { pattern, type_annotation, value } => {
    ctx.register_type(&pattern.ty);
    
    // If type annotation present, constrain pattern type
    if let Some(annotation_ty) = type_annotation {
        ctx.equate(pattern.ty.id(), annotation_ty.id(), pattern.span.clone());
    }
    
    // If initializer present, constrain pattern type to match
    if let Some(init) = value {
        generate_expression_constraints(ctx, init);
        ctx.equate(pattern.ty.id(), init.ty.id(), stmt.span.clone());
    }
    
    // Generate constraints for pattern structure
    generate_pattern_constraints(ctx, pattern);
}
```

---

## Phase 3: Pattern Analysis Crate

**Goal**: Create `kestrel-semantic-pattern-matching` crate for pattern analysis.

### Crate Structure

```
lib/kestrel-semantic-pattern-matching/
├── Cargo.toml
└── src/
    ├── lib.rs              # Public API
    ├── irrefutable.rs      # Irrefutability checking
    ├── exhaustiveness.rs   # Exhaustiveness checking (for match)
    ├── usefulness.rs       # Maranget's usefulness algorithm
    ├── matrix.rs           # Pattern matrix representation
    ├── constructor.rs      # Constructor representation
    ├── witness.rs          # Witness generation for error messages
    └── diagnostics.rs      # Error types
```

### Cargo.toml

```toml
[package]
name = "kestrel-semantic-pattern-matching"
version = "0.1.0"
edition = "2021"

[dependencies]
kestrel-semantic-tree = { path = "../kestrel-semantic-tree" }
kestrel-semantic-model = { path = "../kestrel-semantic-model" }
kestrel-span = { path = "../kestrel-span" }
kestrel-reporting = { path = "../kestrel-reporting" }
```

### Core Types: `src/lib.rs`

```rust
//! Pattern matching analysis for Kestrel.
//!
//! This crate provides:
//! - Irrefutability checking for let/var bindings
//! - Exhaustiveness checking for match expressions
//! - Redundancy detection for match arms
//! - Witness generation for helpful error messages

mod constructor;
mod diagnostics;
mod exhaustiveness;
mod irrefutable;
mod matrix;
mod usefulness;
mod witness;

pub use diagnostics::*;
pub use exhaustiveness::check_exhaustiveness;
pub use irrefutable::check_irrefutable;

use kestrel_semantic_tree::pattern::Pattern;
use kestrel_semantic_tree::ty::Ty;

/// Result of irrefutability check
pub struct IrrefutableResult {
    /// Whether the pattern is irrefutable
    pub is_irrefutable: bool,
    /// Example of unmatched value if refutable
    pub witness: Option<Witness>,
}

/// Result of exhaustiveness check  
pub struct ExhaustivenessResult {
    /// Whether the match is exhaustive
    pub is_exhaustive: bool,
    /// Examples of unmatched values if non-exhaustive
    pub witnesses: Vec<Witness>,
    /// Indices of redundant arms
    pub redundant_arms: Vec<usize>,
}
```

### Constructor: `src/constructor.rs`

```rust
//! Constructor representation for pattern matching.
//!
//! A "constructor" in the pattern matching sense is a way to build a value
//! of a type. For exhaustiveness checking, we need to know all constructors
//! of a type to determine if patterns cover all cases.

use kestrel_semantic_tree::ty::{Ty, TyKind};
use kestrel_semantic_model::SemanticModel;

/// A constructor in the pattern matching sense
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constructor {
    /// Unit type () - single constructor
    Unit,
    
    /// Boolean variants - two constructors
    Bool(bool),
    
    /// Enum case - finite constructors (one per case)
    EnumCase {
        enum_id: SymbolId,
        case_id: SymbolId,
        case_name: String,
        arity: usize,  // Number of associated values
    },
    
    /// Struct - single constructor
    Struct {
        struct_id: SymbolId,
        arity: usize,  // Number of fields
    },
    
    /// Tuple - single constructor for given arity
    Tuple { arity: usize },
    
    /// Integer literal - infinite constructors, requires wildcard
    IntLiteral(i64),
    
    /// Integer range - represents a contiguous range of int constructors
    IntRange { 
        start: i64, 
        end: i64, 
        inclusive: bool,
    },
    
    /// String literal - infinite constructors, requires wildcard
    StringLiteral(String),
    
    /// Char literal - finite but large (could track for bounded types)
    CharLiteral(char),
    
    /// Char range - represents a contiguous range
    CharRange { 
        start: char, 
        end: char, 
        inclusive: bool,
    },
    
    /// Wildcard - not a real constructor, matches anything
    /// Used as a placeholder in the algorithm
    Wildcard,
    
    /// Array with specific length constraints
    /// For exhaustiveness: requires wildcard (variable length)
    Array { 
        /// Minimum number of fixed elements required
        min_length: usize,
        /// Whether there's a rest pattern
        has_rest: bool,
    },
    
    /// Never type - zero constructors (uninhabited)
    Never,
}

impl Constructor {
    /// Get all constructors for a type.
    /// Returns None if the type has infinite constructors (Int, String, Array).
    pub fn all_for_type(ty: &Ty, model: &SemanticModel) -> Option<Vec<Constructor>> {
        match ty.kind() {
            TyKind::Unit => Some(vec![Constructor::Unit]),
            
            TyKind::Bool => Some(vec![
                Constructor::Bool(true), 
                Constructor::Bool(false),
            ]),
            
            TyKind::Never => Some(vec![]),  // No constructors - always exhaustive
            
            TyKind::Enum { symbol, .. } => {
                let cases = symbol.cases();
                Some(cases.iter().map(|c| {
                    let arity = c.callable_behavior()
                        .map(|cb| cb.parameters().len())
                        .unwrap_or(0);
                    Constructor::EnumCase {
                        enum_id: symbol.metadata().id(),
                        case_id: c.metadata().id(),
                        case_name: c.metadata().name().value.clone(),
                        arity,
                    }
                }).collect())
            }
            
            TyKind::Struct { symbol, .. } => {
                Some(vec![Constructor::Struct {
                    struct_id: symbol.metadata().id(),
                    arity: symbol.fields().len(),
                }])
            }
            
            TyKind::Tuple { elements } => {
                Some(vec![Constructor::Tuple { arity: elements.len() }])
            }
            
            // Infinite constructor spaces - require wildcard
            TyKind::Int | TyKind::I8 | TyKind::I16 | TyKind::I32 | TyKind::I64 |
            TyKind::U8 | TyKind::U16 | TyKind::U32 | TyKind::U64 => None,
            
            TyKind::String => None,
            
            TyKind::Char => None,  // Could be made finite for bounded analysis
            
            TyKind::Array { .. } => None,
            
            TyKind::Float | TyKind::F32 | TyKind::F64 => None,
            
            _ => None,
        }
    }
    
    /// Number of sub-patterns this constructor expects
    pub fn arity(&self) -> usize {
        match self {
            Constructor::Unit => 0,
            Constructor::Bool(_) => 0,
            Constructor::EnumCase { arity, .. } => *arity,
            Constructor::Struct { arity, .. } => *arity,
            Constructor::Tuple { arity } => *arity,
            Constructor::IntLiteral(_) | Constructor::IntRange { .. } => 0,
            Constructor::StringLiteral(_) => 0,
            Constructor::CharLiteral(_) | Constructor::CharRange { .. } => 0,
            Constructor::Wildcard => 0,
            Constructor::Array { min_length, has_rest } => {
                // Fixed elements + optional rest
                *min_length + if *has_rest { 1 } else { 0 }
            }
            Constructor::Never => 0,
        }
    }
    
    /// Check if this constructor covers another
    /// Used for range overlap detection
    pub fn covers(&self, other: &Constructor) -> bool {
        match (self, other) {
            (Constructor::Wildcard, _) => true,
            (Constructor::IntRange { start: s1, end: e1, inclusive: i1 }, 
             Constructor::IntLiteral(n)) => {
                let end = if *i1 { *e1 } else { *e1 - 1 };
                *n >= *s1 && *n <= end
            }
            (Constructor::IntRange { start: s1, end: e1, inclusive: i1 },
             Constructor::IntRange { start: s2, end: e2, inclusive: i2 }) => {
                let end1 = if *i1 { *e1 } else { *e1 - 1 };
                let end2 = if *i2 { *e2 } else { *e2 - 1 };
                *s1 <= *s2 && end1 >= end2
            }
            (a, b) => a == b,
        }
    }
}
```

### Witness: `src/witness.rs`

```rust
//! Witness generation for pattern matching errors.
//!
//! A witness is an example value that demonstrates why a match is non-exhaustive.
//! It's used to generate helpful error messages like:
//!   "missing pattern: `.None`"
//!   "missing pattern: `(_, .Some(3))`"

use crate::constructor::Constructor;

/// A witness value demonstrating a pattern gap
#[derive(Debug, Clone)]
pub enum Witness {
    /// Wildcard - any value of this type works as witness
    Any,
    
    /// Specific constructor with sub-witnesses for its arguments
    Constructor {
        /// Display name (e.g., "None", "Some", "Point")
        name: String,
        /// How to display (enum case uses dot prefix)
        style: WitnessStyle,
        /// Sub-witnesses for constructor arguments
        args: Vec<Witness>,
    },
    
    /// Tuple with sub-witnesses for elements
    Tuple(Vec<Witness>),
    
    /// Specific literal value
    Literal(String),  // Display form: "42", "\"hello\"", "'a'"
    
    /// Range of values not covered
    Range {
        start: String,
        end: String,
        inclusive: bool,
    },
    
    /// Array pattern
    Array {
        elements: Vec<Witness>,
        has_rest: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessStyle {
    /// Enum case: displayed as `.Name` or `.Name(args)`
    EnumCase,
    /// Struct: displayed as `Name { field: value }`
    Struct,
    /// Plain: displayed as `Name(args)`
    Plain,
}

impl Witness {
    /// Create a witness for "any value"
    pub fn any_value() -> Self {
        Witness::Any
    }
    
    /// Create a witness for unit ()
    pub fn unit() -> Self {
        Witness::Tuple(vec![])
    }
    
    /// Create a witness from a constructor
    pub fn from_constructor(ctor: &Constructor) -> Self {
        match ctor {
            Constructor::Unit => Witness::unit(),
            
            Constructor::Bool(b) => Witness::Literal(b.to_string()),
            
            Constructor::EnumCase { case_name, arity, .. } => {
                Witness::Constructor {
                    name: case_name.clone(),
                    style: WitnessStyle::EnumCase,
                    args: vec![Witness::Any; *arity],
                }
            }
            
            Constructor::Struct { .. } => {
                Witness::Constructor {
                    name: "...".to_string(),  // Filled in by caller
                    style: WitnessStyle::Struct,
                    args: vec![],  // Filled in by caller
                }
            }
            
            Constructor::Tuple { arity } => {
                Witness::Tuple(vec![Witness::Any; *arity])
            }
            
            Constructor::IntLiteral(n) => Witness::Literal(n.to_string()),
            
            Constructor::IntRange { start, end, inclusive } => {
                Witness::Range {
                    start: start.to_string(),
                    end: end.to_string(),
                    inclusive: *inclusive,
                }
            }
            
            Constructor::StringLiteral(s) => Witness::Literal(format!("\"{}\"", s)),
            
            Constructor::CharLiteral(c) => Witness::Literal(format!("'{}'", c)),
            
            Constructor::CharRange { start, end, inclusive } => {
                Witness::Range {
                    start: format!("'{}'", start),
                    end: format!("'{}'", end),
                    inclusive: *inclusive,
                }
            }
            
            Constructor::Wildcard => Witness::Any,
            
            Constructor::Array { min_length, has_rest } => {
                Witness::Array {
                    elements: vec![Witness::Any; *min_length],
                    has_rest: *has_rest,
                }
            }
            
            Constructor::Never => {
                // Never type has no witnesses - this shouldn't be called
                unreachable!("Never type has no constructors")
            }
        }
    }
    
    /// Apply a constructor to this witness (wrap it)
    pub fn apply_constructor(self, ctor: &Constructor) -> Self {
        // Replace the first Any in constructor's args with this witness
        let mut witness = Witness::from_constructor(ctor);
        if let Witness::Constructor { args, .. } = &mut witness {
            if let Some(first) = args.first_mut() {
                *first = self;
            }
        }
        witness
    }
    
    /// Format for display in error messages
    pub fn display(&self) -> String {
        match self {
            Witness::Any => "_".to_string(),
            
            Witness::Constructor { name, style, args } => {
                let prefix = match style {
                    WitnessStyle::EnumCase => ".",
                    _ => "",
                };
                
                if args.is_empty() {
                    format!("{}{}", prefix, name)
                } else {
                    match style {
                        WitnessStyle::Struct => {
                            format!("{} {{ .. }}", name)
                        }
                        _ => {
                            let args_str = args.iter()
                                .map(|a| a.display())
                                .collect::<Vec<_>>()
                                .join(", ");
                            format!("{}{}({})", prefix, name, args_str)
                        }
                    }
                }
            }
            
            Witness::Tuple(elems) => {
                let elems_str = elems.iter()
                    .map(|e| e.display())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", elems_str)
            }
            
            Witness::Literal(s) => s.clone(),
            
            Witness::Range { start, end, inclusive } => {
                let op = if *inclusive { "..=" } else { ".." };
                format!("{}{}{}", start, op, end)
            }
            
            Witness::Array { elements, has_rest } => {
                let mut parts: Vec<String> = elements.iter()
                    .map(|e| e.display())
                    .collect();
                if *has_rest {
                    parts.push("..".to_string());
                }
                format!("[{}]", parts.join(", "))
            }
        }
    }
}

impl std::fmt::Display for Witness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}
```

### Irrefutability: `src/irrefutable.rs`

```rust
//! Irrefutability checking for patterns.
//!
//! A pattern is irrefutable if it matches all possible values of its type.

use kestrel_semantic_tree::pattern::{Pattern, PatternKind};
use kestrel_semantic_tree::ty::{Ty, TyKind};
use kestrel_semantic_model::SemanticModel;

use crate::witness::Witness;
use crate::IrrefutableResult;

/// Check if a pattern is irrefutable for its type.
pub fn check_irrefutable(
    pattern: &Pattern,
    model: &SemanticModel,
) -> IrrefutableResult {
    let (is_irrefutable, witness) = is_irrefutable_impl(pattern, &pattern.ty, model);
    IrrefutableResult { is_irrefutable, witness }
}

fn is_irrefutable_impl(
    pattern: &Pattern,
    ty: &Ty,
    model: &SemanticModel,
) -> (bool, Option<Witness>) {
    match &pattern.kind {
        // Wildcards always match
        PatternKind::Wildcard => (true, None),
        
        // Simple bindings always match
        PatternKind::Local { .. } => (true, None),
        
        // @-pattern: binding always matches, check subpattern
        PatternKind::At { subpattern, .. } => {
            is_irrefutable_impl(subpattern, ty, model)
        }
        
        // Tuple: all elements must be irrefutable
        PatternKind::Tuple { elements } => {
            if let TyKind::Tuple { elements: ty_elements } = ty.kind() {
                for (i, (pat, elem_ty)) in elements.iter().zip(ty_elements.iter()).enumerate() {
                    let (irref, wit) = is_irrefutable_impl(pat, elem_ty, model);
                    if !irref {
                        // Build tuple witness with Any for other positions
                        let mut tuple_witness = vec![Witness::Any; ty_elements.len()];
                        if let Some(w) = wit {
                            tuple_witness[i] = w;
                        }
                        return (false, Some(Witness::Tuple(tuple_witness)));
                    }
                }
                (true, None)
            } else {
                (false, Some(Witness::Any))
            }
        }
        
        // Literals are never irrefutable (could be other values)
        PatternKind::Literal { value } => {
            let witness = Witness::Literal(format!("value other than {}", value.display()));
            (false, Some(witness))
        }
        
        // Enum variant: only irrefutable if it's the only variant and subpatterns are irrefutable
        PatternKind::EnumVariant { case_id, case_name, bindings } => {
            if let TyKind::Enum { symbol, .. } = ty.kind() {
                let cases = symbol.cases();
                if cases.len() == 1 {
                    // Single variant - check subpatterns
                    for binding in bindings {
                        let (irref, wit) = is_irrefutable_impl(&binding.pattern, &binding.pattern.ty, model);
                        if !irref {
                            return (false, wit);
                        }
                    }
                    (true, None)
                } else {
                    // Multiple variants - find one that's not matched
                    let other_case = cases.iter()
                        .find(|c| c.metadata().name().value != *case_name);
                    
                    if let Some(other) = other_case {
                        let witness = Witness::Constructor {
                            name: other.metadata().name().value.clone(),
                            style: crate::witness::WitnessStyle::EnumCase,
                            args: vec![],
                        };
                        (false, Some(witness))
                    } else {
                        (true, None)
                    }
                }
            } else {
                (false, Some(Witness::Any))
            }
        }
        
        // Or-pattern: irrefutable if ANY alternative is irrefutable
        // (conservative: we say refutable since we can't easily prove one always matches)
        PatternKind::Or { alternatives } => {
            // This is a simplification - a more complex analysis could check
            // if the alternatives together cover all cases
            (false, Some(Witness::Any))
        }
        
        // Range: never irrefutable (always other values in the type)
        PatternKind::Range { .. } => {
            (false, Some(Witness::Any))
        }
        
        // Struct with `..`: irrefutable if all named fields are irrefutable
        PatternKind::Struct { fields, has_rest, .. } => {
            for field in fields {
                let (irref, wit) = is_irrefutable_impl(&field.pattern, &field.pattern.ty, model);
                if !irref {
                    return (false, wit);
                }
            }
            (true, None)
        }
        
        // Array: generally refutable (length varies)
        PatternKind::Array { elements } => {
            // Arrays are refutable unless pattern accepts any length
            let has_rest = elements.iter().any(|e| matches!(e, crate::pattern::ArrayPatternElement::Rest { .. }));
            if has_rest {
                // Still need to check fixed element patterns
                for elem in elements {
                    if let crate::pattern::ArrayPatternElement::Pattern(p) = elem {
                        let (irref, wit) = is_irrefutable_impl(p, &p.ty, model);
                        if !irref {
                            return (false, wit);
                        }
                    }
                }
                (true, None)
            } else {
                // Fixed length pattern - refutable (array could have different length)
                (false, Some(Witness::Array { 
                    elements: vec![Witness::Any],
                    has_rest: true,
                }))
            }
        }
        
        // Rest pattern in tuple: always irrefutable (matches any number)
        PatternKind::TupleRest => (true, None),
        
        PatternKind::Error => (true, None), // Don't cascade errors
    }
}
```

### Pattern Matrix: `src/matrix.rs`

```rust
//! Pattern matrix representation for exhaustiveness checking.

use kestrel_semantic_tree::pattern::Pattern;
use crate::constructor::Constructor;

/// A matrix of patterns where each row is a match arm.
#[derive(Debug, Clone)]
pub struct PatternMatrix {
    /// Number of columns (corresponds to scrutinee arity)
    pub width: usize,
    /// Rows of patterns
    pub rows: Vec<PatternRow>,
}

/// A single row in the pattern matrix
#[derive(Debug, Clone)]
pub struct PatternRow {
    /// Patterns in this row
    pub patterns: Vec<Pattern>,
    /// Index of the original match arm (for redundancy reporting)
    pub arm_index: usize,
    /// Whether this arm has a guard (affects usefulness)
    pub has_guard: bool,
}

impl PatternMatrix {
    /// Create an empty matrix with given width
    pub fn new(width: usize) -> Self {
        PatternMatrix { width, rows: Vec::new() }
    }
    
    /// Add a row to the matrix
    pub fn push_row(&mut self, patterns: Vec<Pattern>, arm_index: usize, has_guard: bool) {
        debug_assert_eq!(patterns.len(), self.width);
        self.rows.push(PatternRow { patterns, arm_index, has_guard });
    }
    
    /// Check if the matrix is empty
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    
    /// Check if matrix has no columns
    pub fn is_unit(&self) -> bool {
        self.width == 0
    }
    
    /// Get the first column's patterns
    pub fn first_column(&self) -> impl Iterator<Item = &Pattern> {
        self.rows.iter().map(|r| &r.patterns[0])
    }
    
    /// Get all head constructors in the first column
    pub fn head_constructors(&self) -> Vec<Constructor> {
        self.first_column()
            .filter_map(|p| p.head_constructor())
            .collect()
    }
    
    /// Specialize the matrix for a constructor.
    /// Keeps rows that match the constructor, expanding their subpatterns.
    pub fn specialize(&self, ctor: &Constructor) -> PatternMatrix {
        let new_width = self.width - 1 + ctor.arity();
        let mut result = PatternMatrix::new(new_width);
        
        for row in &self.rows {
            let first = &row.patterns[0];
            let rest = &row.patterns[1..];
            
            match first.head_constructor() {
                Some(c) if c == *ctor => {
                    // Same constructor: expand subpatterns
                    let mut new_patterns = first.subpatterns().to_vec();
                    new_patterns.extend(rest.iter().cloned());
                    result.push_row(new_patterns, row.arm_index, row.has_guard);
                }
                Some(_) => {
                    // Different constructor: skip this row
                }
                None => {
                    // Wildcard/binding: create arity wildcards
                    let mut new_patterns = vec![Pattern::wildcard_for_ctor(ctor); ctor.arity()];
                    new_patterns.extend(rest.iter().cloned());
                    result.push_row(new_patterns, row.arm_index, row.has_guard);
                }
            }
        }
        
        result
    }
    
    /// Default matrix: keeps only wildcard rows, drops the first column.
    /// Used when checking for missing constructors.
    pub fn default_matrix(&self) -> PatternMatrix {
        let new_width = self.width.saturating_sub(1);
        let mut result = PatternMatrix::new(new_width);
        
        for row in &self.rows {
            let first = &row.patterns[0];
            if first.head_constructor().is_none() {
                // Wildcard: keep this row without the first column
                let rest = row.patterns[1..].to_vec();
                result.push_row(rest, row.arm_index, row.has_guard);
            }
        }
        
        result
    }
}
```

### Usefulness: `src/usefulness.rs`

```rust
//! Maranget's usefulness algorithm.
//!
//! A pattern q is useful with respect to matrix P if there exists
//! a value that matches q but doesn't match any row of P.

use crate::constructor::Constructor;
use crate::matrix::PatternMatrix;
use crate::witness::Witness;

use kestrel_semantic_tree::pattern::Pattern;
use kestrel_semantic_tree::ty::Ty;
use kestrel_semantic_model::SemanticModel;

/// Check if pattern vector q is useful with respect to matrix P.
/// Returns Some(witness) if useful, None if not useful.
pub fn is_useful(
    matrix: &PatternMatrix,
    pattern_row: &[Pattern],
    model: &SemanticModel,
) -> Option<Witness> {
    // Base case: empty matrix means q is useful (matches anything that falls through)
    if matrix.is_empty() {
        return Some(Witness::any_value());
    }
    
    // Base case: no columns means we need to check if there's an empty row
    if matrix.is_unit() {
        // If there's any row without a guard, it catches everything
        let has_unguarded_row = matrix.rows.iter().any(|r| !r.has_guard);
        return if has_unguarded_row {
            None // Caught by an existing row
        } else {
            Some(Witness::unit()) // All rows have guards, might fall through
        };
    }
    
    // Get the first pattern and its type
    let first_pattern = &pattern_row[0];
    let ty = &first_pattern.ty;
    
    // Get all constructors for this type
    let all_constructors = Constructor::all_for_type(ty, model);
    
    // Check based on the head constructor of the first pattern
    match first_pattern.head_constructor() {
        Some(ctor) => {
            // Specific constructor: specialize and recurse
            let specialized = matrix.specialize(&ctor);
            let specialized_row = specialize_row(pattern_row, &ctor);
            is_useful(&specialized, &specialized_row, model)
                .map(|w| w.apply_constructor(&ctor))
        }
        None => {
            // Wildcard/variable pattern
            let covered_ctors: std::collections::HashSet<_> = matrix.head_constructors()
                .into_iter()
                .collect();
            
            match all_constructors {
                Some(all_ctors) => {
                    // Finite constructor set
                    let missing_ctors: Vec<_> = all_ctors.iter()
                        .filter(|c| !covered_ctors.contains(c))
                        .collect();
                    
                    if missing_ctors.is_empty() {
                        // All constructors covered: check each specialization
                        for ctor in &all_ctors {
                            let specialized = matrix.specialize(ctor);
                            let specialized_row = specialize_row(pattern_row, ctor);
                            if let Some(w) = is_useful(&specialized, &specialized_row, model) {
                                return Some(w.apply_constructor(ctor));
                            }
                        }
                        None
                    } else {
                        // Some constructors missing: pattern is useful
                        let first_missing = missing_ctors[0];
                        Some(Witness::from_constructor(first_missing))
                    }
                }
                None => {
                    // Infinite constructor set (Int, String, etc.)
                    // Check if there's a wildcard row
                    if matrix.first_column().any(|p| p.head_constructor().is_none()) {
                        // There's a wildcard - check if we can still be useful
                        let default = matrix.default_matrix();
                        let rest = &pattern_row[1..];
                        is_useful(&default, rest, model)
                    } else {
                        // No wildcard and infinite constructors - always useful
                        Some(Witness::any_value())
                    }
                }
            }
        }
    }
}

fn specialize_row(row: &[Pattern], ctor: &Constructor) -> Vec<Pattern> {
    let first = &row[0];
    let rest = &row[1..];
    
    match first.head_constructor() {
        Some(c) if c == *ctor => {
            // Same constructor: expand subpatterns
            let mut result = first.subpatterns().to_vec();
            result.extend(rest.iter().cloned());
            result
        }
        None => {
            // Wildcard: create arity wildcards
            let wildcards: Vec<Pattern> = (0..ctor.arity())
                .map(|_| Pattern::wildcard_for_ctor(ctor))
                .collect();
            let mut result = wildcards;
            result.extend(rest.iter().cloned());
            result
        }
        Some(_) => {
            // Different constructor: should not happen in valid call
            unreachable!("specialize_row called with mismatched constructor")
        }
    }
}
```

### Integration with Analyzers

Create an analyzer that uses the pattern matching crate:

#### `lib/kestrel-semantic-analyzers/src/analyzers/pattern_check/mod.rs`

```rust
//! Pattern analysis: irrefutability, exhaustiveness, and validation checks.

mod diagnostics;

use kestrel_semantic_pattern_matching::{check_irrefutable, check_exhaustiveness};
use kestrel_semantic_tree::expr::LiteralValue;
use kestrel_semantic_tree::pattern::{Pattern, PatternKind};
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::expr::{Expression, ExprKind};

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

pub struct PatternCheckAnalyzer;

impl Analyzer for PatternCheckAnalyzer {
    fn visit_pattern(&mut self, pattern: &Pattern, ctx: &mut AnalysisContext) {
        // E0524: Float literal in pattern
        if let PatternKind::Literal { value: LiteralValue::Float(f) } = &pattern.kind {
            ctx.emit(diagnostics::FloatLiteralInPattern {
                span: pattern.span.clone(),
                value: *f,
                suggestion: "use a guard instead: `x if x == {f}`",
            });
        }
        
        // E0519: Invalid @-pattern
        self.validate_at_pattern(pattern, ctx);
        
        // E0504, E0511: Or-pattern binding consistency
        self.validate_or_pattern(pattern, ctx);
    }
    
    fn visit_statement(&mut self, stmt: &Statement, ctx: &mut AnalysisContext) {
        // E0502: Check let/var bindings for irrefutability
        if let StatementKind::Binding { pattern, .. } = &stmt.kind {
            let result = check_irrefutable(pattern, ctx.model());
            if !result.is_irrefutable {
                ctx.emit(diagnostics::RefutablePatternInBinding {
                    span: pattern.span.clone(),
                    witness: result.witness,
                });
            }
        }
    }
    
    fn visit_expression(&mut self, expr: &Expression, ctx: &mut AnalysisContext) {
        // Check match expressions for exhaustiveness
        if let ExprKind::Match { scrutinee, arms } = &expr.kind {
            let patterns: Vec<_> = arms.iter().map(|a| &a.pattern).collect();
            let has_guards: Vec<_> = arms.iter().map(|a| a.guard.is_some()).collect();
            
            let result = check_exhaustiveness(
                &scrutinee.ty,
                patterns,
                has_guards,
                ctx.model(),
            );
            
            // E0501: Non-exhaustive match
            if !result.is_exhaustive {
                ctx.emit(diagnostics::NonExhaustiveMatch {
                    span: expr.span.clone(),
                    witnesses: result.witnesses,
                });
            }
            
            // W0508: Unreachable match arm
            for idx in result.redundant_arms {
                ctx.emit(diagnostics::UnreachablePattern {
                    span: arms[idx].pattern.span.clone(),
                });
            }
        }
        
        // W0509: Irrefutable pattern in if-let
        if let ExprKind::IfLet { conditions, .. } = &expr.kind {
            for cond in conditions {
                if let IfCondition::Let { pattern, .. } = cond {
                    let result = check_irrefutable(pattern, ctx.model());
                    if result.is_irrefutable {
                        ctx.emit(diagnostics::IrrefutableIfLetPattern {
                            span: pattern.span.clone(),
                            suggestion: "use `let` instead of `if let`",
                        });
                    }
                }
            }
        }
    }
}

impl PatternCheckAnalyzer {
    fn validate_at_pattern(&self, pattern: &Pattern, ctx: &mut AnalysisContext) {
        if let PatternKind::At { subpattern, .. } = &pattern.kind {
            // E0519: Nested @ patterns not allowed
            if matches!(subpattern.kind, PatternKind::At { .. }) {
                ctx.emit(diagnostics::InvalidAtPattern {
                    span: pattern.span.clone(),
                    reason: "nested @ patterns are not allowed",
                });
            }
        }
    }
    
    fn validate_or_pattern(&self, pattern: &Pattern, ctx: &mut AnalysisContext) {
        if let PatternKind::Or { alternatives } = &pattern.kind {
            if alternatives.is_empty() {
                return;
            }
            
            // Collect bindings from first alternative
            let first_bindings = collect_bindings(&alternatives[0]);
            
            for (i, alt) in alternatives.iter().enumerate().skip(1) {
                let alt_bindings = collect_bindings(alt);
                
                // E0504: Check same names are bound
                for (name, (ty, mutability)) in &first_bindings {
                    match alt_bindings.get(name) {
                        None => {
                            ctx.emit(diagnostics::InconsistentOrPatternBindings {
                                span: alt.span.clone(),
                                missing_name: name.clone(),
                                present_in: 0,
                                missing_in: i,
                            });
                        }
                        Some((alt_ty, alt_mut)) => {
                            // E0511: Check same mutability
                            if mutability != alt_mut {
                                ctx.emit(diagnostics::InconsistentOrPatternMutability {
                                    span: alt.span.clone(),
                                    name: name.clone(),
                                });
                            }
                            // Types are checked by type inference
                        }
                    }
                }
                
                // Check for extra bindings in alternative
                for name in alt_bindings.keys() {
                    if !first_bindings.contains_key(name) {
                        ctx.emit(diagnostics::InconsistentOrPatternBindings {
                            span: alt.span.clone(),
                            missing_name: name.clone(),
                            present_in: i,
                            missing_in: 0,
                        });
                    }
                }
            }
        }
    }
}
```

---

## Pattern Type Inference

**Goal**: Propagate types bidirectionally through patterns during type inference.

### Type Inference for Patterns

Add pattern constraint generation to `kestrel-semantic-type-inference`:

#### `lib/kestrel-semantic-type-inference/src/constraint_generator.rs`

```rust
/// Generate constraints for a pattern.
/// The expected_ty flows from context (e.g., scrutinee type, type annotation).
pub fn generate_pattern_constraints(ctx: &mut InferenceContext, pattern: &Pattern) {
    ctx.register_type(&pattern.ty);
    
    match &pattern.kind {
        PatternKind::Wildcard | PatternKind::Local { .. } => {
            // No additional constraints - type comes from context
        }
        
        PatternKind::Tuple { elements } => {
            // Create tuple type constraint
            let elem_tys: Vec<Ty> = elements.iter()
                .map(|e| {
                    generate_pattern_constraints(ctx, e);
                    e.ty.clone()
                })
                .collect();
            
            let tuple_ty = Ty::tuple(elem_tys, pattern.span.clone());
            ctx.equate(pattern.ty.id(), tuple_ty.id(), pattern.span.clone());
        }
        
        PatternKind::Literal { value } => {
            // Literal constrains the pattern type
            let lit_ty = match value {
                LiteralValue::Integer(_) => Ty::int(pattern.span.clone()),
                LiteralValue::Float(_) => Ty::float(pattern.span.clone()),
                LiteralValue::String(_) => Ty::string(pattern.span.clone()),
                LiteralValue::Char(_) => Ty::char(pattern.span.clone()),
                LiteralValue::Bool(_) => Ty::bool(pattern.span.clone()),
                LiteralValue::Unit => Ty::unit(pattern.span.clone()),
            };
            ctx.equate(pattern.ty.id(), lit_ty.id(), pattern.span.clone());
        }
        
        PatternKind::EnumVariant { case_name, bindings, .. } => {
            // Generate EnumPattern constraint - resolved when pattern.ty is known
            let binding_tys: Vec<(String, TyId)> = bindings.iter()
                .map(|b| {
                    generate_pattern_constraints(ctx, &b.pattern);
                    (b.label.clone(), b.pattern.ty.id())
                })
                .collect();
            
            ctx.enum_pattern(
                pattern.ty.id(),
                case_name.clone(),
                binding_tys,
                pattern.id,  // For recording resolution
                pattern.span.clone(),
            );
        }
        
        PatternKind::Or { alternatives } => {
            // All alternatives must have same type as the or-pattern
            for alt in alternatives {
                generate_pattern_constraints(ctx, alt);
                ctx.equate(pattern.ty.id(), alt.ty.id(), alt.span.clone());
            }
        }
        
        PatternKind::At { subpattern, .. } => {
            // Binding and subpattern have same type
            generate_pattern_constraints(ctx, subpattern);
            ctx.equate(pattern.ty.id(), subpattern.ty.id(), pattern.span.clone());
        }
        
        PatternKind::Range { start, end, .. } => {
            // Range pattern constrains type to the literal type
            let range_ty = match start {
                LiteralValue::Integer(_) => Ty::int(pattern.span.clone()),
                LiteralValue::Char(_) => Ty::char(pattern.span.clone()),
                _ => Ty::error(pattern.span.clone()),
            };
            ctx.equate(pattern.ty.id(), range_ty.id(), pattern.span.clone());
        }
        
        PatternKind::Struct { struct_id, fields, .. } => {
            // Struct pattern: resolve field types from struct definition
            // This is similar to struct initialization
            let struct_ty = Ty::from_symbol(*struct_id, pattern.span.clone());
            ctx.equate(pattern.ty.id(), struct_ty.id(), pattern.span.clone());
            
            // Equate each field pattern with field type
            // (Field lookup happens in solver when struct type is resolved)
            for field in fields {
                generate_pattern_constraints(ctx, &field.pattern);
                ctx.struct_field_pattern(
                    pattern.ty.id(),
                    field.name.clone(),
                    field.pattern.ty.id(),
                    field.name_span.clone(),
                );
            }
        }
        
        PatternKind::Array { elements } => {
            // Array pattern: all elements have same type
            let elem_ty = Ty::infer(pattern.span.clone());
            ctx.register_type(&elem_ty);
            
            for elem in elements {
                match elem {
                    ArrayPatternElement::Pattern(p) => {
                        generate_pattern_constraints(ctx, p);
                        ctx.equate(p.ty.id(), elem_ty.id(), p.span.clone());
                    }
                    ArrayPatternElement::Rest { binding, span } => {
                        if let Some((_, name)) = binding {
                            // Rest binding has array type
                            let rest_ty = Ty::array(elem_ty.clone(), span.clone());
                            // Binding type constraint added by the binding itself
                        }
                    }
                }
            }
            
            let array_ty = Ty::array(elem_ty, pattern.span.clone());
            ctx.equate(pattern.ty.id(), array_ty.id(), pattern.span.clone());
        }
        
        PatternKind::TupleRest | PatternKind::Error => {
            // No constraints
        }
    }
}
```

### Enum Pattern Constraint Resolution

Add to `solver.rs`:

```rust
/// New constraint type for enum patterns
Constraint::EnumPattern {
    pattern_ty: TyId,
    case_name: String,
    bindings: Vec<(String, TyId)>,  // label -> binding type
    pattern_id: PatternId,
    span: Span,
}

/// Resolve enum pattern constraint
fn resolve_enum_pattern(
    ctx: &mut InferenceContext<'_>,
    pattern_ty: TyId,
    case_name: &str,
    bindings: &[(String, TyId)],
    pattern_id: PatternId,
    span: &Span,
) -> Result<SolveResult, InferenceError> {
    let resolved_ty = resolve_type(ctx, pattern_ty);
    
    // If still Infer, defer until scrutinee type is known
    if matches!(resolved_ty.kind(), TyKind::Infer) {
        return Ok(SolveResult::Deferred);
    }
    
    // Must be an enum type
    let TyKind::Enum { symbol: enum_symbol, substitutions } = resolved_ty.kind() else {
        return Err(InferenceError::pattern_type_mismatch(
            "enum",
            &resolved_ty,
            span.clone(),
        ));
    };
    
    // Find the case by name
    let cases = enum_symbol.cases();
    let case = cases.iter().find(|c| c.metadata().name().value == case_name);
    
    let Some(case) = case else {
        return Err(InferenceError::unknown_enum_case(
            case_name.to_string(),
            enum_symbol.metadata().name().value.clone(),
            span.clone(),
        ));
    };
    
    // Validate bindings match case parameters
    let callable = case.callable_behavior();
    
    match (&callable, bindings.is_empty()) {
        // Simple case, no bindings expected
        (None, true) => {
            ctx.pattern_resolutions_mut().insert(pattern_id, case.metadata().id());
            Ok(SolveResult::Solved)
        }
        
        // Simple case but bindings provided
        (None, false) => {
            Err(InferenceError::enum_case_no_params(
                case_name.to_string(),
                bindings.len(),
                span.clone(),
            ))
        }
        
        // Case with params, bindings provided
        (Some(cb), false) => {
            let params = cb.parameters();
            
            // Check arity
            if params.len() != bindings.len() {
                return Err(InferenceError::enum_case_wrong_arity(
                    case_name.to_string(),
                    params.len(),
                    bindings.len(),
                    span.clone(),
                ));
            }
            
            // Match bindings to parameters by label
            for (label, binding_ty) in bindings {
                let param = params.iter().find(|p| &p.label == label);
                
                let Some(param) = param else {
                    return Err(InferenceError::wrong_enum_label(
                        label.clone(),
                        case_name.to_string(),
                        span.clone(),
                    ));
                };
                
                // Apply generic substitutions and equate
                let param_ty = param.ty.apply_substitutions(substitutions);
                ctx.register_type(&param_ty);
                ctx.equate(*binding_ty, param_ty.id(), span.clone());
            }
            
            ctx.pattern_resolutions_mut().insert(pattern_id, case.metadata().id());
            Ok(SolveResult::Solved)
        }
        
        // Case with params but no bindings
        (Some(cb), true) => {
            Err(InferenceError::enum_case_missing_params(
                case_name.to_string(),
                cb.parameters().len(),
                span.clone(),
            ))
        }
    }
}
```

---

## Visibility Checking for Struct Patterns

When matching a struct pattern, we need to check that all accessed fields are visible:

```rust
// In pattern_check analyzer or body resolver
fn check_struct_pattern_visibility(
    pattern: &Pattern,
    ctx: &AnalysisContext,
) {
    if let PatternKind::Struct { struct_id, fields, has_rest, .. } = &pattern.kind {
        let struct_symbol = ctx.model.query(SymbolFor { id: *struct_id });
        let Some(struct_symbol) = struct_symbol else { return };
        
        let all_fields = struct_symbol.fields();
        
        for field_pattern in fields {
            // Find the field in the struct
            let field = all_fields.iter()
                .find(|f| f.metadata().name().value == field_pattern.name);
            
            let Some(field) = field else {
                // E0526: Unknown field
                ctx.emit(diagnostics::UnknownFieldInPattern {
                    span: field_pattern.name_span.clone(),
                    field_name: field_pattern.name.clone(),
                    struct_name: struct_symbol.metadata().name().value.clone(),
                });
                continue;
            };
            
            // Check visibility using existing IsVisibleFrom query
            let is_visible = ctx.model.query(IsVisibleFrom {
                target: field.metadata().id(),
                context: ctx.function_id,
            });
            
            if !is_visible {
                // E0525: Private field in struct pattern
                let visibility = field.metadata()
                    .get_behavior::<VisibilityBehavior>()
                    .and_then(|v| v.visibility().cloned())
                    .unwrap_or(Visibility::Internal);
                
                ctx.emit(diagnostics::PrivateFieldInPattern {
                    span: field_pattern.name_span.clone(),
                    field_name: field_pattern.name.clone(),
                    struct_name: struct_symbol.metadata().name().value.clone(),
                    visibility: visibility.to_string(),
                });
            }
        }
        
        // E0527: Check for missing fields (if no ..)
        if !has_rest {
            for field in all_fields {
                let is_visible = ctx.model.query(IsVisibleFrom {
                    target: field.metadata().id(),
                    context: ctx.function_id,
                });
                
                // Only require visible fields to be mentioned
                if is_visible {
                    let is_mentioned = fields.iter()
                        .any(|f| f.name == field.metadata().name().value);
                    
                    if !is_mentioned {
                        ctx.emit(diagnostics::MissingFieldInPattern {
                            span: pattern.span.clone(),
                            field_name: field.metadata().name().value.clone(),
                            suggestion: "use `..` to ignore remaining fields",
                        });
                    }
                }
            }
        }
    }
}
```

---

## Phase 4: Match Expressions

**Goal**: Implement match expression parsing, binding, type inference, and exhaustiveness.

### Syntax

```kestrel
match value {
    pattern => body,
    pattern if guard => body,
    ...
}
```

### Files to Modify

#### 1. Syntax Tree

Add to `SyntaxKind`:

```rust
ExprMatch,         // match expr { arms }
MatchArm,          // pattern (if guard)? => body
MatchArmGuard,     // if condition
```

#### 2. Parser: `lib/kestrel-parser/src/expr/mod.rs`

Add match expression parser:

```rust
fn match_expr_parser<'tokens>(
    expr: impl Parser<...>,
) -> impl Parser<...> {
    just(Token::Match)
        .map_with(|_, e| to_kestrel_span(e.span()))
        .then(expr.clone())  // scrutinee
        .then(
            just(Token::LBrace)
                .map_with(|_, e| to_kestrel_span(e.span()))
        )
        .then(
            match_arm_parser(expr.clone())
                .separated_by(just(Token::Comma).to(()))
                .allow_trailing()
        )
        .then(
            just(Token::RBrace)
                .map_with(|_, e| to_kestrel_span(e.span()))
        )
        .map(|((((match_span, scrutinee), lbrace), arms), rbrace)| {
            ExprVariant::Match { match_span, scrutinee: Box::new(scrutinee), lbrace, arms, rbrace }
        })
}

fn match_arm_parser<'tokens>(
    expr: impl Parser<...>,
) -> impl Parser<...> {
    or_pattern_parser(pattern_parser())  // Use or_pattern for full support
        .then(
            // Optional guard: if expr
            just(Token::If)
                .map_with(|_, e| to_kestrel_span(e.span()))
                .then(expr.clone())
                .or_not()
        )
        .then(just(Token::FatArrow))
        .then(expr.clone())
        .map(|(((pattern, guard), _), body)| {
            MatchArmData { pattern, guard, body }
        })
}
```

#### 3. Semantic Tree: `lib/kestrel-semantic-tree/src/expr.rs`

Add match expression variant:

```rust
/// Match expression: match scrutinee { arms }
Match {
    scrutinee: Box<Expression>,
    arms: Vec<MatchArm>,
},

/// A single arm in a match expression
#[derive(Debug, Clone)]
pub struct MatchArm {
    /// The pattern to match
    pub pattern: Pattern,
    /// Optional guard condition
    pub guard: Option<Box<Expression>>,
    /// The body expression
    pub body: Box<Expression>,
    /// Span of this arm
    pub span: Span,
}
```

#### 4. Binder

Add resolution for match expressions:

```rust
fn resolve_match_expression(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let span = get_node_span(node, ctx.file_id);
    
    // Resolve scrutinee
    let scrutinee = resolve_scrutinee(node, ctx);
    
    // Resolve each arm
    let arms = node.children()
        .filter(|c| c.kind() == SyntaxKind::MatchArm)
        .map(|arm_node| resolve_match_arm(&arm_node, ctx))
        .collect();
    
    // Match result type is union of arm types (inferred)
    let ty = Ty::infer(span.clone());
    
    Expression::match_expr(scrutinee, arms, ty, span)
}

fn resolve_match_arm(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> MatchArm {
    let span = get_node_span(node, ctx.file_id);
    
    // Push new scope for pattern bindings
    ctx.local_scope.push_scope();
    
    let pattern = resolve_pattern(/* pattern node */, ctx);
    
    // Guard can reference pattern bindings
    let guard = /* resolve optional guard expression */;
    
    // Body can reference pattern bindings
    let body = resolve_expression(/* body node */, ctx);
    
    ctx.local_scope.pop_scope();
    
    MatchArm { pattern, guard, body, span }
}
```

#### 5. Type Inference

Generate constraints for match expressions:

```rust
ExprKind::Match { scrutinee, arms } => {
    // Register match type
    ctx.register_type(&expr.ty);
    
    // Generate constraints for scrutinee
    generate_expression_constraints(ctx, scrutinee);
    
    for arm in arms {
        // Pattern type must match scrutinee type
        generate_pattern_constraints(ctx, &arm.pattern);
        ctx.equate(arm.pattern.ty.id(), scrutinee.ty.id(), arm.pattern.span.clone());
        
        // Generate constraints for guard if present
        if let Some(guard) = &arm.guard {
            generate_expression_constraints(ctx, guard);
            // Guard must be Bool
            let bool_ty = Ty::bool(guard.span.clone());
            ctx.equate(guard.ty.id(), bool_ty.id(), guard.span.clone());
        }
        
        // Generate constraints for body
        generate_expression_constraints(ctx, &arm.body);
        
        // All arm bodies must have same type as match result
        ctx.equate(arm.body.ty.id(), expr.ty.id(), arm.body.span.clone());
    }
}
```

---

## Phase 5: If-Let Expressions

**Goal**: Implement if-let for conditional pattern matching.

### Syntax

```kestrel
if let pattern = expr {
    // pattern bindings in scope
} else {
    // no bindings
}

// With chains:
if let .Some(x) = a, let .Some(y) = b, x > y {
    // ...
}
```

### If-Let Chain Scope Management

Bindings from earlier conditions are visible in later conditions and in the then-block:

```rust
fn resolve_if_let_expression(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Expression {
    let span = get_node_span(node, ctx.file_id);
    
    // Push scope for the entire if-let (conditions + then block share this scope)
    ctx.local_scope.push_scope();
    
    // Resolve conditions - bindings accumulate in this scope
    let conditions = resolve_condition_list(&conditions_node, ctx);
    
    // Then block sees all condition bindings
    let then_block = resolve_block(&then_node, ctx);
    
    // Pop the if-let scope (conditions + then bindings are now out of scope)
    ctx.local_scope.pop_scope();
    
    // Else block gets its own fresh scope (no condition bindings visible)
    let else_block = else_node.map(|n| {
        ctx.local_scope.push_scope();
        let block = resolve_block(&n, ctx);
        ctx.local_scope.pop_scope();
        block
    });
    
    let ty = Ty::infer(span.clone());
    Expression::if_let(conditions, then_block, else_block, ty, span)
}

fn resolve_condition_list(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> Vec<IfCondition> {
    node.children()
        .filter(|c| is_condition_node(c))
        .map(|cond_node| resolve_condition(&cond_node, ctx))
        .collect()
}

fn resolve_condition(node: &SyntaxNode, ctx: &mut BodyResolutionContext) -> IfCondition {
    if is_let_condition(node) {
        // Resolve the value FIRST (before binding pattern)
        // This prevents the pattern's bindings from being in scope for the value
        let value_node = /* extract value node */;
        let value = resolve_expression(&value_node, ctx);
        
        // NOW resolve the pattern (creates bindings visible to subsequent conditions)
        let pattern_node = /* extract pattern node */;
        let pattern = resolve_pattern(&pattern_node, ctx);
        
        IfCondition::Let { pattern, value }
    } else {
        // Boolean condition - can reference bindings from earlier let conditions
        let expr = resolve_expression(/* ... */, ctx);
        IfCondition::Bool(expr)
    }
}
```

### Files to Modify

#### 1. Parser

Extend if expression parser to handle `if let`:

```rust
fn if_expr_parser<'tokens>(expr: impl Parser<...>) -> impl Parser<...> {
    // if condition_list block (else block)?
    just(Token::If)
        .ignore_then(condition_list_parser(expr.clone()))
        .then(block_parser(expr.clone()))
        .then(else_branch_parser(expr).or_not())
        .map(/* ... */)
}

fn condition_list_parser<'tokens>(expr: impl Parser<...>) -> impl Parser<...> {
    condition_parser(expr)
        .separated_by(just(Token::Comma))
        .at_least(1)
}

fn condition_parser<'tokens>(expr: impl Parser<...>) -> impl Parser<...> {
    // let pattern = expr
    let let_condition = just(Token::Let)
        .ignore_then(pattern_parser())
        .then_ignore(just(Token::Equals))
        .then(expr.clone())
        .map(|(pattern, value)| Condition::Let { pattern, value });
    
    // Boolean expression
    let bool_condition = expr
        .map(|e| Condition::Bool(e));
    
    let_condition.or(bool_condition)
}
```

#### 2. Semantic Tree

Add if-let condition types:

```rust
/// A condition in an if-let expression
#[derive(Debug, Clone)]
pub enum IfCondition {
    /// let pattern = expr
    Let {
        pattern: Pattern,
        value: Expression,
    },
    /// Boolean expression
    Bool(Expression),
}
```

#### 3. Analysis

If-let patterns can be refutable - that's the point! But we should warn if the pattern is irrefutable:

```rust
// In pattern_check analyzer
fn check_if_let_pattern(pattern: &Pattern, ctx: &mut AnalysisContext) {
    let result = check_irrefutable(pattern, ctx.model());
    if result.is_irrefutable {
        ctx.emit(diagnostics::IrrefutableIfLetPattern {
            span: pattern.span.clone(),
            suggestion: "use `let` instead of `if let`",
        });
    }
}
```

---

## Phase 6: Guard-Let Statements

**Goal**: Implement guard-let for early exit when pattern doesn't match.

### Syntax

```kestrel
guard let pattern = expr else {
    return  // must diverge
}
// pattern bindings in scope for rest of block
```

### Files to Modify

#### 1. Parser: Add guard-let statement

```rust
fn guard_let_parser<'tokens>(expr: impl Parser<...>) -> impl Parser<...> {
    just(Token::Guard)
        .ignore_then(just(Token::Let))
        .ignore_then(pattern_parser())
        .then_ignore(just(Token::Equals))
        .then(expr.clone())
        .then_ignore(just(Token::Else))
        .then(block_parser(expr))
        .map(|((pattern, value), else_block)| {
            StmtVariant::GuardLet { pattern, value, else_block }
        })
}
```

#### 2. Semantic Tree

```rust
/// Guard-let statement
GuardLet {
    pattern: Pattern,
    value: Expression,
    else_block: Block,  // Must diverge
}
```

#### 3. Analysis

Check that else block diverges:

```rust
fn check_guard_let_diverges(else_block: &Block, ctx: &mut AnalysisContext) {
    if !else_block.diverges() {
        ctx.emit(diagnostics::GuardLetElseMustDiverge {
            span: else_block.span.clone(),
        });
    }
}
```

---

## Phase 7: While-Let Expressions

**Goal**: Implement while-let for looping while pattern matches.

### Syntax

```kestrel
while let pattern = expr {
    // pattern bindings in scope
}
```

### Files to Modify

#### 1. Parser

Extend while expression parser:

```rust
fn while_expr_parser<'tokens>(expr: impl Parser<...>) -> impl Parser<...> {
    just(Token::While)
        .ignore_then(
            // while let pattern = expr
            just(Token::Let)
                .ignore_then(pattern_parser())
                .then_ignore(just(Token::Equals))
                .then(expr.clone())
                .map(|(pattern, value)| WhileCondition::Let { pattern, value })
            .or(
                // while expr
                expr.clone()
                    .map(|e| WhileCondition::Bool(e))
            )
        )
        .then(block_parser(expr))
        .map(|(condition, body)| ExprVariant::While { condition, body })
}
```

#### 2. Semantic Tree

```rust
/// While loop condition
#[derive(Debug, Clone)]
pub enum WhileCondition {
    /// while expr
    Bool(Expression),
    /// while let pattern = expr
    Let {
        pattern: Pattern,
        value: Expression,
    },
}
```

---

## Error Codes

| Code | Description | Phase |
|------|-------------|-------|
| E0501 | Non-exhaustive match - missing pattern cases | 4 |
| E0502 | Refutable pattern in irrefutable context | 3 |
| E0503 | Pattern type mismatch | 1 |
| E0504 | Inconsistent bindings in or-pattern | 1b |
| E0505 | Unknown enum case in pattern | 1 |
| E0506 | Missing associated value label | 1 |
| E0507 | Wrong associated value label | 1 |
| W0508 | Unreachable match arm | 4 |
| W0509 | Irrefutable pattern in `if let` | 5 |
| E0510 | Duplicate binding in pattern | 1 |
| E0511 | Inconsistent mutability in or-pattern | 1b |
| E0512 | Guard condition must be Bool | 4 |
| E0513 | Wrong number of elements in tuple pattern | 1 |
| E0514 | Wrong number of associated values in enum pattern | 1 |
| E0515 | Guard-let else block must diverge | 6 |
| W0516 | Unused binding in pattern | 1 |
| E0517 | Empty match on non-Never type | 4 |
| W0518 | Binding name matches enum case | 1 |
| E0519 | Invalid @-pattern (nested @ or wrong left side) | 1b |
| E0520 | Multiple rest patterns in tuple | 1b |
| E0521 | Invalid range pattern bounds (lower > upper) | 1b |
| E0522 | Range pattern type mismatch | 1b |
| E0523 | Range pattern on unsupported type | 1b |
| E0524 | Float literal in pattern (use guard instead) | 1 |
| E0525 | Private field in struct pattern | 1b |
| E0526 | Unknown field in struct pattern | 1b |
| E0527 | Missing fields in struct pattern (use `..` to ignore) | 1b |
| E0528 | Multiple rest patterns in array | 1b |
| W0529 | Overlapping range patterns | 1b |
| W0530 | Duplicate pattern | 4 |

---

## Implementation Order Summary

| Phase | Description | Estimated Effort | Dependencies |
|-------|-------------|------------------|--------------|
| 1a | Simple patterns (wildcard, binding, tuple, literal, enum) + Guard token | 3-4 hours | None |
| 1b | Complex patterns (range, struct, array, rest, or, @) | 4-5 hours | Phase 1a |
| 2 | Replace let/var binding name with pattern | 2-3 hours | Phase 1a |
| 3 | Pattern analysis crate (irrefutability) | 3-4 hours | Phase 2 |
| 4 | Match expressions + exhaustiveness | 4-5 hours | Phase 3 |
| 5 | If-let expressions | 2-3 hours | Phase 2, 3 |
| 6 | Guard-let statements | 2 hours | Phase 2, 3 |
| 7 | While-let expressions | 1-2 hours | Phase 2 |

**Total Estimated Effort:** 21-28 hours

---

## Key Reference Files

| Purpose | File |
|---------|------|
| Pattern matching spec | `docs/pattern-matching.md` |
| Existing pattern types | `lib/kestrel-semantic-tree/src/pattern.rs` |
| Existing literal values | `lib/kestrel-semantic-tree/src/expr.rs` (LiteralValue) |
| Statement parsing | `lib/kestrel-parser/src/stmt/mod.rs` |
| Syntax tree nodes | `lib/kestrel-syntax-tree/src/lib.rs` |
| Expression parsing | `lib/kestrel-parser/src/expr/mod.rs` |
| Body resolver | `lib/kestrel-semantic-tree-binder/src/body_resolver/` |
| Type inference | `lib/kestrel-semantic-type-inference/src/` |
| Constraint solver | `lib/kestrel-semantic-type-inference/src/solver.rs` |
| Visibility queries | `lib/kestrel-semantic-model/src/queries/is_visible_from.rs` |
| Local scope management | `lib/kestrel-semantic-tree-binder/src/resolution/local_scope.rs` |
| Analyzers | `lib/kestrel-semantic-analyzers/src/analyzers/` |
| Enum implementation | `docs/enum-plan.md` (similar patterns) |
| Closure implementation | `docs/closure-plan.md` (similar patterns) |

---

## Testing Strategy

### Unit Tests Per Phase

1. **Phase 1**: Pattern parsing tests, pattern type resolution
2. **Phase 2**: Let/var with patterns, destructuring tests
3. **Phase 3**: Irrefutability tests (positive and negative)
4. **Phase 4**: Match exhaustiveness tests, redundancy detection
5. **Phase 5**: If-let tests with various pattern types
6. **Phase 6**: Guard-let tests, divergence checking
7. **Phase 7**: While-let loop tests

### Test File Location

```
lib/kestrel-test-suite/tests/
├── patterns/
│   ├── simple_patterns.rs      # Phase 1a
│   ├── complex_patterns.rs     # Phase 1b
│   ├── let_destructuring.rs    # Phase 2
│   └── irrefutability.rs       # Phase 3
├── expressions/
│   ├── match_expr.rs           # Phase 4
│   ├── if_let.rs               # Phase 5
│   └── while_let.rs            # Phase 7
└── statements/
    └── guard_let.rs            # Phase 6
```

---

## Algorithm References

1. **Maranget, L. (2007)**: "Warnings for pattern matching" - Journal of Functional Programming
   - Core algorithm for exhaustiveness and usefulness checking
   - Pattern matrix representation and operations

2. **Rust's pattern matching implementation**: `rustc_pattern_analysis` crate
   - Modern implementation of Maranget's algorithm
   - Witness generation for error messages

3. **OCaml compiler**: Pattern matching warnings module
   - Original production implementation of Maranget's work
