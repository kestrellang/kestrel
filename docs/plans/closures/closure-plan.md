# Closure Implementation Plan

This document provides a practical, step-by-step plan for implementing closures in the Kestrel compiler. The implementation is broken into phases that can be completed incrementally, with each phase building on the previous.

## Overview

Closures are anonymous functions that can capture variables from their enclosing scope. The full specification is in `docs/semantics/closures.md`.

**Key Syntax:**
```kestrel
{ 42 }                          // No params
{ it * 2 }                      // Implicit `it` parameter
{ (x, y) in x + y }             // Explicit params
{ (x: Int) in x * 2 }           // With type annotations
numbers.map { it * 2 }          // Trailing closure
```

**Test Suite:** 72 tests in `lib/kestrel-test-suite/tests/expressions/closures.rs`

---

## Phase 1: Lexer - Add `in` Keyword

**Goal:** Add the `in` keyword token used for closure parameter lists.

### Files to Modify

#### `lib/kestrel-lexer/src/lib.rs`

Add the `In` token to the `Token` enum:

```rust
// In the Statement Keywords section (around line 161)
#[token("in")]
In,
```

**Location:** After `Token::If` in the Statement Keywords section (line ~179).

### Verification

```bash
cargo test -p kestrel-lexer
```

---

## Phase 2: Syntax Tree - Add Closure Nodes

**Goal:** Add syntax node kinds for closure expressions.

### Files to Modify

#### `lib/kestrel-syntax-tree/src/lib.rs`

1. **Add expression node** (after `ExprTupleIndex`, around line 130):

```rust
ExprClosure,      // { params in body } or { body }
ClosureParams,    // (param, param) in closure
ClosureParam,     // Single closure parameter: name or name: Type
```

2. **Add `In` token** (after `While`, around line 170):

```rust
In,               // The `in` keyword for closure params
```

3. **Add token conversion** in `From<Token>` impl (around line 274):

```rust
Token::In => SyntaxKind::In,
```

4. **Add constants and match arms** in `kind_from_raw` (around line 400-635):

```rust
// Add constants
const EXPR_CLOSURE: u16 = SyntaxKind::ExprClosure as u16;
const CLOSURE_PARAMS: u16 = SyntaxKind::ClosureParams as u16;
const CLOSURE_PARAM: u16 = SyntaxKind::ClosureParam as u16;
const IN: u16 = SyntaxKind::In as u16;

// Add match arms
EXPR_CLOSURE => SyntaxKind::ExprClosure,
CLOSURE_PARAMS => SyntaxKind::ClosureParams,
CLOSURE_PARAM => SyntaxKind::ClosureParam,
IN => SyntaxKind::In,
```

### Verification

```bash
cargo test -p kestrel-syntax-tree
cargo build -p kestrel-syntax-tree
```

---

## Phase 3: Parser - Parse Closure Expressions

**Goal:** Parse closure syntax into the syntax tree.

### Files to Modify

#### `lib/kestrel-parser/src/expr/mod.rs`

1. **Add `Closure` variant to `ExprVariant`** (around line 293):

```rust
/// Closure expression: { params in body } or { body }
Closure {
    lbrace: Span,
    params: Option<ClosureParamsData>,
    in_span: Option<Span>,
    body: Vec<BlockItem>,
    rbrace: Span,
},
```

2. **Add `ClosureParamsData` struct** (after `LabelData`, around line 305):

```rust
/// Closure parameter list data: (x, y: Int)
#[derive(Debug, Clone)]
pub struct ClosureParamsData {
    pub lparen: Span,
    pub params: Vec<ClosureParamData>,
    pub commas: Vec<Span>,
    pub rparen: Span,
}

/// Single closure parameter: name or name: Type
#[derive(Debug, Clone)]
pub struct ClosureParamData {
    pub name: Span,
    pub colon: Option<Span>,
    pub ty: Option<TyVariant>,
}
```

3. **Add closure parser** (new function, around line 600):

```rust
/// Parser for closure expressions: { body } or { (params) in body }
fn closure_parser(
    expr: impl Parser<Token, ExprVariant, Error = Simple<Token>> + Clone,
) -> impl Parser<Token, ExprVariant, Error = Simple<Token>> + Clone {
    // Closure parameter list: (x, y: Type)
    let closure_param = skip_trivia()
        .ignore_then(just(Token::Identifier).map_with_span(|_, span| Span::from(span)))
        .then(
            skip_trivia()
                .ignore_then(just(Token::Colon).map_with_span(|_, span| Span::from(span)))
                .then(ty_parser())
                .or_not(),
        )
        .map(|(name, ty_opt)| {
            let (colon, ty) = match ty_opt {
                Some((c, t)) => (Some(c), Some(t)),
                None => (None, None),
            };
            ClosureParamData { name, colon, ty }
        });

    let closure_params = skip_trivia()
        .ignore_then(just(Token::LParen).map_with_span(|_, span| Span::from(span)))
        .then(
            closure_param
                .clone()
                .separated_by(skip_trivia().ignore_then(just(Token::Comma)))
                .allow_trailing(),
        )
        .then_ignore(skip_trivia())
        .then(just(Token::RParen).map_with_span(|_, span| Span::from(span)))
        .then_ignore(skip_trivia())
        .then(just(Token::In).map_with_span(|_, span| Span::from(span)))
        .map(|(((lparen, params), rparen), in_span)| {
            // Extract commas (placeholder - actual implementation would track them)
            let commas = vec![];
            (
                Some(ClosureParamsData { lparen, params, commas, rparen }),
                Some(in_span),
            )
        });

    // Empty params with in: () in
    let empty_params = skip_trivia()
        .ignore_then(just(Token::LParen).map_with_span(|_, span| Span::from(span)))
        .then_ignore(skip_trivia())
        .then(just(Token::RParen).map_with_span(|_, span| Span::from(span)))
        .then_ignore(skip_trivia())
        .then(just(Token::In).map_with_span(|_, span| Span::from(span)))
        .map(|((lparen, rparen), in_span)| {
            (
                Some(ClosureParamsData {
                    lparen,
                    params: vec![],
                    commas: vec![],
                    rparen,
                }),
                Some(in_span),
            )
        });

    // The closure body - reuse block item parsing
    let body_items = block_item_parser(expr.clone())
        .repeated();

    // Full closure: { [params in] body }
    skip_trivia()
        .ignore_then(just(Token::LBrace).map_with_span(|_, span| Span::from(span)))
        .then(closure_params.or(empty_params).or_not().map(|opt| opt.unwrap_or((None, None))))
        .then(body_items)
        .then_ignore(skip_trivia())
        .then(just(Token::RBrace).map_with_span(|_, span| Span::from(span)))
        .map(|(((lbrace, (params, in_span)), body), rbrace)| {
            ExprVariant::Closure {
                lbrace,
                params,
                in_span,
                body,
                rbrace,
            }
        })
}
```

4. **Add closure to primary expressions** (in `expr_parser`, around line 800):

Add closure parsing to the primary expression alternatives. Closures start with `{`, so they need to be distinguished from code blocks (which only appear in specific contexts like `if`/`while`/`loop`).

```rust
// In the primary expression chain, add:
let closure = closure_parser(expr.clone());

// Add to the choice of primary expressions:
let primary = float
    .or(integer)
    // ... existing alternatives ...
    .or(closure)  // Add closure parsing
    // ...
```

5. **Add `emit_closure_expr` function** (around line 1200):

```rust
fn emit_closure_expr(sink: &mut EventSink, data: &ExprVariant) {
    if let ExprVariant::Closure { lbrace, params, in_span, body, rbrace } = data {
        sink.start_node(SyntaxKind::ExprClosure);
        sink.token(SyntaxKind::LBrace, lbrace);

        // Emit params if present
        if let Some(params_data) = params {
            sink.start_node(SyntaxKind::ClosureParams);
            sink.token(SyntaxKind::LParen, &params_data.lparen);

            for (i, param) in params_data.params.iter().enumerate() {
                if i > 0 && i <= params_data.commas.len() {
                    sink.token(SyntaxKind::Comma, &params_data.commas[i - 1]);
                }
                sink.start_node(SyntaxKind::ClosureParam);
                sink.token(SyntaxKind::Identifier, &param.name);
                if let Some(ref colon) = param.colon {
                    sink.token(SyntaxKind::Colon, colon);
                }
                if let Some(ref ty) = param.ty {
                    emit_ty_variant(sink, ty);
                }
                sink.finish_node(); // ClosureParam
            }

            sink.token(SyntaxKind::RParen, &params_data.rparen);
            sink.finish_node(); // ClosureParams
        }

        // Emit `in` keyword
        if let Some(ref in_sp) = in_span {
            sink.token(SyntaxKind::In, in_sp);
        }

        // Emit body items
        for item in body {
            emit_block_item(sink, item);
        }

        sink.token(SyntaxKind::RBrace, rbrace);
        sink.finish_node(); // ExprClosure
    }
}
```

6. **Update `emit_expr_variant`** to handle `Closure`:

```rust
// In emit_expr_variant match:
ExprVariant::Closure { .. } => emit_closure_expr(sink, variant),
```

7. **Add method to `Expression`** (around line 136):

```rust
/// Check if this is a closure expression
pub fn is_closure(&self) -> bool {
    self.kind() == SyntaxKind::ExprClosure
}
```

### Trailing Closure Syntax

Modify call expression parsing to support trailing closures:

```rust
// After parsing a call's closing paren, check for trailing closure
let call_with_trailing = call_expr
    .then(closure_parser(expr.clone()).or_not())
    .map(|(call, trailing)| {
        match trailing {
            Some(closure) => {
                // Add closure as final argument
                // ... modify call to include closure
            }
            None => call,
        }
    });
```

### Verification

```bash
cargo test -p kestrel-parser
```

Write a simple test:

```rust
#[test]
fn test_closure_parsing() {
    let source = "{ 42 }";
    let result = parse_expression(source);
    assert!(result.is_closure());
}

#[test]
fn test_closure_with_params() {
    let source = "{ (x: Int) in x * 2 }";
    let result = parse_expression(source);
    assert!(result.is_closure());
}
```

---

## Phase 4: Semantic Tree - Represent Closures

**Goal:** Add semantic representation for closures.

### Files to Modify

#### `lib/kestrel-semantic-tree/src/expr.rs`

1. **Add `Closure` variant to `ExprKind`** (around line 500):

```rust
/// Closure expression: { params in body }
Closure {
    /// Explicit parameters, if any. None means implicit `it` style.
    params: Option<Vec<ClosureParam>>,
    /// Statements in the closure body
    body: Vec<crate::stmt::Statement>,
    /// Final expression (implicit return value)
    tail_expr: Option<Box<Expression>>,
    /// Variables captured from enclosing scope (filled by capture analysis)
    captures: Vec<Capture>,
},
```

2. **Add supporting types** (after `ExprKind`, around line 510):

```rust
/// A closure parameter.
#[derive(Debug, Clone)]
pub struct ClosureParam {
    /// Parameter name
    pub name: String,
    /// Parameter type (may be inferred initially)
    pub ty: Ty,
    /// Whether the type was explicitly annotated
    pub is_type_annotated: bool,
    /// Source span
    pub span: Span,
}

/// A captured variable from an enclosing scope.
#[derive(Debug, Clone)]
pub struct Capture {
    /// The local variable ID being captured
    pub local_id: LocalId,
    /// Name of the captured variable
    pub name: String,
    /// Type of the captured variable
    pub ty: Ty,
    /// How the variable is captured
    pub kind: CaptureKind,
    /// Span where the capture occurs
    pub span: Span,
}

/// How a variable is captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureKind {
    /// Immutable copy (only option currently)
    Value,
}
```

3. **Add constructor method** (after other constructors, around line 1100):

```rust
/// Create a closure expression.
pub fn closure(
    params: Option<Vec<ClosureParam>>,
    body: Vec<crate::stmt::Statement>,
    tail_expr: Option<Expression>,
    captures: Vec<Capture>,
    ty: Ty,
    span: Span,
) -> Self {
    Expression {
        id: ExprId::new(),
        kind: ExprKind::Closure {
            params,
            body,
            tail_expr: tail_expr.map(Box::new),
            captures,
        },
        ty,
        span,
        mutable: false,
    }
}
```

4. **Update `debug_compact`** to handle closures:

```rust
ExprKind::Closure { params, tail_expr, .. } => {
    let params_str = match params {
        Some(ps) => {
            let p: Vec<_> = ps.iter().map(|p| p.name.clone()).collect();
            format!("({}) in", p.join(", "))
        }
        None => String::new(),
    };
    let body_str = tail_expr
        .as_ref()
        .map(|e| e.debug_compact())
        .unwrap_or_else(|| "...".to_string());
    format!("{{ {}{} }}", params_str, body_str)
}
```

### Verification

```bash
cargo test -p kestrel-semantic-tree
cargo build -p kestrel-semantic-tree
```

---

## Phase 5: Binder - Resolve Closures

**Goal:** Resolve closure syntax into semantic expressions.

### Files to Modify

#### `lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs`

1. **Add case in `resolve_expression`** (around line 100):

```rust
SyntaxKind::ExprClosure => resolve_closure_expression(expr_node, ctx),
```

2. **Add `resolve_closure_expression` function**:

```rust
/// Resolve a closure expression.
fn resolve_closure_expression(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Expression {
    let span = get_node_span(node, ctx.file_id);

    // Push a new scope for the closure
    ctx.local_scope.push_scope();

    // Parse parameters from ClosureParams node
    let (params, has_explicit_params) = resolve_closure_params(node, ctx);

    // If no explicit params and expected type has arity 1, inject `it`
    let inject_it = !has_explicit_params; // Will be refined with expected type later

    // Resolve the closure body (statements and trailing expression)
    let (body, tail_expr) = resolve_closure_body(node, ctx);

    // Perform capture analysis
    let captures = perform_capture_analysis(ctx);

    // Pop the closure scope
    ctx.local_scope.pop_scope();

    // Determine closure type
    let param_types: Vec<Ty> = params
        .as_ref()
        .map(|ps| ps.iter().map(|p| p.ty.clone()).collect())
        .unwrap_or_default();

    let return_ty = tail_expr
        .as_ref()
        .map(|e| e.ty.clone())
        .unwrap_or_else(|| Ty::unit(span.clone()));

    let closure_ty = Ty::function(param_types, return_ty, span.clone());

    Expression::closure(params, body, tail_expr, captures, closure_ty, span)
}

/// Resolve closure parameters from the syntax tree.
fn resolve_closure_params(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> (Option<Vec<ClosureParam>>, bool) {
    // Find ClosureParams node
    let params_node = node
        .children()
        .find(|c| c.kind() == SyntaxKind::ClosureParams);

    match params_node {
        Some(pn) => {
            let mut params = Vec::new();
            for child in pn.children() {
                if child.kind() == SyntaxKind::ClosureParam {
                    if let Some(param) = resolve_single_closure_param(&child, ctx) {
                        // Bind parameter as local
                        ctx.local_scope.bind(
                            param.name.clone(),
                            param.ty.clone(),
                            false, // closure params are immutable
                            param.span.clone(),
                        );
                        params.push(param);
                    }
                }
            }
            (Some(params), true)
        }
        None => (None, false),
    }
}

/// Resolve a single closure parameter.
fn resolve_single_closure_param(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> Option<ClosureParam> {
    let span = get_node_span(node, ctx.file_id);

    // Extract name
    let name = node
        .children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| t.text().to_string())?;

    // Extract optional type annotation
    let ty_node = node.children().find(|c| c.kind() == SyntaxKind::Ty);
    let (ty, is_annotated) = match ty_node {
        Some(tn) => {
            let resolved_ty = resolve_type(&tn, ctx);
            (resolved_ty, true)
        }
        None => (Ty::infer(span.clone()), false),
    };

    Some(ClosureParam {
        name,
        ty,
        is_type_annotated: is_annotated,
        span,
    })
}

/// Resolve the body of a closure.
fn resolve_closure_body(
    node: &SyntaxNode,
    ctx: &mut BodyResolutionContext,
) -> (Vec<Statement>, Option<Expression>) {
    let mut statements = Vec::new();
    let mut tail_expr = None;

    // Collect children after ClosureParams and `in` keyword
    let children: Vec<_> = node.children().collect();
    let body_start = children
        .iter()
        .position(|c| c.kind() == SyntaxKind::ClosureParams)
        .map(|i| i + 1)
        .unwrap_or(0);

    for (i, child) in children[body_start..].iter().enumerate() {
        let is_last = i == children[body_start..].len() - 1;

        match child.kind() {
            SyntaxKind::Statement | SyntaxKind::ExpressionStatement => {
                if let Some(stmt) = resolve_statement(child, ctx) {
                    statements.push(stmt);
                }
            }
            SyntaxKind::VariableDeclaration => {
                if let Some(stmt) = resolve_variable_declaration(child, ctx) {
                    statements.push(stmt);
                }
            }
            SyntaxKind::Expression => {
                if is_last && !has_trailing_semicolon(child) {
                    tail_expr = Some(resolve_expression(child, ctx));
                } else {
                    let expr = resolve_expression(child, ctx);
                    let stmt_span = get_node_span(child, ctx.file_id);
                    statements.push(Statement::expr(expr, stmt_span));
                }
            }
            _ if is_expression_kind(child.kind()) => {
                if is_last {
                    tail_expr = Some(resolve_expression(child, ctx));
                } else {
                    let expr = resolve_expression(child, ctx);
                    let stmt_span = get_node_span(child, ctx.file_id);
                    statements.push(Statement::expr(expr, stmt_span));
                }
            }
            _ => {}
        }
    }

    (statements, tail_expr)
}
```

#### `lib/kestrel-semantic-tree-binder/src/body_resolver/captures.rs` (new file)

```rust
//! Capture analysis for closures.

use kestrel_semantic_tree::expr::Capture;
use super::context::BodyResolutionContext;

/// Perform capture analysis for the current closure scope.
///
/// This identifies variables referenced inside the closure that are
/// defined in an enclosing scope (not parameters or locals of this closure).
pub fn perform_capture_analysis(ctx: &BodyResolutionContext) -> Vec<Capture> {
    // TODO: Implement capture tracking
    // For now, return empty - captures will be added in a later iteration
    Vec::new()
}
```

### Verification

```bash
cargo test -p kestrel-semantic-tree-binder
```

---

## Phase 6: Type Inference - Infer Closure Types

**Goal:** Support type inference for closure parameters and return types.

### Files to Modify

#### `lib/kestrel-semantic-type-inference/src/constraint_generator.rs`

1. **Add case for closures** in `generate_expression_constraints`:

```rust
ExprKind::Closure { params, body, tail_expr, .. } => {
    // Register the closure type
    ctx.register_type(&expr.ty);

    // Generate constraints for body statements
    for stmt in body {
        generate_statement_constraints(ctx, stmt);
    }

    // Generate constraints for tail expression
    if let Some(tail) = tail_expr {
        generate_expression_constraints(ctx, tail);
    }

    // If we have an expected function type, use it to infer param types
    // This is handled during constraint solving via bidirectional inference
}
```

### Bidirectional Type Inference

For closures like `{ (x) in x + 1 }` where `x`'s type is not annotated, we need to infer it from context:

```rust
// When resolving a closure in a context that expects (Int) -> Int:
// 1. The expected type tells us x should be Int
// 2. We propagate this to the parameter
// 3. The return type is inferred from the body
```

This requires passing expected type context into closure resolution. Modify `resolve_expression` to accept an optional expected type hint.

### Verification

```bash
cargo test -p kestrel-semantic-type-inference
```

---

## Phase 7: Semantic Analyzers - Validate Closures

**Goal:** Add error detection for closure-specific issues.

### Files to Create

#### `lib/kestrel-semantic-analyzers/src/analyzers/closure/mod.rs`

```rust
//! Closure semantic analysis.

mod diagnostics;

pub use diagnostics::*;

use kestrel_semantic_tree::expr::{ExprKind, Expression};
use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

pub struct ClosureAnalyzer;

impl Analyzer for ClosureAnalyzer {
    fn visit_expression(&mut self, expr: &Expression, ctx: &mut AnalysisContext) {
        if let ExprKind::Closure { params, captures, .. } = &expr.kind {
            // Check for `it` usage errors
            // Check for captured variable mutation attempts
            // Check for closure parameter mutation attempts
        }
    }
}
```

#### `lib/kestrel-semantic-analyzers/src/analyzers/closure/diagnostics.rs`

```rust
//! Closure-specific diagnostic errors.

use kestrel_reporting::IntoDiagnostic;
use kestrel_span::Span;
use codespan_reporting::diagnostic::{Diagnostic, Label};

/// Error: `it` used but closure arity is not 1
pub struct ItUsedWithWrongArityError {
    pub span: Span,
    pub expected_arity: usize,
}

impl IntoDiagnostic for ItUsedWithWrongArityError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "`it` can only be used when closure has exactly 1 parameter, but {} expected",
                self.expected_arity
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("used here")
            ])
    }
}

/// Error: `it` used with explicit parameters
pub struct ItNotInScopeError {
    pub span: Span,
}

impl IntoDiagnostic for ItNotInScopeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("`it` is not in scope; closure has explicit parameters")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
            ])
            .with_notes(vec![
                "use the explicit parameter name instead".to_string()
            ])
    }
}

/// Error: Cannot assign to captured variable
pub struct CannotAssignToCapturedVariableError {
    pub span: Span,
    pub name: String,
}

impl IntoDiagnostic for CannotAssignToCapturedVariableError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("cannot assign to captured variable `{}`", self.name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
            ])
            .with_notes(vec![
                "captures are by value and immutable".to_string()
            ])
    }
}

/// Error: Cannot assign to closure parameter
pub struct CannotAssignToClosureParameterError {
    pub span: Span,
    pub name: String,
}

impl IntoDiagnostic for CannotAssignToClosureParameterError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("cannot assign to immutable parameter `{}`", self.name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
            ])
    }
}

/// Error: Cannot infer closure parameter type
pub struct CannotInferClosureParameterTypeError {
    pub span: Span,
    pub name: String,
}

impl IntoDiagnostic for CannotInferClosureParameterTypeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "cannot infer type for closure parameter `{}`; add a type annotation",
                self.name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
            ])
    }
}

/// Error: Closure arity mismatch
pub struct ClosureArityMismatchError {
    pub span: Span,
    pub actual: usize,
    pub expected: usize,
}

impl IntoDiagnostic for ClosureArityMismatchError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "closure has {} parameters but {} expected",
                self.actual, self.expected
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
            ])
    }
}

/// Error: Closure return type mismatch
pub struct ClosureReturnTypeMismatchError {
    pub span: Span,
    pub actual: String,
    pub expected: String,
}

impl IntoDiagnostic for ClosureReturnTypeMismatchError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "closure returns `{}` but `{}` expected",
                self.actual, self.expected
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
            ])
    }
}
```

### Register Analyzer

In `lib/kestrel-semantic-analyzers/src/analyzers/mod.rs`:

```rust
pub mod closure;
```

In the runner, add:

```rust
runners.push(Box::new(closure::ClosureAnalyzer));
```

### Verification

```bash
cargo test -p kestrel-semantic-analyzers
```

---

## Phase 8: Handle `it` Parameter

**Goal:** Implement the implicit `it` parameter for closures.

### Implementation Strategy

1. When resolving a closure without explicit params:
   - Check if expected type is known and has arity 1
   - If so, inject `it` as an implicit parameter
   - Track whether `it` was actually used in the body

2. When a path resolves to `it`:
   - Check if we're in a closure context
   - Check if the closure has explicit params (error if so)
   - Check if the expected arity is 1 (error if not and `it` is used)

### Modifications

In `resolve_closure_expression`:

```rust
// After determining we have no explicit params:
let expected_arity = ctx.expected_type()
    .and_then(|t| t.as_function())
    .map(|(params, _)| params.len());

if expected_arity == Some(1) {
    // Inject `it` parameter
    let it_ty = ctx.expected_type()
        .and_then(|t| t.as_function())
        .map(|(params, _)| params[0].clone())
        .unwrap_or_else(|| Ty::infer(span.clone()));

    ctx.local_scope.bind("it".to_string(), it_ty, false, span.clone());
}
```

### Verification

Run the `it` parameter tests:

```bash
cargo test -p kestrel-test-suite closure::implicit_it
```

---

## Phase 9: Trailing Closure Syntax

**Goal:** Support `func { closure }` and `func(args) { closure }` syntax.

### Implementation

In the call expression parser, after parsing a standard call:

```rust
// Check for trailing closure
let trailing = closure_parser(expr.clone()).or_not();

// Combine with call
call.then(trailing).map(|(call, trailing_closure)| {
    match trailing_closure {
        Some(closure) => {
            // Add closure as the last argument to the call
            // Create new Call variant with additional argument
        }
        None => call
    }
})
```

Also handle the case where the closure is the only argument:

```rust
// func { closure } - no parens needed
path.then(closure_parser(expr.clone())).map(|(callee, closure)| {
    ExprVariant::Call {
        callee: Box::new(callee),
        lparen: closure.lbrace, // Use closure's span
        arguments: vec![CallArg { label: None, colon: None, value: closure }],
        commas: vec![],
        rparen: closure.rbrace,
    }
})
```

### Verification

```bash
cargo test -p kestrel-test-suite closure::trailing_closure
```

---

## Phase 10: Capture Analysis

**Goal:** Implement proper capture analysis to track which variables are captured.

### Implementation

Track variable references during body resolution:

```rust
// In BodyResolutionContext, add:
pub struct ClosureContext {
    /// Variables referenced that were defined before this closure
    pub referenced_outer_vars: Vec<(LocalId, String, Ty, Span)>,
    /// Scope depth when closure started
    pub closure_scope_depth: usize,
}

// When resolving a LocalRef inside a closure:
if let Some(closure_ctx) = &mut ctx.closure_context {
    if local_scope_depth < closure_ctx.closure_scope_depth {
        // This is a capture!
        closure_ctx.referenced_outer_vars.push((local_id, name, ty, span));
    }
}
```

### Verification

```bash
cargo test -p kestrel-test-suite closure::captures
```

---

## Testing Strategy

### Unit Tests

Each phase should have unit tests for the specific functionality:

1. **Lexer:** Test `in` token is recognized
2. **Parser:** Test various closure forms parse correctly
3. **Binder:** Test closures resolve to semantic expressions
4. **Type Inference:** Test closure types are inferred correctly
5. **Analyzers:** Test error messages are produced

### Integration Tests

Run the full test suite after each phase:

```bash
cargo test -p kestrel-test-suite
```

### Closure-Specific Tests

The 72 tests in `lib/kestrel-test-suite/tests/expressions/closures.rs` should progressively pass as phases are completed:

| Phase | Expected Tests Passing |
|-------|------------------------|
| 1-3   | Parser tests (structure only) |
| 4-5   | Basic syntax tests |
| 6     | Type inference tests |
| 7     | Error tests |
| 8     | `it` parameter tests |
| 9     | Trailing closure tests |
| 10    | Capture tests |

---

## Implementation Order Summary

| Phase | Component | Estimated Effort | Dependencies |
|-------|-----------|------------------|--------------|
| 1 | Lexer - `in` keyword | 15 min | None |
| 2 | Syntax Tree - nodes | 30 min | Phase 1 |
| 3 | Parser - parsing | 2-3 hours | Phase 2 |
| 4 | Semantic Tree - types | 1 hour | Phase 3 |
| 5 | Binder - resolution | 2-3 hours | Phase 4 |
| 6 | Type Inference | 2-3 hours | Phase 5 |
| 7 | Analyzers - errors | 2 hours | Phase 5 |
| 8 | `it` parameter | 2 hours | Phase 6 |
| 9 | Trailing closures | 1-2 hours | Phase 3 |
| 10 | Capture analysis | 2-3 hours | Phase 5 |

**Total Estimated Effort:** 15-20 hours

---

## Key Reference Files

| Purpose | File |
|---------|------|
| Closure spec | `docs/semantics/closures.md` |
| Test suite | `lib/kestrel-test-suite/tests/expressions/closures.rs` |
| Lexer | `lib/kestrel-lexer/src/lib.rs` |
| Syntax tree | `lib/kestrel-syntax-tree/src/lib.rs` |
| Parser | `lib/kestrel-parser/src/expr/mod.rs` |
| Semantic expressions | `lib/kestrel-semantic-tree/src/expr.rs` |
| Expression binder | `lib/kestrel-semantic-tree-binder/src/body_resolver/expressions.rs` |
| Type inference | `lib/kestrel-semantic-type-inference/src/constraint_generator.rs` |
| Analyzers | `lib/kestrel-semantic-analyzers/src/analyzers/` |

---

## Similar Implementations to Reference

| Feature | Pattern To Follow |
|---------|-------------------|
| If expressions | Block body, scoping, multiple branches |
| While/Loop | Body resolution, scope management |
| Function parameters | Parameter parsing, type annotation |
| Function calls | Argument handling, trailing syntax |
| Block parsing | Statement + trailing expression |
