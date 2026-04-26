# Arrays

`Array[T]` (or its shorthand `[T]`) holds an ordered, indexable, growable sequence of values of one type.

## Creating

```swift
let xs: [Int] = [1, 2, 3]
let empty: [String] = []
let zeroes: [Int] = Array(repeating: 0, count: 10)
```

The literal syntax `[a, b, c]` infers the element type from the contents; an empty literal needs the type written.

## Indexing

```swift
let first = xs[0]      // 1
xs[0] = 99             // requires xs to be `var`
```

Out-of-bounds access is a runtime error, not a compile-time one — guard with bounds checks if the index might be wrong:

```swift
if i < xs.count() {
    let value = xs[i]
}
```

For a guaranteed-safe lookup, use `xs.get(at: i)` which returns `Optional[T]`.

## Common methods

```swift
xs.count()           // number of elements
xs.isEmpty()         // count == 0
xs.first()           // Optional[T]
xs.last()            // Optional[T]

xs.append(4)         // mutates; xs must be `var`
xs.insert(0, at: 0)
xs.remove(at: 1)
xs.removeLast()      // returns Optional[T]

xs.contains(2)       // Bool
xs.firstIndex(of: 2) // Optional[Int]

xs.sort()            // requires T: Comparable, mutates
xs.sorted()          // non-mutating, returns a new Array
xs.reverse()
xs.reversed()
```

## Iterator chain

Arrays plug into the iterator chain. See [Iterators](iterators.md) for the full set:

```swift
let doubled = xs.map { it * 2 }
let evens = xs.filter { it % 2 == 0 }
let sum = xs.reduce(0) { acc, n in acc + n }
let any = xs.any { it > 100 }
```

Most iterator methods are lazy until a terminal operation forces them — chains stay efficient even on long arrays.

---

[← Collections](index.md) · [↑ Collections](index.md) · [Dictionaries →](dictionaries.md)
