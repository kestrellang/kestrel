# Throw Expression Design

## Overview

Throw expressions provide ergonomic error propagation in Kestrel. They allow early returns with error values in functions that implement the `FromResidual` protocol.

**Motivation**: Instead of manually wrapping errors in Result types (`return .Err(error)`), developers can use the more intuitive `throw error` syntax. This is particularly useful with the `try` operator for error handling chains.

## Syntax

```kestrel
// Basic throw expression
throw errorValue

// Common usage with try
let value = try mightFail() ?? throw MyError()

// In function returning Result type
func divide(a: Int, b: Int) -> Int throws DivByZero {
    if b == 0 {
        throw DivByZero()  // desugars to: return Int throws DivByZero.fromResidual(DivByZero())
    }
    return a / b
}
```

## Semantic Behavior

### Desugaring

`throw expr` desugars to `return R.fromResidual(expr)` where:
- `R` is the enclosing function's declared return type
- The return type must implement `FromResidual[ErrorType]` (enforced by type system)
- `expr` must match the error type expected by `FromResidual`

### Type

Throw expressions have type `Never` (same as return, break, continue) because control flow does not continue after a throw.

### Protocol Requirements

The enclosing function's return type must implement `FromResidual`. This is the same protocol used by the `try` operator for error propagation.

**Note**: The type system will naturally enforce this constraint when resolving the `fromResidual` method call during semantic analysis.

## Error Cases

| Condition | Error Message |
|-----------|---------------|
| Missing expression after `throw` | "expected expression after 'throw'" |
| Return type doesn't implement `FromResidual` | "type `T` does not implement `FromResidual[E]`" (from type system) |
| Incompatible error type | "cannot convert expression of type `E1` to expected type `E2`" (from type system) |
| Throw outside function | "'throw' can only be used inside a function body" |

## Edge Cases

1. **Nested Functions**: Throw affects the innermost function context. If a closure is defined inside a function, throw in the closure refers to the closure's return type, not the outer function.

2. **Generic Functions**: Works correctly with generic return types that implement `FromResidual`.

3. **Never Type Propagation**: Since throw has type Never, it participates in Never type propagation like other diverging expressions.

4. **Dead Code Detection**: Code after a throw expression is unreachable and will be flagged by dead code detection.

5. **Expression Context**: Throw can be used anywhere an expression is expected, as long as the return type implements `FromResidual`.

## Open Questions (Resolved)

**Q: Should bare `throw` be allowed?**  
A: No. Always require an explicit error value.

**Q: Should throw require an explicit Result return type?**  
A: No. It only requires the return type to implement `FromResidual`, which the type system enforces naturally.

**Q: How does throw interact with the `??` operator?**  
A: Out of scope for this feature. The `??` operator with Never type handling is a separate feature.

## Implementation Notes

- Parser: Add `throw` as a keyword, parse `throw <expression>`
- Semantic: Desugar during body resolution to `return R.fromResidual(expr)`
- Type: Never (diverging expression)
- Lowering: Same as return (since it desugars to return)
