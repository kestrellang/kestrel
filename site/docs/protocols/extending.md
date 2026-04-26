# Extending

Protocol extensions add behavior to all conforming types in one place.

## Adding methods to every conformer

```swift
protocol Counter {
    var value: Int { get }
}

extend Counter {
    public func isZero() -> Bool {
        self.value == 0
    }

    public func isPositive() -> Bool {
        self.value > 0
    }
}
```

Every type that conforms to `Counter` now has `isZero` and `isPositive` for free, no extra work at the conformance site.

## Constrained extensions

You can add behavior only when conditions are met. Use a `where` clause:

```swift
extend Container where Item: Comparable {
    public func max() -> Optional[Item] { /* ... */ }
}
```

`max()` exists on `Container[Int]` (because `Int` is `Comparable`) but not on `Container[File]` (assuming `File` isn't). The compiler tracks this at the type level — calling `max()` on a non-`Comparable` container is a compile error, not a runtime one.

## Extension vs default method

These look similar but have different intents:

- A **default method** ([Default Methods](default-methods.md)) supplies a fallback for a *required* member. Conforming types can override it.
- An **extension method** adds a *new* member that isn't part of the protocol's contract. Conforming types can't override it — extension methods aren't dispatched virtually.

Pick a default method when the operation is genuinely part of the protocol and individual types might need to specialize. Pick an extension method when the operation is convenience, derived from existing requirements, and shouldn't be customized per type.

For struct-side extensions (adding methods to your own types or stdlib types), see [Extending Types](../extending-types.md).

---

[← Inheritance Rules](inheritance-rules.md) · [↑ Protocols](index.md) · [Generics →](../generics/index.md)
