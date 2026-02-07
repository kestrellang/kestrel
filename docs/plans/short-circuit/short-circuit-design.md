# Short-Circuit Evaluation for `and`/`or` Operators

## Overview

Implement short-circuit evaluation for the `and` and `or` logical operators by changing the protocol signatures to accept closures for the right-hand operand.

## Current Behavior

```kestrel
// Current: both operands always evaluated
a and b  // desugars to a.logicalAnd(b)
a or b   // desugars to a.logicalOr(b)
```

Both operands are evaluated eagerly, even when the result is determined by the left operand alone.

## Proposed Behavior

```kestrel
// Proposed: right operand wrapped in closure
a and b  // desugars to a.logicalAnd { b }
a or b   // desugars to a.logicalOr { b }
```

The right operand is wrapped in a zero-parameter closure. The protocol implementation decides whether to call it.

## Syntax

No syntax changes. The `and` and `or` operators continue to work as before:

```kestrel
if isValid and hasPermission { ... }
if isEmpty or isDisabled { ... }
```

## Semantic Behavior

### Protocol Changes

**Before:**
```kestrel
public protocol And[Rhs = Self] {
    type Output
    func logicalAnd(other: Rhs) -> Output
}

public protocol Or[Rhs = Self] {
    type Output
    func logicalOr(other: Rhs) -> Output
}
```

**After:**
```kestrel
public protocol And[Rhs = Self] {
    type Output
    func logicalAnd(other: () -> Rhs) -> Output
}

public protocol Or[Rhs = Self] {
    type Output
    func logicalOr(other: () -> Rhs) -> Output
}
```

### Bool Implementation

```kestrel
extend Bool with And[Bool] {
    type Output = Bool

    public func logicalAnd(other: () -> Bool) -> Bool {
        if self.value { other() } else { false }
    }
}

extend Bool with Or[Bool] {
    type Output = Bool

    public func logicalOr(other: () -> Bool) -> Bool {
        if self.value { true } else { other() }
    }
}
```

### Short-Circuit Semantics

| Expression | Left Value | Right Evaluated? | Result |
|------------|------------|------------------|--------|
| `a and b`  | `false`    | No               | `false` |
| `a and b`  | `true`     | Yes              | `b` |
| `a or b`   | `true`     | No               | `true` |
| `a or b`   | `false`    | Yes              | `b` |

### Chained Operators

```kestrel
a and b and c
// Parses as: (a and b) and c
// Desugars to: (a.logicalAnd { b }).logicalAnd { c }

// If a is false: b and c are not evaluated
// If a is true, b is false: c is not evaluated
// If a is true, b is true: c is evaluated
```

```kestrel
a or b or c
// Parses as: (a or b) or c
// Desugars to: (a.logicalOr { b }).logicalOr { c }

// If a is true: b and c are not evaluated
// If a is false, b is true: c is not evaluated
// If a is false, b is false: c is evaluated
```

### Mixed Operators

Precedence: `and` (20) > `or` (10)

```kestrel
a or b and c
// Parses as: a or (b and c)
// Desugars to: a.logicalOr { b.logicalAnd { c } }

// If a is true: neither b nor c evaluated
// If a is false: b is evaluated, then c if b is true
```

## Error Cases

| Condition | Error Message |
|-----------|---------------|
| LHS doesn't implement `And`/`Or` | "type `X` does not implement `And`" |
| Type mismatch in RHS | "expected `() -> Bool`, found `() -> Int`" |

## Edge Cases

### Side Effects

```kestrel
func log(msg: String) -> Bool {
    print(msg)
    true
}

false and log("never printed")  // log() not called
true or log("never printed")    // log() not called
```

### Nested Expressions

```kestrel
(a and b) or (c and d)
// If a && b is true: c and d not evaluated
// Desugars to: (a.logicalAnd { b }).logicalOr { c.logicalAnd { d } }
```

### With Other Operators

```kestrel
x > 0 and x < 10
// Comparison operators have higher precedence
// Parses as: (x > 0) and (x < 10)
```

## Breaking Changes

This is a **breaking change** for:

1. **Protocol implementations** - Any type implementing `And` or `Or` must update method signatures
2. **Direct method calls** - Code calling `.logicalAnd(value)` must change to `.logicalAnd { value }`

The `and`/`or` operator syntax remains unchanged.

## Implementation Notes

### Desugaring Location

The change is in `lib/kestrel-semantic-tree-binder/src/body_resolver/operators.rs` where binary operators are desugared to method calls.

### Closure Creation

The RHS expression `b` becomes `{ b }` - a zero-parameter closure returning `b`.

### No Control Flow Changes

Unlike traditional short-circuit implementations that require compiler-level branching, this approach uses the existing closure infrastructure. The branching happens inside the protocol method implementation via `if`.
