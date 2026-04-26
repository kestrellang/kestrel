# Dictionaries

`Dict[K, V]` is a hash map: an unordered collection of key-value pairs where each key appears at most once. Keys must conform to `Hashable`.

## Creating

```swift
let ages: Dict[String, Int] = ["alice": 30, "bob": 27]
let empty: Dict[String, Int] = [:]
```

Literal syntax `[k: v, ...]` mirrors the array literal but with `:` between key and value. An empty literal is `[:]`.

## Lookup

Indexing returns `Optional[V]` — keys might not exist:

```swift
let alice = ages["alice"]   // .Some(30)
let mallory = ages["mallory"] // .None

if let .Some(age) = ages["alice"] {
    println("alice is \(age)")
}
```

For a default if missing, use `unwrapOr`:

```swift
let bobAge = ages["bob"].unwrapOr(0)
```

## Inserting and updating

```swift
var scores: Dict[String, Int] = [:]
scores["alice"] = 99       // insert
scores["alice"] = 100      // update
scores.remove(key: "alice")
```

The dict must be `var` for any of these to work. Insertion and update use the same syntax — the dict figures out which it is.

## Iteration

A `for` loop yields key-value pairs:

```swift
for (name, score) in scores {
    println("\(name): \(score)")
}
```

Iteration order is unspecified — don't rely on it. If you need order, sort the keys explicitly.

## Common methods

```swift
scores.count()
scores.isEmpty()
scores.contains(key: "alice")
scores.keys()      // iterator over keys
scores.values()    // iterator over values
```

The iterator chain works on the values:

```swift
let total = scores.values().reduce(0) { acc, v in acc + v }
```

---

[← Arrays](arrays.md) · [↑ Collections](index.md) · [Sets →](sets.md)
