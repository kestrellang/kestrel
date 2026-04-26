# Enums

An enum is a type with a fixed set of variants — *one* of several distinct shapes. Variants can carry data ("payloads"), and the compiler forces you to handle every variant when you destructure one.

## A first enum

```swift
enum Suit {
    case Hearts
    case Diamonds
    case Clubs
    case Spades
}

let s: Suit = .Hearts
```

The `.Hearts` shorthand works whenever the type is known from context.

## Cases & Payloads

A variant can carry data:

```swift
enum Shape {
    case Circle(radius: Float)
    case Rectangle(width: Float, height: Float)
    case Point
}

let s = Shape.Rectangle(width: 3.0, height: 4.0)
let c = Shape.Circle(radius: 2.5)
```

Payloads can be labeled (as above) or positional:

```swift
enum Result[T, E] {
    case Ok(T)
    case Err(E)
}
```

A variant can hold the enum itself, but only with `indirect`:

```swift
indirect enum Tree[T] {
    case Leaf(T)
    case Node(left: Tree[T], right: Tree[T])
}
```

`indirect` is what tells the compiler the value needs heap indirection — without it, the type would be infinitely sized.

## Exhaustiveness

When you `match` an enum, you have to handle every variant. The compiler refuses to compile if you forget one:

```swift
match shape {
    .Circle(radius) => Float.pi * radius * radius,
    .Rectangle(width, height) => width * height
    // .Point is missing — compile error
}
```

You can use `_` as a catch-all when you genuinely want to default. Don't use it as a way to ignore unhandled variants — when you add a new case to the enum later, an `_` will silently absorb it instead of pointing you at the call sites that need updating.

For the deeper pattern matching story (destructuring, guards, bindings, nested patterns), see [Pattern Matching](pattern-matching.md).

---

[← Subscripts](../structs/subscripts.md) · [↑ The Kestrel Language](../index.md) · [Pattern Matching →](pattern-matching.md)
