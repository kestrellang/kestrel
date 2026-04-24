# Operator Desugaring

All operators desugar to protocol method calls (`ProtocolCall` expressions). The protocol entity is resolved from the DefMap during HIR lowering. Adding a new operator requires only a table entry — no new HIR variants.

## Binary Operators

`a + b` becomes `ProtocolCall { receiver: a, protocol: Addable, method: "add", args: [b] }`

| Operator | Syntax | Protocol | Method | Arg Label |
|----------|--------|----------|--------|-----------|
| `Add` | `a + b` | `Addable` | `add` | — |
| `Sub` | `a - b` | `Subtractable` | `subtract` | — |
| `Mul` | `a * b` | `Multipliable` | `multiply` | — |
| `Div` | `a / b` | `Divisible` | `divide` | — |
| `Rem` | `a % b` | `Modulo` | `modulo` | — |
| `Eq` | `a == b` | `Equal` | `equals` | — |
| `Ne` | `a != b` | `NotEqual` | `notEquals` | — |
| `Lt` | `a < b` | `Less` | `lessThan` | — |
| `Gt` | `a > b` | `Greater` | `greaterThan` | — |
| `Le` | `a <= b` | `LessOrEqual` | `lessThanOrEqual` | — |
| `Ge` | `a >= b` | `GreaterOrEqual` | `greaterThanOrEqual` | — |
| `BitAnd` | `a & b` | `BitwiseAnd` | `bitwiseAnd` | — |
| `BitOr` | `a \| b` | `BitwiseOr` | `bitwiseOr` | — |
| `BitXor` | `a ^ b` | `BitwiseXor` | `bitwiseXor` | — |
| `Shl` | `a << b` | `LeftShift` | `shiftLeft` | `by` |
| `Shr` | `a >> b` | `RightShift` | `shiftRight` | `by` |
| `RangeInclusive` | `a..=b` | `ClosedRangeConstructible` | `inclusiveRange` | `to` |
| `RangeExclusive` | `a..<b` | `RangeConstructible` | `exclusiveRange` | `to` |

## Short-Circuit Operators

Right operand is wrapped in a closure to enable short-circuit evaluation.

`a and b` becomes `ProtocolCall { receiver: a, protocol: And, method: "logicalAnd", args: [Closure { body: b }] }`

| Operator | Syntax | Protocol | Method | Arg Label |
|----------|--------|----------|--------|-----------|
| `And` | `a and b` | `And` | `logicalAnd` | — |
| `Or` | `a or b` | `Or` | `logicalOr` | — |
| `Coalesce` | `a ?? b` | `Coalesce` | `coalesce` | — |

## Unary Operators

`-x` becomes `ProtocolCall { receiver: x, protocol: Negatable, method: "negate", args: [] }`

| Operator | Syntax | Protocol | Method |
|----------|--------|----------|--------|
| `Neg` | `-x` | `Negatable` | `negate` |
| `BitNot` | `^x` | `BitwiseNot` | `bitwiseNot` |
| `LogicalNot` | `not x` | `Not` | `logicalNot` |

`Pos` (`+x`) is intentionally unmapped — it's a no-op.

## Compound Assignment Operators

`x += 1` becomes `ProtocolCall { receiver: x, protocol: AddAssign, method: "addAssign", args: [1] }`

| Operator | Syntax | Protocol | Method | Arg Label |
|----------|--------|----------|--------|-----------|
| `AddAssign` | `x += y` | `AddAssign` | `addAssign` | — |
| `SubAssign` | `x -= y` | `SubtractAssign` | `subtractAssign` | — |
| `MulAssign` | `x *= y` | `MultiplyAssign` | `multiplyAssign` | — |
| `DivAssign` | `x /= y` | `DivideAssign` | `divideAssign` | — |
| `RemAssign` | `x %= y` | `ModuloAssign` | `modAssign` | — |
| `BitAndAssign` | `x &= y` | `BitwiseAndAssign` | `bitwiseAndAssign` | — |
| `BitOrAssign` | `x \|= y` | `BitwiseOrAssign` | `bitwiseOrAssign` | — |
| `BitXorAssign` | `x ^= y` | `BitwiseXorAssign` | `bitwiseXorAssign` | — |
| `ShlAssign` | `x <<= y` | `LeftShiftAssign` | `shiftLeftAssign` | `by` |
| `ShrAssign` | `x >>= y` | `RightShiftAssign` | `shiftRightAssign` | `by` |

## Lookup Functions

```rust
lookup_binary_op(&BinaryOp)         -> Option<(protocol, method, label)>
lookup_short_circuit_op(&BinaryOp)  -> Option<(protocol, method, label)>
lookup_unary_op(&UnaryOp)           -> Option<(protocol, method)>
lookup_compound_assign_op(&CompoundAssignOp) -> Option<(protocol, method, label)>
```

Label is `None` for single-name params (no external label in Kestrel), `Some("by")` for shift ops, `Some("to")` for range ops.
