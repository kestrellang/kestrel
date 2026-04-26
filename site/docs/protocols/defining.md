# Defining

A protocol declaration lists the methods, properties, and associated types a conforming type must provide.

## Required methods

```swift
protocol Hashable {
    func hash() -> Int
}
```

Any type that conforms to `Hashable` must provide a `hash()` method with that exact signature. The protocol body holds only the signatures — no implementations.

## Required properties

```swift
protocol Named {
    var name: String { get }
}
```

`{ get }` declares a read-only requirement — the conforming type can satisfy it with a stored field, a computed variable, or anything else that produces a `String` when read. `{ get set }` requires it to be writable too.

## Multiple requirements

```swift
protocol Comparable {
    func compare(to other: Self) -> Int
    func equals(other: Self) -> Bool
}
```

`Self` inside a protocol means "the type that ends up conforming." `Comparable.compare(to:)` on `Int` takes another `Int`; on `String`, another `String`.

## Mutating requirements

If a method needs to mutate the conforming value, mark it `mutating`:

```swift
protocol Counter {
    mutating func increment()
    func current() -> Int
}
```

A struct conforming to `Counter` will provide `increment` as a `mutating func`.

## Static requirements

Protocols can require `static` methods too — useful for factories and "default value" patterns:

```swift
protocol Defaultable {
    static func default() -> Self
}
```

## Associated types

Sometimes the protocol depends on a type the conforming type chooses. That's an `associatedtype`:

```swift
protocol Container {
    associatedtype Item
    func count() -> Int
    func get(at index: Int) -> Item
}
```

`Array[T]` conforms to `Container` with `Item = T`; a `Stack[U]` conforms with `Item = U`. Each conformance picks the concrete type. See [Generics → Associated Types](../generics/associated-types.md) for the full story.

---

[← Protocols](index.md) · [↑ Protocols](index.md) · [Conformance →](conformance.md)
