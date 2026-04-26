# Operator Overloading

Operators in Kestrel are dispatched through protocols. To make `+` work on your type, conform to the protocol that defines `+`.

## Conforming to an arithmetic protocol

```swift
struct Vec2 {
    let x: Float
    let y: Float
}

extend Vec2: Addable {
    public func add(other: Vec2) -> Vec2 {
        Vec2(x: self.x + other.x, y: self.y + other.y)
    }
}

let v = Vec2(x: 1.0, y: 2.0) + Vec2(x: 3.0, y: 4.0)   // Vec2(4, 6)
```

`Addable` (or whatever the stdlib calls it — see [Reference → Operators](../reference/operators.md)) requires an `add` method. The compiler rewrites `a + b` into `a.add(b)` for any conforming type.

The same pattern applies to `-` (`Subtractable`), `*` (`Multipliable`), `<` (`Comparable`), `==` (`Equatable`), and so on.

## Equality

Equality (`==`) goes through `Equatable`:

```swift
extend Vec2: Equatable {
    public func equals(other: Vec2) -> Bool {
        self.x == other.x && self.y == other.y
    }
}

v1 == v2   // calls v1.equals(v2)
```

Conforming to `Equatable` is also a prerequisite for using a type as a dictionary key (which additionally requires `Hashable`) or in a `Set`.

## Comparable

Ordering (`<`, `>`, `<=`, `>=`) goes through `Comparable`:

```swift
extend Length: Comparable {
    public func compare(to other: Length) -> Int {
        self.meters - other.meters
    }
}
```

`compare` returns negative, zero, or positive — the same convention as C's `strcmp`. The compiler derives all four operators from this single method.

## When to overload

Overload operators when the type really *is* numeric or orderable in a way readers will recognize at a glance — vectors, money, durations, points. Don't overload to be cute; an unfamiliar type with overloaded `+` makes call sites read worse, not better. If the operation needs a name to be understood, give it one.

For the canonical operator-to-protocol mapping, see [Reference → Operators](../reference/operators.md).

---

[← Closures](closures.md) · [↑ Functions](index.md) · [Control Flow →](../control-flow.md)
