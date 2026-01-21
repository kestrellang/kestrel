# Trailing Closures Implementation Plan

This document describes the design and implementation plan for trailing closure syntax in Kestrel.

## Overview

Trailing closures allow closures to be written outside the parentheses of a function call, making code more readable, especially for DSL-style APIs.

## Syntax

```
TrailingClosureArg → (Identifier COLON)? ClosureExpr
TrailingClosures → TrailingClosureArg*
```

### Rules

1. **Must be a closure** - The expression must start with `{`
2. **Label is optional** - Follows the same rules as regular labeled arguments
3. **Multiple trailing closures allowed** - Each can have an optional label
4. **Appears after call or path expression** - Can follow `callee(args)` or just `callee`
5. **Works for functions, methods, and initializers** - Parser doesn't distinguish

### Examples

```kestrel
// Single trailing closure, no label needed
numbers.map { it * 2 }
apply { 42 }

// Single trailing closure with label
apply f: { 42 }

// Multiple trailing closures (labels help disambiguate)
animate { view.alpha = 0 } onComplete: { print("done") }

// Mixed parenthesized args and trailing closures
combine(1, 2) { it * 2 }
fold(0) { (acc, n) in acc + n }
Button(title: "OK") onTap: { save() } onLongPress: { showMenu() }

// Multiple trailing closures without parens
configure onTap: { save() } onLongPress: { showMenu() }
```

### Invalid Syntax

```kestrel
// Non-closure values cannot trail
Button title: "OK" onTap: { save() }  // ERROR: "OK" is not a closure

// This is valid - "OK" is in parens
Button(title: "OK") onTap: { save() }  // OK
```

## Relationship to Labels

Trailing closures follow the same labeling rules as regular arguments:

```kestrel
// Function with no explicit label
func apply(f: () -> Int) -> Int { f() }

// All valid:
apply({ 42 })      // Regular, unlabeled
apply(f: { 42 })   // Regular, labeled
apply { 42 }       // Trailing, unlabeled
apply f: { 42 }    // Trailing, labeled

// Function with explicit label
func perform(using f: () -> Int) -> Int { f() }

// Must use label:
perform(using: { 42 })  // Regular
perform using: { 42 }   // Trailing
```

## Implementation

### Design Decision: Minimal Downstream Changes

Trailing closures are emitted as regular `Argument` nodes within an `ArgumentList`. This means:

- **No new syntax tree node types needed**
- **No binder changes needed**
- **Only parser changes required**

### Data Structure Changes

**File:** `lib/kestrel-parser/src/expr/mod.rs`

Make `lparen` and `rparen` optional in the `Call` variant to support calls with only trailing closures:

```rust
/// Call expression: callee(args) or callee { closure }
Call {
    callee: Box<ExprVariant>,
    /// Left paren (None for trailing-closure-only calls)
    lparen: Option<Span>,
    arguments: Vec<CallArg>,
    commas: Vec<Span>,
    /// Right paren (None for trailing-closure-only calls)
    rparen: Option<Span>,
},
```

### Parser Changes

**File:** `lib/kestrel-parser/src/expr/mod.rs`

#### 1. Create trailing closure argument parser

```rust
/// Trailing closure argument: { closure } or label: { closure }
/// Only matches if the value is a closure (starts with {)
let trailing_closure_arg = skip_trivia()
    .ignore_then(
        // Optional label: identifier followed by colon
        filter_map(|span, token| match token {
            Token::Identifier => Ok(Span::from(span)),
            _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
        })
        .then(
            skip_trivia()
                .ignore_then(just(Token::Colon).map_with_span(|_, span| Span::from(span)))
        )
        .or_not()
    )
    .then(closure_expr.clone())
    .map(|(label_opt, closure)| {
        let (label, colon) = match label_opt {
            Some((l, c)) => (Some(l), Some(c)),
            None => (None, None),
        };
        CallArg { label, colon, value: closure }
    });
```

#### 2. Update postfix operation chain

```rust
let postfix = primary
    .clone()
    .then(postfix_op.repeated())
    .then(trailing_closure_arg.repeated())  // NEW
    .map(|((base, ops), trailing_closures)| {
        let result = ops.into_iter().fold(base, |acc, op| { ... });
        
        if trailing_closures.is_empty() {
            result
        } else {
            attach_trailing_closures(result, trailing_closures)
        }
    });
```

#### 3. Implement `attach_trailing_closures` helper

```rust
fn attach_trailing_closures(
    expr: ExprVariant,
    trailing: Vec<CallArg>,
) -> ExprVariant {
    match expr {
        // Existing call: append trailing closures to arguments
        ExprVariant::Call { callee, lparen, mut arguments, commas, rparen } => {
            arguments.extend(trailing);
            ExprVariant::Call { callee, lparen, arguments, commas, rparen }
        }
        
        // Path becomes a call with no parens
        path @ ExprVariant::Path { .. } => {
            ExprVariant::Call {
                callee: Box::new(path),
                lparen: None,
                arguments: trailing,
                commas: vec![],
                rparen: None,
            }
        }
        
        // MemberAccess becomes a call with no parens
        member @ ExprVariant::MemberAccess { .. } => {
            ExprVariant::Call {
                callee: Box::new(member),
                lparen: None,
                arguments: trailing,
                commas: vec![],
                rparen: None,
            }
        }
        
        // Other expressions can't have trailing closures
        other => other,
    }
}
```

### Emitter Changes

**File:** `lib/kestrel-parser/src/expr/mod.rs`

Update `emit_call_expr` to handle optional parentheses:

```rust
fn emit_call_expr(
    sink: &mut EventSink,
    callee: &ExprVariant,
    lparen: Option<&Span>,
    arguments: &[CallArg],
    commas: &[Span],
    rparen: Option<&Span>,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprCall);
    emit_expr_variant(sink, callee);
    
    sink.start_node(SyntaxKind::ArgumentList);
    
    if let Some(lp) = lparen {
        sink.add_token(SyntaxKind::LParen, lp.clone());
    }
    
    for (i, arg) in arguments.iter().enumerate() {
        sink.start_node(SyntaxKind::Argument);
        if let (Some(label), Some(colon)) = (&arg.label, &arg.colon) {
            sink.add_token(SyntaxKind::Identifier, label.clone());
            sink.add_token(SyntaxKind::Colon, colon.clone());
        }
        emit_expr_variant(sink, &arg.value);
        sink.finish_node();
        
        if i < commas.len() {
            sink.add_token(SyntaxKind::Comma, commas[i].clone());
        }
    }
    
    if let Some(rp) = rparen {
        sink.add_token(SyntaxKind::RParen, rp.clone());
    }
    
    sink.finish_node(); // ArgumentList
    sink.finish_node(); // ExprCall
    sink.finish_node(); // Expression
}
```

## Test Updates

Update the 3 failing trailing closure tests in `lib/kestrel-test-suite/tests/expressions/closures.rs`:

### trailing_closure::trailing_closure_only_argument

```kestrel
func apply(f: () -> Int) -> Int {
    f()
}

func test() -> Int {
    apply { 42 }
}
```

### trailing_closure::trailing_closure_with_other_args

```kestrel
func fold(initial: Int, f: (Int, Int) -> Int) -> Int {
    f(initial, 10)
}

func test() -> Int {
    fold(0) { (acc, n) in acc + n }
}
```

### trailing_closure::trailing_closure_with_multiple_args

```kestrel
func combine(a: Int, b: Int, f: (Int) -> Int) -> Int {
    f(a + b)
}

func test() -> Int {
    combine(1, 2) { it * 2 }
}
```

## Files to Modify

| File | Change |
|------|--------|
| `lib/kestrel-parser/src/expr/mod.rs` | Make `lparen`/`rparen` optional, add trailing closure parsing, update emitter |

## Files NOT Modified

The following require no changes due to the design of emitting trailing closures as regular `Argument` nodes:

- `lib/kestrel-syntax-tree/src/lib.rs` - No new node types
- `lib/kestrel-semantic-tree-binder/` - Existing argument resolution works
- `lib/kestrel-semantic-tree/` - No changes needed
- `lib/kestrel-semantic-type-inference/` - No changes needed
- `lib/kestrel-semantic-analyzers/` - No changes needed

## Edge Cases

### Ambiguity with Loop Labels

`foo: { ... }` at statement level is a labeled loop. But trailing closures only appear after:
- A path expression (function name)
- A call expression (with parens)
- A member access expression

So context disambiguates:
- `foo: { ... }` alone = labeled block/loop
- `bar foo: { ... }` = call with labeled trailing closure

### Chained Calls

```kestrel
foo().bar { 42 }
```

The `{ 42 }` attaches to the `.bar` call, producing `foo().bar({ 42 })`.

### Binary Operators

```kestrel
apply { 42 } + 1
```

Trailing closures bind tightly, so this is `(apply { 42 }) + 1`.

## Status

- [x] Phase 1: Update `Call` variant to have optional parens
- [x] Phase 2: Add trailing closure parser
- [x] Phase 3: Update postfix chain to collect trailing closures
- [x] Phase 4: Implement `attach_trailing_closures`
- [x] Phase 5: Update emitter for optional parens
- [x] Phase 6: Run tests and fix any issues

**COMPLETED: 2025-12-18**

All 4 trailing closure tests pass:
- `trailing_closure_only_argument`
- `trailing_closure_with_other_args`
- `trailing_closure_with_multiple_args`
- `non_trailing_closure_in_parens`
