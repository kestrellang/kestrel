# New Collection Types

This document describes new data structures to add to `std.collections`. Each type conforms to `Iterable` and provides its own `count`/`isEmpty`. They use `CowBox[T]` for copy-on-write semantics where appropriate.

---

## Deque[T]

Ring-buffer double-ended queue. O(1) amortized push/pop from both ends.

```
struct Deque[T]: Iterable {
    private var storage: CowBox[DequeStorage[T]]
    // DequeStorage: (ptr, len, cap, head) — ring buffer
}
```

API (overlapping with Array):

```
init()
init(capacity: Int64)

var count: Int64
var isEmpty: Bool

mutating func pushBack(element: T)      // O(1) amortized
mutating func pushFront(element: T)     // O(1) amortized
mutating func popBack() -> T?           // O(1)
mutating func popFront() -> T?          // O(1)

func first() -> T?
func last() -> T?

subscript(index: Int64) -> T            // O(1)
func iter() -> DequeIterator[T]
```

**Slice conformance:** Deque's ring buffer may wrap around, so it can't always provide a single contiguous `ArraySlice[T]`. Options:

1. Deque conforms to `Iterable` only — loses the shared contiguous API
2. Deque provides `asSlices() -> (ArraySlice[T], ArraySlice[T])` as a Deque-specific method
3. Deque linearizes on `asSlice()` — copies into a temp buffer (defeats the purpose)

Recommendation: option 2. Deque conforms to `Iterable` and has an `asSlices()` method for direct buffer access. The `Slice` protocol is strictly for single-contiguous-buffer types.

---

## OrderedDictionary[K, V]

Insertion-order-preserving map. For JSON round-tripping, config files, deterministic output.

```
struct OrderedDictionary[K, V]: Iterable where K: Equatable, K: Hashable {
    // Dense array of (K, V) pairs (insertion order)
    // Hash table mapping K -> index into dense array
}
```

Same API as Dictionary plus:
- Guaranteed iteration in insertion order
- `subscript(position: Int64) -> (K, V)` — access by insertion position
- `mutating func moveToEnd(key: K)` — reorder

Separate type from Dictionary — different performance characteristics (extra indirection on lookup, but better cache locality on iteration).

---

## SortedDictionary[K, V] and SortedSet[T]

B-tree backed. O(log n) insert/lookup/delete. Ordered iteration by key. Range queries.

```
struct SortedDictionary[K, V]: Iterable where K: Comparable {
    // B-tree backing
}

struct SortedSet[T]: Iterable where T: Comparable {
    // B-tree backing (or SortedDictionary[T, Unit])
}
```

Unique capabilities:
- `func range(from: K, to: K) -> SortedDictionarySlice[K, V]` — O(log n + k)
- `func min() -> (K, V)?` — O(log n) or O(1) with cached extrema
- `func max() -> (K, V)?` — O(log n) or O(1) with cached extrema

---

## Heap[T]

Binary heap backed by Array. Min-heap by default (or parameterized by comparator).

```
struct Heap[T]: Iterable where T: Comparable {
    private var data: Array[T]

    init()
    init(from: Array[T])                    // O(n) heapify

    mutating func push(element: T)          // O(log n)
    mutating func pop() -> T?               // O(log n), returns min
    func peek() -> T?                       // O(1)

    var count: Int64
    var isEmpty: Bool
}
```

Small type but commonly needed: Dijkstra, event scheduling, top-K, merge-K-sorted.

---

## BitSet

Compact set of non-negative integers. For flags, membership bitmaps, graph algorithms.

```
struct BitSet: Iterable {
    private var words: Array[UInt64]

    func contains(value: Int64) -> Bool          // O(1)
    mutating func insert(value: Int64)           // O(1)
    mutating func remove(value: Int64)           // O(1)

    var count: Int64                              // popcount
    var isEmpty: Bool

    // Set operations — O(n/64) via bitwise ops
    func union(other: BitSet) -> BitSet
    func intersection(other: BitSet) -> BitSet
    func difference(other: BitSet) -> BitSet
    func symmetricDifference(other: BitSet) -> BitSet

    func iter() -> BitSetIterator               // yields set members in order
}
```

---

## Future: InlineArray (Fixed-Size, Stack-Allocated)

Every `[1, 2, 3]` currently heap-allocates. For small, fixed-size data (coordinates, colors, small lookup tables), this is significant overhead.

```
struct InlineArray[T; N] {
    // N elements stored inline in the struct, no heap allocation
}
```

This requires const generics in the type system — a significant language feature. Without full const generics, alternatives:

1. **Compiler-supported fixed sizes** (2, 3, 4, 8, 16, 32) — covers most use cases
2. **SmallArray[T]** with a fixed inline capacity + overflow to heap — like SmallVec in Rust

This is explicitly a future item, listed here for completeness. The protocol hierarchy should be designed to accommodate it: `InlineArray[T; N]` would conform to `Slice[T]`.
