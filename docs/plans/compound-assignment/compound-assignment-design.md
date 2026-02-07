# Compound Assignment Operators Design

## Overview

Compound assignment operators combine a binary operation with assignment, providing a concise syntax for updating a variable in place. For example, `x += 1` is equivalent to `x = x + 1`, but expressed more concisely and potentially more efficiently.

## Syntax

```kestrel
// Arithmetic
x += value    // addition assignment
x -= value    // subtraction assignment
x *= value    // multiplication assignment
x /= value    // division assignment
x %= value    // remainder assignment

// Bitwise
x &= value    // bitwise AND assignment
x |= value    // bitwise OR assignment
x ^= value    // bitwise XOR assignment

// Shift
x <<= value   // left shift assignment
x >>= value   // right shift assignment
```

## Semantic Behavior

### Desugaring

Compound assignments desugar to protocol method calls:

| Operator | Desugars To | Protocol |
|----------|-------------|----------|
| `a += b` | `a.addAssign(b)` | `AddAssign` |
| `a -= b` | `a.subtractAssign(b)` | `SubtractAssign` |
| `a *= b` | `a.multiplyAssign(b)` | `MultiplyAssign` |
| `a /= b` | `a.divideAssign(b)` | `DivideAssign` |
| `a %= b` | `a.moduloAssign(b)` | `ModuloAssign` |
| `a &= b` | `a.bitwiseAndAssign(b)` | `BitwiseAndAssign` |
| `a \|= b` | `a.bitwiseOrAssign(b)` | `BitwiseOrAssign` |
| `a ^= b` | `a.bitwiseXorAssign(b)` | `BitwiseXorAssign` |
| `a <<= b` | `a.leftShiftAssign(b)` | `LeftShiftAssign` |
| `a >>= b` | `a.rightShiftAssign(b)` | `RightShiftAssign` |

### Return Type

Compound assignment expressions have type `()` (unit), just like regular assignment. This prevents chaining:

```kestrel
var a = 1
var b = 2
a += b += 1  // ERROR: b += 1 has type (), cannot use as operand
```

### Mutability Requirements

The target of a compound assignment must be mutable:

```kestrel
let x = 5
x += 1       // ERROR: cannot mutate immutable binding

var y = 5
y += 1       // OK
```

### Default Protocol Implementations

Types that implement binary operator protocols with `Output = Self` automatically get default compound assignment implementations:

```kestrel
extend Add[Rhs]: AddAssign[Rhs] where Add[Rhs].Output = Self {
    mutating func addAssign(other: Rhs) { self = self.add(other) }
}

extend Subtract[Rhs]: SubtractAssign[Rhs] where Subtract[Rhs].Output = Self {
    mutating func subtractAssign(other: Rhs) { self = self.subtract(other) }
}

// ... and so on for all compound assignment protocols
```

This means any type with `Add[Output = Self]` automatically supports `+=`.

## Error Cases

| Condition | Error Message |
|-----------|---------------|
| Target is immutable | "cannot assign to immutable variable 'x'" |
| Target doesn't support protocol | "type 'T' does not conform to 'AddAssign'" |
| Invalid assignment target | "invalid assignment target" |
| Type mismatch | "expected 'Int', found 'String'" |

## Edge Cases

### Field Access
```kestrel
struct Point { var x: Int, var y: Int }

var p = Point(x: 0, y: 0)
p.x += 1     // OK: p is mutable, x is var field

let q = Point(x: 0, y: 0)
q.x += 1     // ERROR: q is immutable
```

### Subscript Access
```kestrel
var arr = [1, 2, 3]
arr(0) += 10  // OK: calls subscript setter after compound operation
```

### Complex Expressions
```kestrel
obj.field[index] += value  // All parts must be mutable
```

### Precedence
Compound assignment has lower precedence than binary operators:
```kestrel
x += y + z   // Parses as: x += (y + z)
x += y * z   // Parses as: x += (y * z)
```

## Open Questions (Resolved)

1. **Q: Should compound assignment be an expression or statement?**
   A: Expression with type `()` (unit), same as regular assignment. This prevents confusing chaining like `a += b += c`.

2. **Q: Should we support `??=` (null coalesce assignment)?**
   A: No, not in this feature.

3. **Q: How to handle default implementations?**
   A: Enable default extensions in `assign.ks` - types with `Add[Output = Self]` automatically get `AddAssign`.

4. **Q: What about types where `Add` returns a different type?**
   A: They must explicitly implement `AddAssign` if they want to support `+=`. No default is provided since the output type doesn't match.
