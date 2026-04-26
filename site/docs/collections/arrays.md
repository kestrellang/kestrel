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
let first = xs(0)      // 1
xs(0) = 99             // requires xs to be `var`
```

The default subscript panics on out-of-bounds. For a non-panicking lookup, use the `checked:` variant which returns `Optional[T]`:

```swift
if let .Some(v) = xs(checked: i) {
    // ...
}
```

Other subscript variants cover common access patterns:

```swift
xs(unchecked: i)        // skips the bounds check (UB if out of range)
xs(wrapping: -1)        // Optional[T]; negative/overflow wraps modulo count
xs(clamping: 100)       // Optional[T]; saturates to first/last
xs(0..<3)               // Slice[T]; panics if range is out of bounds
xs(checked: 0..<4)     // Optional[Slice[T]]
xs(clamping: -5..<100)    // Slice[T]; clamps the range to valid bounds
```

## Common methods

```swift
xs.count             // number of elements
xs.isEmpty           // count == 0
xs.first()           // Optional[T]
xs.last()            // Optional[T]

xs.append(element: 4)            // mutates; xs must be `var`
xs.insert(element: 0, at: 0)
xs.remove(at: 1)                 // returns T
xs.pop()                         // returns Optional[T]; removes the last
xs.popFirst()                    // returns Optional[T]; removes the first

xs.contains(element: 2)          // Bool
xs.firstIndex(of: 2)             // Optional[Int64]

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
