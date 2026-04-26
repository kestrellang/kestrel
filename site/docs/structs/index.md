# Structs

A struct groups named values into a single type. They're Kestrel's go-to for representing data — points, requests, configurations, anything where you want a fixed set of fields with known types.

## A first struct

```swift
struct Point {
    let x: Int
    let y: Int
}

let origin = Point(x: 0, y: 0)
let p = Point(x: 3, y: 4)

println("(\(p.x), \(p.y))")
```

Two fields, both `let`, both `Int`. The compiler synthesizes a memberwise initializer that takes one labeled argument per field — that's why `Point(x: 0, y: 0)` works without you writing an `init`.

## Mutability

Mark a field `var` to make it writable:

```swift
struct Counter {
    var value: Int
}

var c = Counter(value: 0)
c.value = c.value + 1
```

Writing through `c.value` requires `c` to be `var` itself. A `let counter` is fully frozen, regardless of whether its fields are `let` or `var`.

## Methods

Methods live in `extend` blocks:

```swift
extend Point {
    func distance(to other: Point) -> Float {
        let dx = self.x - other.x
        let dy = self.y - other.y
        Float.sqrt(Float(dx * dx + dy * dy))
    }
}

origin.distance(to: p)
```

`mutating` methods write to `var` fields. `static` methods don't need an instance. See [Methods](methods.md) for the full coverage.

## What's in this section

- [Fields](fields.md) — `let` vs `var`, defaults, type annotations
- [Methods](methods.md) — instance, mutating, and static
- [Initializers](initializers.md) — custom `init` and the memberwise default
- [Deinitializers](deinitializers.md) — cleanup hooks tied to ARC
- [Computed Variables](computed-variables.md) — properties derived from other state
- [Subscripts](subscripts.md) — defining `obj(key)` access on your own types

For sum types (variants instead of fields), see [Enums](../enums/index.md). For abstracting over multiple struct types, see [Protocols](../protocols/index.md).

---

[← Iterators](../collections/iterators.md) · [↑ The Kestrel Language](../index.md) · [Fields →](fields.md)
