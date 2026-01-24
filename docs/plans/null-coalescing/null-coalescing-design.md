# Null Coalescing Operator (`??`) Design

## Overview

Add the null coalescing operator `??` to provide a concise way to unwrap optional values with a default fallback. This operator is commonly found in Swift, C#, TypeScript, and Kotlin.

## Syntax

```kestrel
let value = optional ?? defaultValue
```

## Semantic Behavior

### Type Signature

```
Optional[T] ?? T -> T
Optional[T] ?? Optional[T] -> Optional[T]
```

The operator unwraps the optional if it has a value, otherwise evaluates and returns the RHS.

### Short-Circuit Evaluation

The RHS is only evaluated if the LHS is `None`. This is implemented by wrapping the RHS in a closure, similar to `and`/`or`:

```kestrel
let x = someOptional ?? expensiveComputation()
// expensiveComputation() is NOT called if someOptional has a value
```

### Precedence and Associativity

| Operator | Precedence | Associativity |
|----------|------------|---------------|
| `??`     | 15 (new: COALESCING) | Right |
| `and`    | 20 (CONJUNCTIVE) | Left |
| `or`     | 10 (DISJUNCTIVE) | Left |

**Right-associativity** enables natural chaining:
```kestrel
a ?? b ?? c ?? default
// Parses as: a ?? (b ?? (c ?? default))
```

**Higher precedence than `or`** ensures intuitive grouping:
```kestrel
x ?? y ?? false or z
// Parses as: (x ?? (y ?? false)) or z
```

### Protocol-Based Implementation

Define a `Coalesce` protocol in `std.core`:

```kestrel
@builtin(.CoalesceOperatorProtocol)
public protocol Coalesce[Default] {
    type Output

    @builtin(.CoalesceOperatorMethod)
    func coalesce(default: () -> Default) -> Output
}
```

`Optional[T]` implements this protocol:

```kestrel
extend Optional[T]: Coalesce[T] {
    type Output = T

    public func coalesce(default: () -> T) -> T {
        match self {
            .Some(value) => value,
            .None => default()
        }
    }
}

// Also support Optional ?? Optional -> Optional
extend Optional[T]: Coalesce[Optional[T]] {
    type Output = Optional[T]

    public func coalesce(default: () -> Optional[T]) -> Optional[T] {
        match self {
            .Some(value) => .Some(value),
            .None => default()
        }
    }
}
```

## Error Cases

| Condition | Error Message |
|-----------|---------------|
| LHS is not optional | "operator `??` requires an optional type on the left-hand side" |
| RHS type mismatch | "expected `T` or `Optional[T]`, found `U`" |
| Type doesn't implement Coalesce | "type `X` does not conform to `Coalesce`" |

## Edge Cases

1. **Chaining**: `a ?? b ?? c` works naturally with right-associativity
2. **Mixed with `or`**: `x ?? false or y` parses as `(x ?? false) or y` due to precedence
3. **Nested optionals**: `Optional[Optional[T]] ?? Optional[T]` returns `Optional[T]`
4. **With method calls**: `dict.get(key) ?? computeDefault()` - short-circuits correctly

## Open Questions (Resolved)

1. **`??` vs extending `or`?** → Use `??` for semantic clarity; `or` is for booleans
2. **Associativity?** → Right-associative (matches Swift, C#, enables chaining)
3. **Precedence relative to `or`?** → Higher than `or` (15 vs 10)
4. **Short-circuit?** → Yes, via closure wrapping (like `and`/`or`)
5. **Protocol-based?** → Yes, via `Coalesce` protocol for extensibility

## Existing Infrastructure

The following already exists and will be reused:
- Token: `Token::QuestionQuestion` (lexer)
- Syntax: `SyntaxKind::QuestionQuestion` (parser)
- Operator: `BinaryOp::Coalesce` with method name `"coalesce"`
- Operator registry entry (needs precedence/associativity update)

## References

- [Swift Nil Coalescing](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/basicoperators/#Nil-Coalescing-Operator)
- [Zig orelse](https://zig.guide/language-basics/optionals/)
- [Swift Evolution SE-0077: Operator Precedence](https://github.com/apple/swift-evolution/blob/master/proposals/0077-operator-precedence.md)
