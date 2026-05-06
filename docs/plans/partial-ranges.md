# Partial Range Expressions

Design doc for [#27](https://github.com/kestrellang/kestrel/issues/27).

## Summary

Add three partial range expression forms that mirror the existing binary range
operators:

| Expression | Type              | Stores   | Iterable? |
|------------|-------------------|----------|-----------|
| `x..`      | `RangeFrom[T]`    | `start`  | Yes       |
| `..<x`     | `RangeUpTo[T]`    | `end`    | No        |
| `..=x`     | `RangeThrough[T]` | `end`    | No        |

These complement the existing binary forms (`a..<b` → `Range[T]`,
`a..=b` → `ClosedRange[T]`).

## Syntax

**Postfix `x..`** — the `..` token immediately after an expression, where no
right-hand operand follows. Recognized when the next token is `)`, `,`, `]`,
`}`, newline, or expression-terminating position. If `..<` or `..=` follows
instead, it is the existing binary form.

**Prefix `..<x` and `..=x`** — the `..<` or `..=` token in expression-start
position (no left-hand operand). The parser already distinguishes these tokens,
and already handles the prefix forms in pattern context.

Pattern syntax (`x..`, `..<x`, `..=x` in `match` arms) is **unchanged** —
patterns continue to go through `RangeMatchable`, not the new protocols.

## Construction Protocols

Three new protocols, following the same pattern as
`RangeConstructible` / `ClosedRangeConstructible`:

```kestrel
@builtin(.RangeFromOperatorProtocol)
public protocol RangeFromConstructible {
    type Output

    @builtin(.RangeFromOperatorMethod)
    func rangeFrom() -> Output
}

@builtin(.RangeUpToOperatorProtocol)
public protocol RangeUpToConstructible {
    type Output

    @builtin(.RangeUpToOperatorMethod)
    func rangeUpTo() -> Output
}

@builtin(.RangeThroughOperatorProtocol)
public protocol RangeThroughConstructible {
    type Output

    @builtin(.RangeThroughOperatorMethod)
    func rangeThrough() -> Output
}
```

Each protocol has a single method with no arguments — the operand is `self`.
`RangeFromConstructible.rangeFrom()` is called on the *start* bound;
`RangeUpToConstructible.rangeUpTo()` and `RangeThroughConstructible.rangeThrough()`
are called on the *end* bound.

The `Output` associated type lets conformers produce custom partial range
types, just as `RangeConstructible.Output` can be anything.

## Stdlib Types

### `RangeFrom[T]` where `T: Steppable, T: Comparable`

```kestrel
public struct RangeFrom[T]: Equatable, Iterable
    where T: Steppable, T: Comparable
{
    type Item = T
    type TargetIterator = RangeFromIterator[T]

    public var start: T

    public init(start: T)

    public func contains(value: T) -> Bool    // value >= start
    public func isEqual(to other: RangeFrom[T]) -> Bool
    public func iter() -> RangeFromIterator[T]
}
```

`RangeFromIterator[T]` yields `start`, `start.successor()`, … with no
upper bound. Callers must `break` — this is an infinite iterator, same as
Swift's `(5...)`.

### `RangeUpTo[T]` where `T: Comparable`

```kestrel
public struct RangeUpTo[T]: Equatable
    where T: Comparable
{
    public var end: T

    public init(end: T)

    public func contains(value: T) -> Bool    // value < end
    public func isEqual(to other: RangeUpTo[T]) -> Bool
}
```

**Not `Iterable`** — there is no start to iterate from.

Note: `RangeUpTo` only requires `Comparable`, not `Steppable`, since it
never needs `successor()` / `predecessor()`.

### `RangeThrough[T]` where `T: Comparable`

```kestrel
public struct RangeThrough[T]: Equatable
    where T: Comparable
{
    public var end: T

    public init(end: T)

    public func contains(value: T) -> Bool    // value <= end
    public func isEqual(to other: RangeThrough[T]) -> Bool
}
```

**Not `Iterable`** — same reason as `RangeUpTo`.

## Conformances on Integer Types

All integer types (`Int64`, `Int32`, `Int16`, `Int8`, `UInt64`, `UInt32`,
`UInt16`, `UInt8`) conform to the three new protocols:

```kestrel
extend Int64: RangeFromConstructible, RangeUpToConstructible, RangeThroughConstructible {
    type RangeFromConstructible.Output    = RangeFrom[Int64]
    type RangeUpToConstructible.Output    = RangeUpTo[Int64]
    type RangeThroughConstructible.Output = RangeThrough[Int64]

    public func rangeFrom() -> RangeFrom[Int64] {
        RangeFrom[Int64](self)
    }

    public func rangeUpTo() -> RangeUpTo[Int64] {
        RangeUpTo[Int64](self)
    }

    public func rangeThrough() -> RangeThrough[Int64] {
        RangeThrough[Int64](self)
    }
}
```

## HIR Desugaring

The three expression forms desugar to protocol method calls, just like the
binary operators:

| Expression | Desugars to                                     |
|------------|-------------------------------------------------|
| `x..`      | `x.rangeFrom()` via `RangeFromConstructible`     |
| `..<x`     | `x.rangeUpTo()` via `RangeUpToConstructible`      |
| `..=x`     | `x.rangeThrough()` via `RangeThroughConstructible` |

This hooks into the existing protocol-call infrastructure — no new HIR node
kinds are needed.

## Compiler Touch Points

1. **Builtins** (`kestrel-hir/src/builtin.rs`) — six new variants:
   `RangeFromOperatorProtocol`, `RangeFromOperatorMethod`,
   `RangeUpToOperatorProtocol`, `RangeUpToOperatorMethod`,
   `RangeThroughOperatorProtocol`, `RangeThroughOperatorMethod`.

2. **Parser** (`kestrel-parser/src/expr/`) — recognize the three new
   expression forms. Postfix `x..` needs precedence handling.
   Prefix `..<x` and `..=x` need expression-start recognition.

3. **AST** (`kestrel-ast`) — new unary operator variants or new expression
   nodes for the three forms.

4. **HIR lowering** (`kestrel-hir-lower/src/desugar.rs`) — desugar to
   protocol calls, paralleling the existing `desugar_binary_hir` path.

5. **Stdlib** (`lang/std/core/range.ks`) — new protocols, types, iterators.
   Integer conformances in the `.ks.template` files.

6. **Type inference** — no special handling needed; the protocol-call
   machinery handles it.

## Out of Scope

- **`..` (unbounded range)** — a separate `UnboundedRange` type. Different
  enough to be its own feature.
- **Array/String subscript overloads** — will be a follow-up once the types
  exist.
- **Checked/clamped subscript variants** — follow-up.
- **Changes to pattern matching** — patterns already support these forms via
  `RangeMatchable`.

## Examples

```kestrel
// Infinite iteration with break
for i in 0.. {
    if i >= 100 { break }
    print(i)
}

// Containment checks
let positive = 0..
positive.contains(42)    // true
positive.contains(-1)    // false

let small = ..<100
small.contains(50)       // true
small.contains(100)      // false

// Passing as values
func clampLower(range: RangeFrom[Int64], value: Int64) -> Int64 {
    if range.contains(value) { value } else { range.start }
}
```
