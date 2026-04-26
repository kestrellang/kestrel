# Operators

Full operator table with precedence, associativity, and the protocol that backs each one. To define operators on your own types, see [Functions → Operator Overloading](../functions/operator-overloading.md).

## Precedence (high to low)

| Level | Operators | Associativity | Protocol |
|---|---|---|---|
| 1 | `.` (member access) | left | — |
| 2 | `()` `[]` (call, subscript) | left | — |
| 3 | `!` `~` `-` (unary, prefix) | right | `Negatable`, `BitwiseNot`, `LogicalNot` |
| 4 | `*` `/` `%` | left | `Multipliable`, `Divisible`, `Modulo` |
| 5 | `+` `-` | left | `Addable`, `Subtractable` |
| 6 | `<<` `>>` | left | `Shiftable` |
| 7 | `&` | left | `BitwiseAnd` |
| 8 | `^` | left | `BitwiseXor` |
| 9 | `|` | left | `BitwiseOr` |
| 10 | `==` `!=` | none | `Equatable` |
| 11 | `<` `<=` `>` `>=` | none | `Comparable` |
| 12 | `&&` | left | `Bool` (short-circuit) |
| 13 | `||` | left | `Bool` (short-circuit) |
| 14 | `=` `+=` `-=` `*=` `/=` `%=` (assignment) | right | — |

## Notes

- `&&` and `||` are short-circuit and only work on `Bool`. They aren't backed by a protocol because the laziness can't be expressed in a protocol method.
- Comparison operators (`<`, `==`) are non-associative — `a < b < c` is a compile error. Write the conjunction explicitly: `a < b && b < c`.
- Compound-assignment operators (`+=`, etc.) require both `Addable` and `var` on the left-hand side.
- Subscript (`obj(key)` or `obj[key]`) is special: defined per-type via the `subscript` declaration. See [Structs → Subscripts](../structs/subscripts.md).
- Unary `-` requires `Negatable`. There is no unary `+`.

## Custom operators

Kestrel does not currently support user-defined operator symbols (`<>`, `>>=`, etc.). Operator support is fixed to the table above; conform your type to the relevant protocol to give those symbols meaning.

---

[← Stdlib](stdlib.md) · [↑ Reference](index.md) · [Builtins →](builtins.md)
