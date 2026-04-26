# Tuples

A tuple groups a fixed number of values, possibly of different types, into a single value. Use them when you need to return more than one thing from a function and the combination doesn't deserve its own struct.

## Creating

```swift
let pair = (42, "answer")
let triple: (Int, Int, Int) = (1, 2, 3)
let labeled = (x: 3, y: 4)
```

Type is `(Int, String)` for the first; tuple types are written the same way they're constructed.

## Access

By position, with `.0`, `.1`, etc.:

```swift
println(pair.0)   // 42
println(pair.1)   // "answer"
```

Or by destructuring:

```swift
let (number, name) = pair
println("\(name): \(number)")
```

Labeled tuples can also be accessed by name:

```swift
let point = (x: 3, y: 4)
point.x   // 3
point.y   // 4
```

## Returning multiple values

The most common use:

```swift
func minMax(of xs: [Int]) -> (Int, Int) {
    var lo = xs[0]
    var hi = xs[0]
    for n in xs {
        if n < lo { lo = n }
        if n > hi { hi = n }
    }
    (lo, hi)
}

let (low, high) = minMax(of: [3, 1, 4, 1, 5, 9, 2, 6])
```

When the result has more than two or three elements, or the meaning isn't clear from position alone, prefer a struct — readers shouldn't have to remember whether `result.0` is the count or the average.

## In pattern matching

Tuples destructure in `match` and `if let`:

```swift
match (x, y) {
    (0, 0) => "origin",
    (_, 0) => "on x-axis",
    (0, _) => "on y-axis",
    _ => "elsewhere"
}
```

This is what makes `for (key, value) in dict` work — the tuple destructures into named bindings.

---

[← Sets](sets.md) · [↑ Collections](index.md) · [Iterators →](iterators.md)
