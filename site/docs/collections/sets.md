# Sets

`Set[T]` is an unordered collection of unique values. Like `Dict`, the element type must conform to `Hashable`.

## Creating

```swift
let tags: Set[String] = ["urgent", "pending", "review"]
let empty: Set[Int] = Set()
```

Set literals use the same `[a, b, c]` syntax as arrays — the type annotation is what tells the compiler which to make.

## Membership

```swift
tags.contains("urgent")    // true
tags.contains("draft")     // false
```

Constant-time lookup, since Sets are hash-backed.

## Inserting and removing

```swift
var tags: Set[String] = []
tags.insert("urgent")
tags.insert("urgent")    // no-op; already present
tags.remove("urgent")
```

Inserting an existing value is a silent no-op — that's the point of a set.

## Set algebra

```swift
let a: Set[Int] = [1, 2, 3]
let b: Set[Int] = [3, 4, 5]

a.union(b)           // {1, 2, 3, 4, 5}
a.intersection(b)    // {3}
a.difference(b)      // {1, 2}
a.isSubsetOf(b)      // false
```

These operations don't mutate; they return a new set.

## Iteration

```swift
for tag in tags {
    println(tag)
}
```

Order is unspecified.

## When to reach for Set vs Array

- **Set** when membership testing, deduplication, or set algebra is the primary operation, and order doesn't matter.
- **Array** when order matters, duplicates are allowed, or you need indexed access.

If you find yourself calling `array.contains(...)` a lot, the data probably wanted to be a `Set`.

---

[← Dictionaries](dictionaries.md) · [↑ Collections](index.md) · [Tuples →](tuples.md)
