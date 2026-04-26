# Inheritance Rules

Protocols can refine other protocols. A type that conforms to the refined protocol automatically owes the requirements of the parent.

## Refinement

```swift
protocol Equatable {
    func equals(other: Self) -> Bool
}

protocol Comparable: Equatable {
    func compare(to other: Self) -> Int
}
```

`Comparable` refines `Equatable`. A type conforming to `Comparable` must satisfy *both* — `equals` and `compare`. Anywhere a function asks for `Equatable`, a `Comparable` value works.

## Combining protocols

A protocol can inherit from many. Stack them:

```swift
protocol Sortable: Comparable, Hashable {
    // Sortable's own requirements, if any
}
```

A `Sortable` type satisfies `Comparable`, `Hashable`, *and* anything Sortable adds.

## Composition without naming

When a function needs more than one capability and you don't want to invent a new protocol, list them:

```swift
func process[T](item: T) where T: Drawable, T: Hashable { /* ... */ }
```

Same effect as creating a `protocol DrawableAndHashable: Drawable, Hashable {}`, without the bookkeeping.

## When to refine, when to compose

- **Refine** when the relationship is permanent and meaningful — `Comparable` types are *always* `Equatable`.
- **Compose** at the use site when you just happen to need two capabilities for one function.

Don't invent intermediate protocols just to give a name to a `where` clause.

---

[← Default Methods](default-methods.md) · [↑ Protocols](index.md) · [Extending →](extending.md)
