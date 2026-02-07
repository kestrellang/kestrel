# Compound Assignment Operators

Compound assignment operators combine a binary operation with assignment, providing a concise syntax for updating a variable in place.

## Syntax

```kestrel
x += value    // addition assignment
x -= value    // subtraction assignment
x *= value    // multiplication assignment
x /= value    // division assignment
x %= value    // remainder assignment
x &= value    // bitwise AND assignment
x |= value    // bitwise OR assignment
x ^= value    // bitwise XOR assignment
x <<= value   // left shift assignment
x >>= value   // right shift assignment
```

## Behavior

Compound assignments desugar to protocol method calls:

| Operator | Desugars To | Protocol |
|----------|-------------|----------|
| `a += b` | `a.addAssign(b)` | `AddAssign` |
| `a -= b` | `a.subtractAssign(b)` | `SubtractAssign` |
| `a *= b` | `a.multiplyAssign(b)` | `MultiplyAssign` |
| `a /= b` | `a.divideAssign(b)` | `DivideAssign` |
| `a %= b` | `a.modAssign(b)` | `ModuloAssign` |
| `a &= b` | `a.bitwiseAndAssign(b)` | `BitwiseAndAssign` |
| `a \|= b` | `a.bitwiseOrAssign(b)` | `BitwiseOrAssign` |
| `a ^= b` | `a.bitwiseXorAssign(b)` | `BitwiseXorAssign` |
| `a <<= b` | `a.shiftLeftAssign(by: b)` | `LeftShiftAssign` |
| `a >>= b` | `a.shiftRightAssign(by: b)` | `RightShiftAssign` |

## Return Type

Compound assignment expressions have type `()` (unit). This prevents chaining:

```kestrel
var a = 1
var b = 2
a += b += 1  // Error: b += 1 has type (), cannot use as operand
```

## Examples

### Basic Usage

```kestrel
var x: Int = 5
x += 1      // x is now 6
x *= 2      // x is now 12
x -= 3      // x is now 9
```

### With Fields

```kestrel
struct Counter {
    var value: Int
}

var counter = Counter(value: 0)
counter.value += 1  // counter.value is now 1
```

### With Expressions

```kestrel
var x: Int = 10
x += getValue() + 5  // RHS can be any expression
x *= 2 + 3           // Parses as: x *= (2 + 3)
```

## Protocols

The compound assignment protocols are defined in `std.core`:

```kestrel
public protocol AddAssign[Rhs = Self] {
    mutating func addAssign(other: Rhs)
}

public protocol SubtractAssign[Rhs = Self] {
    mutating func subtractAssign(other: Rhs)
}

// ... etc for other operators
```

All numeric types (`Int`, `Int8`, `Int16`, etc.) implement these protocols.

## Implementing for Custom Types

To support compound assignment on your own types, implement the appropriate protocol:

```kestrel
struct Vector2 {
    var x: Int
    var y: Int
}

extend Vector2: AddAssign {
    public mutating func addAssign(other: Vector2) {
        self.x += other.x
        self.y += other.y
    }
}

// Now you can use +=
var v = Vector2(x: 1, y: 2)
v += Vector2(x: 3, y: 4)  // v is now (4, 6)
```
