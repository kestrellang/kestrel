# Where Clauses

A `where` clause constrains type parameters. You write it after the parameter list (or after the type, for type-level constraints) and the compiler treats anything inside as a precondition.

## Conformance constraints

```swift
func deduplicate[T](items: [T]) -> [T] where T: Hashable {
    // ...
}
```

Reads: "deduplicate works on any `T`, as long as `T` is `Hashable`." Inside the body you can call `.hash()` on any `T` because the constraint guarantees it.

## Multiple constraints

Stack with commas:

```swift
func sortAndCount[T](items: [T]) -> Int
    where T: Comparable, T: Hashable
{
    // ...
}
```

Or, equivalently, refine a protocol that includes both:

```swift
protocol Sortable: Comparable, Hashable {}

func sortAndCount[T](items: [T]) -> Int where T: Sortable { /* ... */ }
```

Reach for refinement when the combination is meaningful and reused. Reach for `where T: A, T: B` when it's a one-off.

## Equality constraints

A `where` clause can also pin associated types:

```swift
func zip[A, B](a: A, b: B) -> [(A.Item, B.Item)]
    where A: Container, B: Container, A.Item == B.Item
{
    // ...
}
```

`A.Item == B.Item` says the two containers must hold the same kind of element. The compiler enforces it at the call site.

## On extensions

The same `where` syntax constrains an extension to apply only when conditions are met:

```swift
extend Container where Item: Comparable {
    public func max() -> Optional[Item] { /* ... */ }
}
```

`max()` shows up on `Container[Int]` but not `Container[Connection]`.

## When constraints get complicated

If your `where` clause is more than three lines, the abstraction is probably trying to do too much. Either:

- Split the function into a more specific version, or
- Define a protocol that captures the combination ("X, Y, and Z all hold; let's call that Foo").

A long constraint list is the abstraction begging you for a name.

---

[← Generics](index.md) · [↑ Generics](index.md) · [Associated Types →](associated-types.md)
