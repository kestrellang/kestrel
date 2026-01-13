# Try Operator Design

This document describes the `try` operator for early return propagation in Kestrel.

## Overview

The `try` operator provides ergonomic error/early-return propagation. It extracts the success value from a `Tryable` type, or returns early from the enclosing function with the error/early value.

```kestrel
func processFile(path: String) -> Result[Data, Error] {
    let file = try openFile(path)    // Returns early if Err
    let data = try file.read()       // Returns early if Err
    .Ok(data)
}
```

## Core Types

### ControlFlow Enum

The `ControlFlow` enum represents the two possible outcomes of a try extraction:

```kestrel
@builtin(.ControlFlowEnum)
public enum ControlFlow[Continue, Break] {
    case Continue(Continue)  // Continue execution with value
    case Break(Break)        // Break/return early with value
}
```

### Tryable Protocol

Types that can be "tried" conform to `Tryable`:

```kestrel
@builtin(.TryableProtocol)
public protocol Tryable {
    type Output   // The success value type
    type Early    // The early-return value type

    @builtin(.TryExtractMethod)
    func tryExtract() -> ControlFlow[Output, Early]
}
```

### FromResidual Protocol

Types that can be constructed from an early-return value conform to `FromResidual`:

```kestrel
@builtin(.FromResidualProtocol)
public protocol FromResidual[Early] {
    @builtin(.FromResidualMethod)
    static func fromResidual(residual: Early) -> Self
}
```

## Desugaring

The expression `try expr` desugars to:

```kestrel
match expr.tryExtract() {
    .Continue(value) => value,
    .Break(early) => return R.fromResidual(early)
}
```

Where `R` is the enclosing function's return type.

### Type Constraints

For `try expr` to be valid:
1. `expr` must conform to `Tryable`
2. The enclosing function's return type `R` must conform to `FromResidual[expr.Early]`

If either constraint fails, a type error is produced.

## Precedence

`try` is a **high-precedence prefix operator**, binding tightly to the immediate expression:

```kestrel
try foo() + bar()      // Parses as: (try foo()) + bar()
try (foo() + bar())    // try applies to the sum
```

## Standard Library Conformances

### Result

```kestrel
extend Result[T, E]: Tryable {
    type Output = T
    type Early = E

    func tryExtract() -> ControlFlow[T, E] {
        match self {
            .Ok(value) => .Continue(value),
            .Err(error) => .Break(error)
        }
    }
}

extend Result[T, E]: FromResidual[E] {
    static func fromResidual(residual: E) -> Result[T, E] {
        .Err(residual)
    }
}
```

### Optional

```kestrel
extend Optional[T]: Tryable {
    type Output = T
    type Early = Unit

    func tryExtract() -> ControlFlow[T, Unit] {
        match self {
            .Some(value) => .Continue(value),
            .None => .Break(())
        }
    }
}

extend Optional[T]: FromResidual[Unit] {
    static func fromResidual(residual: Unit) -> Optional[T] {
        .None
    }
}
```

## Closures

When `try` appears inside a closure, it returns from the **closure**, not the outer function:

```kestrel
func outer() -> Result[Int, String] {
    let items = [1, 2, 3]

    // try inside closure returns from the closure
    let results = items.map { item =>
        try mightFail(item)  // Returns Result from closure if error
    }

    // results is Array[Result[Int, String]] if mightFail returns Result
    .Ok(results.sum())
}
```

For `try` to work in a closure, the closure's return type must conform to `FromResidual[Early]`.

## Cross-Type Try

You can `try` one type inside a function returning a different type, **if** the return type conforms to `FromResidual` for the inner type's `Early`:

```kestrel
// This works if Result[T, String] conforms to FromResidual[Unit]
func foo() -> Result[Int, String] {
    let x = try someOptional()  // Optional.Early = Unit
    .Ok(x)
}

// Add this conformance to enable the above:
extend Result[T, E]: FromResidual[Unit] {
    static func fromResidual(residual: Unit) -> Result[T, E] {
        .Err(/* some default error */)
    }
}
```

Without the conformance, the type system produces an error.

## Error Messages

When `try` fails to type-check, error messages should be user-friendly:

```
Error: Cannot use `try` on `Result[Int, CustomError]` here
       Function returns `Result[Int, String]` which does not conform to `FromResidual[CustomError]`
Hint: Convert the error type before using `try`, or add a FromResidual conformance
```

## Builtins

Add to `LanguageFeature` enum in `builtins.rs`:

```rust
// Try operator
ControlFlowEnum,
TryableProtocol,
TryExtractMethod,
FromResidualProtocol,
FromResidualMethod,
```

## Implementation Steps

### 1. Add Builtins

Add the five new `LanguageFeature` variants to `builtins.rs`:
- `ControlFlowEnum`
- `TryableProtocol`
- `TryExtractMethod`
- `FromResidualProtocol`
- `FromResidualMethod`

### 2. Add `try` Keyword

Add `try` as a reserved keyword in the lexer.

### 3. Parse Try Expression

In the expression parser, handle `try` as a prefix operator with high precedence:

```rust
// In expression parsing
if current_token == Try {
    let operand = parse_expression(TRY_PRECEDENCE);
    return TryExpression { operand };
}
```

### 4. Add TryExpression to AST

Add a new expression variant:

```rust
pub enum Expression {
    // ...
    Try(TryExpression),
}

pub struct TryExpression {
    pub operand: Box<Expression>,
    pub span: Span,
}
```

### 5. Desugar in Body Resolver

In the body resolver, desugar `TryExpression` to the match expression:

```rust
fn resolve_try_expression(&mut self, expr: &TryExpression) -> Expression {
    let operand = self.resolve_expression(&expr.operand);

    // Create: match operand.tryExtract() { ... }
    let try_extract_call = self.create_method_call(
        operand,
        "tryExtract",
        vec![],
    );

    // Get the return type of the enclosing function
    let return_type = self.current_function_return_type();

    // Create the match arms
    let continue_arm = /* .Continue(value) => value */;
    let break_arm = /* .Break(early) => return R.fromResidual(early) */;

    Expression::match_expr(try_extract_call, vec![continue_arm, break_arm])
}
```

### 6. Add Standard Library Types

Create `lang/std/ops/try.ks`:

```kestrel
module std.ops

@builtin(.ControlFlowEnum)
public enum ControlFlow[Continue, Break] {
    case Continue(Continue)
    case Break(Break)
}

@builtin(.TryableProtocol)
public protocol Tryable {
    type Output
    type Early

    @builtin(.TryExtractMethod)
    func tryExtract() -> ControlFlow[Output, Early]
}

@builtin(.FromResidualProtocol)
public protocol FromResidual[Early] {
    @builtin(.FromResidualMethod)
    static func fromResidual(residual: Early) -> Self
}
```

### 7. Add Conformances

Add `Tryable` and `FromResidual` conformances to `Result` and `Optional` in their respective files.

## Future Extensions

### Catch Blocks

A future `catch { }` block could capture errors instead of returning:

```kestrel
let result = catch {
    let a = try foo()
    let b = try bar()
    a + b
}
// result: Result[Int, Error]
```

### Async Interaction

When async is added, determine syntax for combining `try` and `await`:

```kestrel
let x = try await asyncOperation()
// or
let x = await try asyncOperation()
```

## References

- [Rust Try trait](https://doc.rust-lang.org/std/ops/trait.Try.html)
- [Rust FromResidual trait](https://doc.rust-lang.org/std/ops/trait.FromResidual.html)
- [RFC 3058 - try-trait-v2](https://rust-lang.github.io/rfcs/3058-try-trait-v2.html)
