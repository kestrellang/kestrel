# Conformance

A type conforms to a protocol by stating it does, then providing the required members.

## Stating conformance

The conformance goes on an `extend` block:

```swift
struct Point {
    let x: Int
    let y: Int
}

extend Point: Hashable {
    public func hash() -> Int {
        self.x * 31 + self.y
    }
}
```

Now any code that takes a `Hashable` accepts `Point`.

## Multiple conformances

A type can conform to many protocols. Stack them in one extension or split across several:

```swift
extend Point: Hashable, Equatable, Drawable {
    public func hash() -> Int { /* ... */ }
    public func equals(other: Point) -> Bool { /* ... */ }
    public func draw() { /* ... */ }
}
```

## Conformance via existing methods

If a type already has methods that match the protocol's signatures, the extension just states the conformance — you don't have to repeat the implementations:

```swift
extend Stack: Container {}   // Stack already has count() and get(at:)
```

The compiler matches existing members against the requirements automatically.

## Conditional conformance

Sometimes a generic type only conforms when its type parameter does. Use a `where` clause:

```swift
extend Box[T]: Hashable where T: Hashable {
    public func hash() -> Int {
        self.value.hash()
    }
}
```

`Box[Int]` is `Hashable` because `Int` is. `Box[Connection]` isn't, because `Connection` isn't.

## Where to put the extension

Conformances can live in the same file as the type, in the same file as the protocol, or anywhere else — including a different module. This is the affordance that lets you make a foreign type conform to a protocol you control:

```swift
import std.collections.Array

extend Array[Int]: Sortable {
    public func sort() -> Array[Int] { /* ... */ }
}
```

That's a powerful tool. Use it when the conformance genuinely belongs to your code; avoid it when it's something the upstream module should own.

---

[← Defining](defining.md) · [↑ Protocols](index.md) · [Default Methods →](default-methods.md)
