# Operator Protocols Design

This document describes the protocol-based operator system for Kestrel.

## Overview

Operators are desugared to protocol method calls. Each operator has a corresponding protocol marked with `@builtin`, and the operator's method is also marked with `@builtin`.

```kestrel
x + y  // desugars to: Addable.add(x)(y)
```

The desugaring phase creates a call to the protocol method without checking conformance. Conformance checking happens later during type checking.

## Protocol Structure

Each operator protocol follows this pattern:

```kestrel
@builtin(.AddOperatorProtocol)
public protocol Addable[Rhs = Self] {
    type Output

    @builtin(.AddOperatorMethod)
    func add(other: Rhs) -> Output
}
```

Key aspects:
- `@builtin(.XxxOperatorProtocol)` marks the protocol for compiler lookup
- `@builtin(.XxxOperatorMethod)` marks the method for compiler lookup
- `Rhs` type parameter with default `Self` enables heterogeneous operations
- `Output` associated type allows flexible return types

## Operator Categories

### Arithmetic Operators

| Operator | Protocol | Method | Builtin (Protocol) | Builtin (Method) |
|----------|----------|--------|-------------------|------------------|
| `+` | `Addable` | `add` | `AddOperatorProtocol` | `AddOperatorMethod` |
| `-` | `Subtractable` | `subtract` | `SubtractOperatorProtocol` | `SubtractOperatorMethod` |
| `*` | `Multipliable` | `multiply` | `MultiplyOperatorProtocol` | `MultiplyOperatorMethod` |
| `/` | `Divisible` | `divide` | `DivideOperatorProtocol` | `DivideOperatorMethod` |
| `%` | `Modulable` | `modulo` | `ModuloOperatorProtocol` | `ModuloOperatorMethod` |

### Comparison Operators

| Operator | Protocol | Method | Builtin (Protocol) | Builtin (Method) |
|----------|----------|--------|-------------------|------------------|
| `==` | `Equatable` | `equals` | `EqualsOperatorProtocol` | `EqualsOperatorMethod` |
| `!=` | `NotEquatable` | `notEquals` | `NotEqualsOperatorProtocol` | `NotEqualsOperatorMethod` |
| `<` | `LessThanComparable` | `lessThan` | `LessThanOperatorProtocol` | `LessThanOperatorMethod` |
| `<=` | `LessOrEqualComparable` | `lessThanOrEqual` | `LessOrEqualOperatorProtocol` | `LessOrEqualOperatorMethod` |
| `>` | `GreaterThanComparable` | `greaterThan` | `GreaterThanOperatorProtocol` | `GreaterThanOperatorMethod` |
| `>=` | `GreaterOrEqualComparable` | `greaterThanOrEqual` | `GreaterOrEqualOperatorProtocol` | `GreaterOrEqualOperatorMethod` |

### Bitwise Operators

| Operator | Protocol | Method | Builtin (Protocol) | Builtin (Method) |
|----------|----------|--------|-------------------|------------------|
| `&` | `BitwiseAndable` | `bitwiseAnd` | `BitwiseAndOperatorProtocol` | `BitwiseAndOperatorMethod` |
| `\|` | `BitwiseOrable` | `bitwiseOr` | `BitwiseOrOperatorProtocol` | `BitwiseOrOperatorMethod` |
| `^` | `BitwiseXorable` | `bitwiseXor` | `BitwiseXorOperatorProtocol` | `BitwiseXorOperatorMethod` |
| `<<` | `LeftShiftable` | `shiftLeft` | `ShiftLeftOperatorProtocol` | `ShiftLeftOperatorMethod` |
| `>>` | `RightShiftable` | `shiftRight` | `ShiftRightOperatorProtocol` | `ShiftRightOperatorMethod` |

### Logical Operators

| Operator | Protocol | Method | Builtin (Protocol) | Builtin (Method) |
|----------|----------|--------|-------------------|------------------|
| `and` | `LogicalAndable` | `logicalAnd` | `LogicalAndOperatorProtocol` | `LogicalAndOperatorMethod` |
| `or` | `LogicalOrable` | `logicalOr` | `LogicalOrOperatorProtocol` | `LogicalOrOperatorMethod` |

Note: Logical operators are **not** short-circuiting in the current design.

### Unary Operators

| Operator | Protocol | Method | Builtin (Protocol) | Builtin (Method) |
|----------|----------|--------|-------------------|------------------|
| `-x` | `Negatable` | `negate` | `NegateOperatorProtocol` | `NegateOperatorMethod` |
| `not x` | `LogicalNegatable` | `logicalNot` | `LogicalNotOperatorProtocol` | `LogicalNotOperatorMethod` |
| `~x` | `BitwiseNegatable` | `bitwiseNot` | `BitwiseNotOperatorProtocol` | `BitwiseNotOperatorMethod` |

Unary operator protocols have no `Rhs` parameter:

```kestrel
@builtin(.NegateOperatorProtocol)
public protocol Negatable {
    type Output

    @builtin(.NegateOperatorMethod)
    func negate() -> Output
}
```

### Range Operators

| Operator | Protocol | Method | Builtin (Protocol) | Builtin (Method) |
|----------|----------|--------|-------------------|------------------|
| `..` | `ExclusiveRangeable` | `exclusiveRange` | `ExclusiveRangeOperatorProtocol` | `ExclusiveRangeOperatorMethod` |
| `..=` | `InclusiveRangeable` | `inclusiveRange` | `InclusiveRangeOperatorProtocol` | `InclusiveRangeOperatorMethod` |

## Removed Operators

The identity operator `+x` is removed entirely. Using `+x` will result in a syntax error.

## Compound Assignment

Compound assignment operators desugar to the binary operator plus assignment:

```kestrel
x += y  // desugars to: x = x + y
x -= y  // desugars to: x = x - y
// etc.
```

No separate protocols are needed for compound assignment.

## Primitive Types

Primitive types (`Int32`, `Float64`, etc.) continue to use `PrimitiveMethodCall` for operators. They do not go through the protocol system. This is a compiler optimization - the primitives have built-in operator semantics.

## Heterogeneous Operations

Types can conform to operator protocols with different `Rhs` types:

```kestrel
extension Int32: Addable[Int32] {
    type Output = Int32
    func add(other: Int32) -> Int32 { ... }
}

extension Int32: Addable[Float64] {
    type Output = Float64
    func add(other: Float64) -> Float64 { ... }
}
```

This enables operations like `myInt + myFloat`.

## Desugaring Process

When the compiler sees a binary operator expression `x + y`:

1. Look up the `Addable` protocol via `@builtin(.AddOperatorProtocol)`
2. Look up the `add` method via `@builtin(.AddOperatorMethod)`
3. Create a protocol method call expression
4. Conformance checking happens later during type checking

For unary operators like `-x`:

1. Look up the `Negatable` protocol via `@builtin(.NegateOperatorProtocol)`
2. Look up the `negate` method via `@builtin(.NegateOperatorMethod)`
3. Create a protocol method call expression

## Type Inference

When the receiver type is not yet known (inferred), the desugaring still creates the protocol method call. Type inference will:

1. Add a constraint that the receiver must conform to the operator protocol
2. Resolve the `Output` associated type to determine the expression's type

## Implementation Changes

### New `LanguageFeature` Variants

Add to `builtins.rs`:

```rust
// Operator protocols
AddOperatorProtocol,
AddOperatorMethod,
SubtractOperatorProtocol,
SubtractOperatorMethod,
// ... etc for all operators
```

### Operator Desugaring

Modify `body_resolver/operators.rs` to:

1. Look up the operator protocol and method from the builtin registry
2. Create a protocol method call instead of `PrimitiveMethodCall` for non-primitive types
3. Remove the identity operator (`+x`) handling

### Remove Identity Operator

- Remove `UnaryOp::Identity` from `operators.rs`
- Remove `IntIdentity` and `FloatIdentity` from `PrimitiveMethod`
- Update the parser to reject `+x` as a syntax error

## Standard Library Changes

Update `lang/std/ops/` protocols to use the new `@builtin` attributes:

```kestrel
// arithmetic.ks
@builtin(.AddOperatorProtocol)
public protocol Addable[Rhs = Self] {
    type Output

    @builtin(.AddOperatorMethod)
    func add(other: Rhs) -> Output
}

// ... similar for other operators
```

## Migration

Existing code using operators will continue to work for primitive types. User-defined types that want to support operators must conform to the appropriate protocols.
