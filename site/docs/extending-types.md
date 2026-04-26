# Extending Types

Two ways to shape types from outside their definition: **extensions** add behavior, **type aliases** name something that already exists.

## Extensions

`extend` adds methods, computed variables, or protocol conformance to any type — including types you didn't define:

```swift
import std.collections.Array

extend Array[Int] {
    func sum() -> Int {
        var total = 0
        for n in self { total = total + n }
        total
    }
}

[1, 2, 3].sum()   // 6
```

This works on stdlib types, your own types, and types from third-party modules. The extension lives in whatever file declares it; everywhere that file is imported, `sum()` is available on `Array[Int]`.

You can also add **conformance** through an extension:

```swift
extend Point: Hashable {
    public func hash() -> Int {
        self.x * 31 + self.y
    }
}
```

Anywhere a function takes a `Hashable`, your `Point` now works.

For protocol-side extensions (adding behavior to all conformers of a protocol), see [Protocols → Extending](protocols/extending.md).

## Type aliases

A type alias gives a new name to an existing type:

```swift
typealias UserId = Int
typealias Handler = (Request) -> Response
typealias StringDict[V] = Dict[String, V]
```

Aliases are *not* new types — they're just names. `UserId` is `Int`, with all the same methods and constraints. Don't reach for an alias when you want a distinct type with different rules; use a struct:

```swift
struct UserId {
    let raw: Int
}   // distinct from Int — can't accidentally pass a row count where a UserId is expected
```

The two together are powerful: aliases for "this is a kind of Int we use a lot," structs for "this is a different thing that happens to be wrapped around an Int."

---

[← Associated Types](generics/associated-types.md) · [↑ The Kestrel Language](index.md) · [Organization →](organization.md)
