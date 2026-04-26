# Generics

A generic function or type is parameterized over one or more types. The same code works on `Int`, `String`, `User`, or anything else, with the compiler checking each use is type-correct.

## Generic functions

```swift
func identity[T](value: T) -> T {
    value
}

identity(42)         // T = Int
identity("hello")    // T = String
```

`[T]` after the name introduces a type parameter. Inside the function, `T` is just a stand-in for whatever the caller passed.

## Generic types

A struct or enum can be generic too:

```swift
struct Box[T] {
    var value: T
}

let intBox = Box(value: 42)        // Box[Int]
let stringBox = Box(value: "hi")   // Box[String]
```

Use one type parameter per "kind of thing" the type holds. Multiple parameters are written `[K, V]` and so on.

## Constraints

Pure generics work on any type. Once a function needs to *do* something with a type parameter (compare, hash, draw), constrain it to a protocol:

```swift
func deduplicate[T](items: [T]) -> [T] where T: Hashable {
    // can call .hash() on any T because of the constraint
}
```

The `where` clause is how Kestrel says "T can be anything, as long as it's `Hashable`." See [Where Clauses](where-clauses.md) for the longer story.

## Associated types

Sometimes a protocol has a type that depends on the conforming type — `Container.Item`, `Iterator.Element`. Those are **associated types**, declared with `associatedtype` inside the protocol. See [Associated Types](associated-types.md).

## When to reach for generics

- The code does the same thing for many types and the operations are protocol-defined.
- A collection holds many of one kind of thing, and the kind shouldn't be hardcoded.
- A function takes a callback and you want to preserve the type through the call.

If you find yourself with one generic that has so many constraints it's effectively a single concrete type, you didn't need a generic.

## Subpages

- [Where Clauses](where-clauses.md) — protocol constraints, equality constraints, complex predicates
- [Associated Types](associated-types.md) — type members of protocols

---

[← Extending](../protocols/extending.md) · [↑ The Kestrel Language](../index.md) · [Where Clauses →](where-clauses.md)
