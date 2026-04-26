# Associated Types

An associated type is a type *member* of a protocol. The protocol declares that a type exists; each conforming type picks a concrete one.

## Declaring

```swift
protocol Container {
    associatedtype Item

    func count() -> Int
    func get(at index: Int) -> Item
}
```

`Item` is a placeholder. Any type conforming to `Container` will choose what `Item` becomes.

## Conforming

```swift
struct Stack[T] {
    var items: [T]
}

extend Stack: Container {
    public func count() -> Int { self.items.count() }
    public func get(at index: Int) -> T { self.items[index] }
}
```

Kestrel infers `Item = T` from the signatures — `get(at:)` returns `T`, so `Item` must be `T`. You can also state it explicitly:

```swift
extend Stack: Container {
    typealias Item = T
    // ...
}
```

## Using associated types in generic code

Refer to a protocol's associated type with dotted access:

```swift
func first[C](in container: C) -> Optional[C.Item] where C: Container {
    if container.count() > 0 {
        .Some(container.get(at: 0))
    } else {
        .None
    }
}
```

`C.Item` is whatever the conforming type picked. `first` works on `Stack[User]` (returning `Optional[User]`) and `Stack[Int]` (returning `Optional[Int]`).

## Constraining associated types

A `where` clause can pin an associated type:

```swift
func max[C](in container: C) -> Optional[C.Item]
    where C: Container, C.Item: Comparable
{
    // ...
}
```

This is what lets generic code do real work: combining "the item type whatever it is" with "and that item type can be compared."

## Why bother?

The alternative would be making every protocol generic — `Container[T]` instead of `Container { associatedtype Item }`. That works but forces every consumer of the protocol to thread `T` through. Associated types let the protocol stay un-parameterized at the call site while still being type-safe.

A useful rule: if the type parameter is part of *what the protocol means*, it's an associated type. If it's part of *which protocol you want*, it's a generic parameter.

---

[← Where Clauses](where-clauses.md) · [↑ Generics](index.md) · [Extending Types →](../extending-types.md)
