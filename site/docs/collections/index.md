# Collections

Kestrel ships four built-in collection types: ordered (`Array`), keyed (`Dict`), unordered-unique (`Set`), and fixed-size heterogeneous (`Tuple`). All four work with the iterator chain (`map`, `filter`, `reduce`, etc.).

## Quick reference

```swift
let xs: [Int] = [1, 2, 3]                            // Array
let ages: Dict[String, Int] = ["alice": 30]          // Dict
let tags: Set[String] = ["urgent", "pending"]        // Set
let pair: (Int, String) = (42, "answer")             // Tuple
```

The literal syntaxes use `[...]` for arrays, `[k: v, ...]` for dicts, and `(...)` for tuples. Sets use a constructor or a literal-with-context pattern (the type annotation distinguishes from `Array`).

## Iteration

Every collection type works with `for`:

```swift
for x in xs { /* ... */ }
for (key, value) in ages { /* ... */ }
```

And with the iterator chain:

```swift
let evens = xs.filter { it % 2 == 0 }
let sum = xs.reduce(0) { acc, n in acc + n }
```

For the deeper iterator story — laziness, custom iterators, the `Iterator` protocol — see [Iterators](iterators.md).

## Subpages

- [Arrays](arrays.md) — ordered, indexable collections
- [Dictionaries](dictionaries.md) — key-value lookup
- [Sets](sets.md) — unordered collections of unique values
- [Tuples](tuples.md) — fixed-size heterogeneous values
- [Iterators](iterators.md) — `for`, `map`/`filter`/`reduce`, custom iterators

---

[← Control Flow](../control-flow.md) · [↑ The Kestrel Language](../index.md) · [Arrays →](arrays.md)
