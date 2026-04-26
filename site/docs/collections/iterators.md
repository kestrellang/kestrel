# Iterators

An **iterator** produces a sequence of values, one at a time, on demand. Every collection in Kestrel exposes one, and the iterator chain (`map`, `filter`, `reduce`, etc.) is how you transform data without writing explicit loops.

## `for` loops

The simplest way to consume an iterator:

```swift
for x in xs {
    process(x)
}
```

`for` works on anything that conforms to the `Iterable` protocol — arrays, dicts, sets, ranges, custom types you write yourself.

## The iterator chain

Methods that transform an iterator return another iterator. Methods that consume one return a value.

**Transformations** (return an iterator):

```swift
xs.map { it * 2 }
xs.filter { it > 0 }
xs.take(5)
xs.skip(2)
xs.flatMap { array -> array }
xs.zip(ys)
```

**Terminal operations** (return a value):

```swift
xs.reduce(0) { acc, x in acc + x }
xs.toArray()
xs.count()
xs.first()           // Optional[T]
xs.firstWhere { it > 10 }   // Optional[T]
xs.any { it < 0 }
xs.all { it > 0 }
xs.contains(42)
xs.sum()             // requires T: Numeric
xs.max()             // requires T: Comparable, returns Optional[T]
```

A typical chain:

```swift
let totalAdult = users
    .filter { it.age >= 18 }
    .map { it.score }
    .sum()
```

## Laziness

Transformations are **lazy** — they don't do work until a terminal operation forces them. The chain above doesn't allocate a filtered array, then a mapped array, then sum — it pulls one user at a time, filters, maps, accumulates. For long sequences this is a big win.

## Custom iterators

Define your own by conforming to `Iterator` (and usually `Iterable`):

```swift
struct Countdown {
    var current: Int
}

extend Countdown: Iterator {
    associatedtype Element = Int

    public mutating func next() -> Optional[Int] {
        if self.current < 0 {
            .None
        } else {
            let value = self.current
            self.current = self.current - 1
            .Some(value)
        }
    }
}

for n in Countdown(current: 5) {
    println(n)   // 5, 4, 3, 2, 1, 0
}
```

`next()` returns `Optional[Element]` — `None` signals the sequence is exhausted. The rest of the iterator chain works automatically.

---

[← Tuples](tuples.md) · [↑ Collections](index.md) · [Structs →](../structs/index.md)
