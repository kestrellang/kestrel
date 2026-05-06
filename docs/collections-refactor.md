# Collections Library Refactor

This document describes a redesign of `std.collections` and `std.memory` that introduces a `Collection` / `Seq` protocol hierarchy, promotes `Slice[T]` to a first-class collection type, replaces single-pass iterators with multi-pass views, adds `CowBox[T]` as reusable COW infrastructure, adds `ArrayBuilder[T]` for zero-overhead construction, and fills structural gaps (Deque, OrderedDictionary, Heap, BitSet, sorted collections). The design follows the same principles as the `std.text` string refactor: one protocol, one kernel method, all read-only logic in a protocol extension, the borrowed type as the universal return.

## Motivation

The current `std.collections` API has several structural problems:

1. **No shared collection abstraction.** Array, Slice, Dictionary, Set, and String views all independently implement `count`, `isEmpty`, `first()`, `last()`, subscripting, and iteration. There is no way to write a generic function over "anything with elements."

2. **Slice is severely undercooked.** `Slice[T]` is a bare `(pointer, count)` pair with almost no methods. Its `Equatable` conformance is broken (compares length only, not elements). It doesn't conform to `Iterable`. It lives in `std.memory`, signaling "low-level" when it should be the standard borrowed-collection type.

3. **Massive code duplication.** Six internal index protocols (`ArrayIndex`, `ArrayClampable`, `ArrayWrappable`, `SliceIndex`, `SliceClampable`, `SliceWrappable`) mirror each other exactly. Each has conformances for `Int64`, `Range[Int64]`, and `ClosedRange[Int64]`, producing ~600 lines of near-identical code.

4. **ArrayIterator and SliceIterator are identical.** Both are `(Pointer[T], Int64)` with the same `next()` body. Two types doing the same thing.

5. **Chunks and windows are iterators, not views.** `arr.chunks(of: 2)` returns a `ChunksIterator` — single-pass, no subscripting, no `count`. Once consumed, you must re-create it. The string refactor's insight is that these should be multi-pass views you can subscript and iterate repeatedly.

6. **COW is copy-pasted.** Array, String, and Dictionary all independently implement the same `makeUnique()` / `RcBox` / `clone()` pattern. Every new collection will copy-paste it again.

7. **`collect()` only produces Array.** There's no way to materialize an iterator into a Dictionary, Set, String, or any other container.

8. **Sort is O(n^2).** Array's `sort(by:)` uses insertion sort.

9. **No Deque.** `Array.popFirst()` is O(n). There's no O(1)-amortized double-ended queue.

10. **No ordered or sorted map.** Dictionary has unspecified iteration order. There's no insertion-order map or sorted-key map.

11. **No small-buffer / inline array.** Every array heap-allocates, even `[1, 2]`.

## Design Overview

### Protocol Hierarchy

```
Iterable                    <- has iter()
  +-- Collection             <- adds count, isEmpty
       +-- Seq[T]            <- contiguous: asSlice(), subscript(Int64)
       |    +-- Array[T]
       |    +-- Slice[T]
       |    +-- InlineArray[T, N]
       |    +-- ArrayBuilder[T] (read-only view during construction)
       +-- KeyedCollection   <- keyed access: subscript(Key)
            +-- Dictionary[K, V]
            +-- OrderedDictionary[K, V]
            +-- Set[T]
```

### Type Inventory

| Type | Role | New? |
|------|------|------|
| `Collection` | Protocol: count, isEmpty, iteration | New |
| `Seq[T]` | Protocol: contiguous read-only API via `asSlice()` | New |
| `Slice[T]` | Non-owning contiguous view, first-class collection | Exists, promoted |
| `Array[T]` | Owning growable array, COW | Exists, trimmed |
| `ArrayBuilder[T]` | Write-only construction buffer, no COW | New |
| `CowBox[T]` | Reusable copy-on-write wrapper | New |
| `ChunksView[T]` | Multi-pass lazy view over chunks | New (replaces `ChunksIterator`) |
| `WindowsView[T]` | Multi-pass lazy view over windows | New (replaces `WindowsIterator`) |
| `ReversedView[T]` | Multi-pass lazy reversed view | New |
| `SplitView[T]` | Multi-pass lazy view over predicate splits | New |
| `Deque[T]` | Ring-buffer double-ended queue | New |
| `OrderedDictionary[K, V]` | Insertion-order-preserving map | New |
| `SortedDictionary[K, V]` | B-tree sorted map | New |
| `SortedSet[T]` | B-tree sorted set | New |
| `Heap[T]` | Binary min/max heap | New |
| `BitSet` | Compact set of non-negative integers | New |
| `Collectible` | Protocol: materialize iterators into containers | New |

---

## Part 1: The Protocol Hierarchy

### Collection

The broadest collection protocol. Captures what every collection shares: element count, emptiness check, iteration.

```
protocol Collection: Iterable {
    var count: Int64
    var isEmpty: Bool
}
```

No indexing requirement — Dictionary isn't integer-indexed. This lets you write:

```
func logSize[C](c: C) where C: Collection {
    print("contains \{c.count} items")
}
```

And it works for Array, Slice, Dictionary, Set, Deque, any future type.

### Seq[T]

The contiguous-collection protocol. Mirrors `Str` from the string refactor: one kernel method, all read-only logic in the extension.

```
protocol Seq[T]: Collection {
    func asSlice() -> Slice[T]
}
```

Array conforms with `func asSlice() -> Slice[T] { Slice(pointer: self.ptr(), count: self.len()) }`. Slice conforms with `func asSlice() -> Slice[T] { self }`.

### Seq Protocol Extension

All read-only methods defined once. Both Array and Slice inherit them automatically:

```
extend Seq {
    // Size
    var count: Int64 { self.asSlice().count }
    var isEmpty: Bool { self.asSlice().isEmpty }
    var indices: Range[Int64] { 0..<self.count }

    // Element access
    func first() -> T? { ... }
    func last() -> T? { ... }

    // Subscripts (via unified SeqIndex protocol — see Part 3)
    subscript[I](index: I) -> I.SeqYield where I: SeqIndex[T] { ... }
    subscript[I](checked index: I) -> I.SeqYield? where I: SeqIndex[T] { ... }
    subscript[I](unchecked index: I) -> I.SeqYield where I: SeqIndex[T] { ... }
    subscript[I](clamped index: I) -> I.SeqClampedYield where I: SeqClampable[T] { ... }
    subscript[I](wrapped index: I) -> I.SeqWrappedYield where I: SeqWrappable[T] { ... }

    // Slicing
    func prefix(count: Int64) -> Slice[T] { ... }
    func suffix(count: Int64) -> Slice[T] { ... }
    func drop(first count: Int64) -> Slice[T] { ... }
    func drop(last count: Int64) -> Slice[T] { ... }

    // Views (multi-pass, subscriptable — see Part 4)
    func chunks(of size: Int64) -> ChunksView[T] { ... }
    func windows(of size: Int64) -> WindowsView[T] { ... }
    func reversed() -> ReversedView[T] { ... }
    func split(matching predicate: (T) -> Bool) -> SplitWhereView[T] { ... }

    // Iteration
    func iter() -> SliceIterator[T] { self.asSlice().iter() }

    // Search (requires T: Equatable)
    // Defined in conditional extension: extend Seq where T: Equatable

    // Pointer access (for FFI)
    func asPointer() -> Pointer[T] { self.asSlice().pointer }

    // Validation
    func isValidIndex(index: Int64) -> Bool { ... }
}
```

Conditional extensions add methods when element constraints are met:

```
extend Seq where T: Equatable {
    func contains(element: T) -> Bool { ... }
    func firstIndex(of element: T) -> Int64? { ... }
    func lastIndex(of element: T) -> Int64? { ... }
    func starts(with prefix: Slice[T]) -> Bool { ... }
    func ends(with suffix: Slice[T]) -> Bool { ... }
    func split(separator: T) -> SplitView[T] { ... }
    func isEqual(to other: Self) -> Bool { ... }  // element-wise
    func dedup() -> Array[T] { ... }
}

extend Seq where T: Comparable {
    func min() -> T? { ... }
    func max() -> T? { ... }
    func isSorted() -> Bool { ... }
    func binarySearch(element: T) -> Int64? { ... }
    func sorted() -> Array[T] { ... }
}

extend Seq where T: Formattable {
    func format(options: FormatOptions) -> String { ... }
}

extend Seq where T: Hash {
    func unique() -> Array[T] { ... }
}
```

### What Moves Off of Array

| Current Array method | New location | Notes |
|---|---|---|
| `count`, `isEmpty`, `capacity` | `count`/`isEmpty` in `extend Seq`; `capacity` stays on Array | capacity is Array-specific |
| `first()`, `last()` | `extend Seq` | |
| All subscript variants | `extend Seq` via `SeqIndex` | single protocol, not six |
| `prefix`, `suffix`, `drop` | `extend Seq` | |
| `chunks(of:)`, `windows(of:)` | `extend Seq`, returns views | |
| `contains`, `firstIndex(of:)`, etc. | `extend Seq where T: Equatable` | |
| `min()`, `max()`, `sorted()`, etc. | `extend Seq where T: Comparable` | |
| `isEqual(to:)` | `extend Seq where T: Equatable` | |
| `asPointer()`, `asSlice()` | `asSlice()` is the `Seq` kernel; `asPointer()` in `extend Seq` | |
| `iter()` | `extend Seq` | uses `SliceIterator[T]` |

### What Stays on Array (Mutating + Construction Only)

```
struct Array[T]: Seq, Cloneable, Defaultable, ExpressibleByArrayLiteral {
    // Kernel
    func asSlice() -> Slice[T]

    // Constructors
    init()
    init(capacity: Int64)
    init(repeating: T, count: Int64)
    init[I](from: I) where I: Iterable, I.Item = T
    init(of: Int64, generatedBy: (Int64) -> T)
    init(arrayLiteral: LiteralSlice[T])

    // Capacity management
    var capacity: Int64
    mutating func reserveCapacity(minimumCapacity: Int64)
    mutating func shrinkToFit()

    // Adding elements
    mutating func append(element: T)
    mutating func append(contentsOf: Array[T])
    mutating func appendFrom[I](iterable: I) where I: Iterable, I.Item = T
    mutating func insert(element: T, at: Int64)

    // Removing elements
    mutating func pop() -> T?
    mutating func popFirst() -> T?
    mutating func remove(at: Int64) -> T
    mutating func removeSubrange(range: Range[Int64])
    mutating func clear()
    mutating func retain(where: (T) -> Bool)
    mutating func removeAll(where: (T) -> Bool)

    // Reordering
    mutating func swap(at: Int64, with: Int64)
    mutating func reverse()
    mutating func rotate(by: Int64)
    mutating func replaceSubrange(range: Range[Int64], with: Array[T])
    mutating func shuffle()
    mutating func partition(by: (T) -> Bool) -> Int64

    // Sorting (mutating; non-mutating sorted() is in extend Seq)
    mutating func sort() where T: Comparable
    mutating func sort(by: (T, T) -> Bool)
    mutating func sort[K](byKey: (T) -> K) where K: Comparable

    // Everything else: inherited from extend Seq
}
```

---

## Part 2: Slice as First-Class Collection

Just as the string refactor makes `StringSlice` the central type that views operate on and all read paths return, `Slice[T]` becomes the collection equivalent.

### Current Problems

1. **Broken equality.** `isEqual` only checks length — `[1,2,3] == [4,5,6]` if same length.
2. **No Iterable conformance.** Has `iter()` but doesn't declare `Iterable`, so generic `I: Iterable` code rejects Slices.
3. **No Formattable.** Can't print a slice.
4. **Missing methods.** No `contains`, `sort`, `fill`, `copyFrom`, `reversed`, `chunks`, `windows`, `split`.
5. **Lives in `std.memory`.** Signals "low-level" when it should be the standard borrowed-collection type.

### Design

Slice conforms to `Seq[T]` and inherits all read-only methods from the protocol extension. Its `asSlice()` returns `self`. Additionally, Slice gets methods that are specific to mutable non-owning views:

```
struct Slice[T]: Seq, ArrayMatchable {
    private var ptr: Pointer[T]
    private var len: Int64

    // Seq kernel
    func asSlice() -> Slice[T] { self }

    // Slice-specific (mutable, writes through pointer)
    func fill(with value: T) { ... }
    func copyFrom(source: Slice[T]) { ... }
    mutating func sort(by: (T, T) -> Bool) where T: Comparable { ... }

    // Sub-slicing (O(1), pointer arithmetic)
    func slice(from: Int64, to: Int64) -> Slice[T] { ... }
}
```

All subscript variants, `count`, `isEmpty`, `first()`, `last()`, `contains()`, `iter()`, `chunks()`, `windows()`, `prefix()`, `suffix()`, `isEqual()`, `format()`, etc. come from `extend Seq`.

### Ownership Model

Slice stays as a bare `(pointer, count)` — no RcBox, no lifetime tracking. This is a deliberate difference from `StringSlice`:

- **StringSlice** holds a shared `RcBox` to prevent dangling. This works because String mutations are relatively rare — the permanent COW barrier is acceptable.
- **Slice[T]** stays non-owning because Array mutations (append, sort, etc.) are common. If Slice held a reference to Array's RcBox, the refcount would always be > 1, and every mutation would trigger a deep copy. That's unacceptable for a systems language.

The trade-off: Slice is a transient borrow. Don't store it in data structures or return it across call boundaries without ensuring the source outlives it. This matches Rust's `&[T]` (enforced by lifetimes), Go's slices (accepted risk), and C++'s `std::span` (accepted risk).

### File Location

Slice should eventually move from `std.memory` to `std.collections` (or be re-exported from there). Conceptually it's a collection type, not a memory primitive. The `std.memory` module keeps `RawPointer`, `Pointer[T]`, `Layout`, and `Buffer[T, A]` — the truly low-level types.

---

## Part 3: Unified Index Protocols

The current six index protocols (`ArrayIndex[T]`, `ArrayClampable[T]`, `ArrayWrappable[T]`, `SliceIndex[T]`, `SliceClampable[T]`, `SliceWrappable[T]`) collapse into three:

```
internal protocol SeqIndex[T] {
    type SeqYield
    func readSeq[S](from seq: S) -> SeqYield where S: Seq[T]
    func readSeqChecked[S](from seq: S) -> SeqYield? where S: Seq[T]
    func readSeqUnchecked[S](from seq: S) -> SeqYield where S: Seq[T]
    func writeSeq[S](mutating to seq: S, with value: SeqYield) where S: Seq[T]
    func writeSeqUnchecked[S](mutating to seq: S, with value: SeqYield) where S: Seq[T]
}

internal protocol SeqClampable[T] {
    type SeqClampedYield
    func readSeqClamped[S](from seq: S) -> SeqClampedYield where S: Seq[T]
    func writeSeqClamped[S](mutating to seq: S, with value: SeqClampedYield) where S: Seq[T]
}

internal protocol SeqWrappable[T] {
    type SeqWrappedYield
    func readSeqWrapped[S](from seq: S) -> SeqWrappedYield where S: Seq[T]
    func writeSeqWrapped[S](mutating to seq: S, with value: SeqWrappedYield) where S: Seq[T]
}
```

All implementations operate on `seq.asSlice()` internally — the pointer + count is available from any `Seq` conformer. Conformances for `Int64`, `Range[Int64]`, and `ClosedRange[Int64]` are written once each.

**Trade-off:** Write subscripts need care — writing through a Slice is direct pointer access (no COW), while writing through an Array needs `makeUnique()` first. The `mutating to seq` parameter handles this: Array's subscript setter calls `makeUnique()` before dispatching to the index protocol. The index protocol's write methods take a `Seq` that has already been through any necessary COW barrier.

---

## Part 4: Views as Lenses

The string refactor's key insight: **views can be subscripted, counted, and iterated multiple times. Iterators are single-pass and discarded.**

### ChunksView

```
struct ChunksView[T] {
    slice: Slice[T]
    chunkSize: Int64
}
```

| Member | Type | Complexity |
|--------|------|-----------|
| `count` | `Int64` | O(1) — `ceil(slice.count / chunkSize)` |
| `isEmpty` | `Bool` | O(1) |
| subscript(i) | `Slice[T]` | O(1) |
| `first()` | `Slice[T]?` | O(1) |
| `last()` | `Slice[T]?` | O(1) |
| `iter()` | `ChunksIterator[T]` | yields `Slice[T]` |

### WindowsView

```
struct WindowsView[T] {
    slice: Slice[T]
    windowSize: Int64
}
```

| Member | Type | Complexity |
|--------|------|-----------|
| `count` | `Int64` | O(1) — `max(slice.count - windowSize + 1, 0)` |
| `isEmpty` | `Bool` | O(1) |
| subscript(i) | `Slice[T]` | O(1) |
| `first()` | `Slice[T]?` | O(1) |
| `last()` | `Slice[T]?` | O(1) |
| `iter()` | `WindowsIterator[T]` | yields `Slice[T]` |

### ReversedView

```
struct ReversedView[T] {
    slice: Slice[T]
}
```

| Member | Type | Complexity |
|--------|------|-----------|
| `count` | `Int64` | O(1) |
| subscript(i) | `T` | O(1) — reads `slice[count - 1 - i]` |
| `iter()` | `ReversedSliceIterator[T]` | yields `T` back-to-front |

### SplitView

```
struct SplitView[T] where T: Equatable {
    slice: Slice[T]
    separator: T
}
```

| Member | Type | Complexity |
|--------|------|-----------|
| `count` | `Int64` | O(n) |
| `isEmpty` | `Bool` | O(1) |
| `first()` | `Slice[T]?` | O(n)* |
| `last()` | `Slice[T]?` | O(n) |
| `iter()` | `SplitIterator[T]` | yields `Slice[T]` |
| `collect()` | `Array[Slice[T]]` | O(n), materializes all segments |

\* O(scan to first separator), not O(whole collection).

### SplitWhereView

```
struct SplitWhereView[T] {
    slice: Slice[T]
    predicate: (T) -> Bool
}
```

| Member | Type | Complexity |
|--------|------|-----------|
| `count` | `Int64` | O(n) |
| `isEmpty` | `Bool` | O(1) |
| `first()` | `Slice[T]?` | O(n)* |
| `iter()` | `SplitWhereIterator[T]` | yields `Slice[T]` |
| `collect()` | `Array[Slice[T]]` | O(n), materializes all segments |

\* O(scan to first match), not O(whole collection).

No random-access subscript on `SplitView` or `SplitWhereView` — the split points aren't known until you scan. To access the i-th segment, either iterate or `.collect()` into an array.

The iterator types (`ChunksIterator`, `WindowsIterator`, etc.) still exist — they're the single-pass cursor produced by calling `.iter()` on a view. But the user-facing API returns views, not iterators.

---

## Part 5: CowBox — Reusable COW Infrastructure

Array, String, and Dictionary all independently implement the same pattern:

```
private var storage: RcBox[XStorage]

private mutating func makeUnique() {
    if self.storage.isUnique() == false {
        self.storage = RcBox(self.storage.getValue().clone())
    }
}
```

Extract it into a reusable wrapper:

```
struct CowBox[T] where T: Cloneable {
    private var inner: RcBox[T]

    init(value: T) {
        self.inner = RcBox(value)
    }

    /// Read access — no clone, no refcount check.
    func read() -> T { self.inner.getValue() }

    /// Write access — clones storage if shared, then returns the unique copy.
    mutating func write() -> T {
        if self.inner.isUnique() == false {
            self.inner = RcBox(self.inner.getValue().clone())
        }
        self.inner.getValue()
    }

    /// Shallow clone — shares storage, bumps refcount.
    func clone() -> CowBox[T] {
        CowBox(inner: self.inner.clone())
    }

    /// True when this is the only reference to the storage.
    func isUnique() -> Bool { self.inner.isUnique() }
}
```

Every COW collection becomes:

```
struct Array[T] {
    private var storage: CowBox[ArrayStorage[T]]

    var count: Int64 { self.storage.read().len }

    mutating func append(element: T) {
        var s = self.storage.write();  // COW barrier — one line
        // ... append logic
    }
}
```

Benefits:
- One implementation of COW. One place to get it right.
- One place to add optimizations later (small-buffer optimization, etc.).
- New collection types (Deque, OrderedDictionary) get COW for free.
- Shared with `std.text`: `String` uses `CowBox[StringStorage]` instead of hand-rolling the same `RcBox` + `makeUnique()` pattern.

---

## Part 6: ArrayBuilder

Write-only buffer for efficient array construction. No COW, no RcBox, no `isUnique` checks. Mirrors `StringBuilder` from the string refactor.

```
struct ArrayBuilder[T] {
    private var ptr: Pointer[T]
    private var len: Int64
    private var cap: Int64
}
```

Public API:

```
init()
init(capacity: Int64)

mutating func append(element: T)
mutating func append(contentsOf: Slice[T])
mutating func appendFrom[I](iterable: I) where I: Iterable, I.Item = T

func build() -> Array[T]     // transfer ownership, zero-copy (wrap buffer in CowBox)
mutating func clear()         // reset len to 0, keep buffer for reuse

var count: Int64
var isEmpty: Bool
```

`build()` wraps the builder's buffer in a `CowBox[ArrayStorage[T]]` and returns an owned Array without copying. The builder is left empty after `build()`. Calling `build()` on an already-empty builder returns an empty Array.

This matters for the common "build up an array in a loop, then freeze it" pattern — currently paying for COW bookkeeping on every append even though nothing is sharing the storage.

---

## Part 7: Collection-Returning Transforms

Currently, transforming an Array requires the iterator pipeline:

```
let doubled = arr.iter().map { it * 2 }.collect()
```

Three steps, an iterator allocation, and `collect()` doesn't know the output size. The `Seq` protocol extension should provide eager transforms that know the source size:

```
extend Seq {
    func map[U](transform: (T) -> U) -> Array[U] {
        var builder = ArrayBuilder[U](capacity: self.count);
        var it = self.iter();
        while let .Some(item) = it.next() {
            builder.append(transform(item))
        }
        builder.build()
    }

    func filter(matching predicate: (T) -> Bool) -> Array[T] { ... }
    func compactMap[U](transform: (T) -> U?) -> Array[U] { ... }
    func flatMap[U](transform: (T) -> Array[U]) -> Array[U] { ... }
}
```

The lazy `.iter().map().filter()` chain stays available for when you want fusion or early termination. The eager versions handle the common case ("transform this collection into another") without the ceremony.

---

## Part 8: Collectible Protocol

`collect()` only produces `Array[Item]`. The `Collectible` protocol lets iterators materialize into any container:

```
protocol Collectible {
    type Element
    static func fromIterator[I](iter: I) -> Self where I: Iterator, I.Item = Element
}
```

Conformances:

```
extend Array: Collectible {
    type Element = T
    static func fromIterator[I](iter: I) -> Array[T] where I: Iterator, I.Item = T {
        var builder = ArrayBuilder[T]();
        while let .Some(item) = iter.next() {
            builder.append(item)
        }
        builder.build()
    }
}

extend Set: Collectible {
    type Element = T
    static func fromIterator[I](iter: I) -> Set[T] where I: Iterator, I.Item = T { ... }
}

// Dictionary from (K, V) pairs
extend Dictionary: Collectible {
    type Element = (K, V)
    static func fromIterator[I](iter: I) -> Dictionary[K, V] where I: Iterator, I.Item = (K, V) { ... }
}

extend String: Collectible {
    type Element = Char
    static func fromIterator[I](iter: I) -> String where I: Iterator, I.Item = Char { ... }
}
```

Iterator gets a generic collect:

```
extend Iterator {
    func collectInto[C]() -> C where C: Collectible, C.Element = Item {
        C.fromIterator(self)
    }
}
```

Usage:

```
let s: Set = numbers.iter().filter { it > 0 }.collectInto[Set]()
let d: Dictionary = pairs.iter().collectInto[Dictionary]()
```

Whether type inference can elide the explicit type annotation depends on the solver. Even with explicit types, this is better than having no path at all.

---

## Part 9: Sort Algorithm

Array's `sort(by:)` currently uses insertion sort — O(n^2). This must be replaced with an O(n log n) algorithm.

Recommended: **introsort** (quicksort with heapsort fallback when recursion depth exceeds 2 * log2(n)). This is what C++ `std::sort` uses. It provides:

- O(n log n) worst case (heapsort fallback prevents quicksort's degenerate case)
- Excellent cache locality (in-place, no allocation)
- Low constant factor for random data (quicksort's inner loop)
- Optional: fall back to insertion sort for partitions smaller than ~16 elements

The sort should be defined on `Slice[T]` (mutable sort of a borrowed region) and Array calls it through `asSlice()` after the COW barrier. This means sorted sub-slices come for free.

---

## Part 10: New Data Structures

### Deque[T]

Ring-buffer double-ended queue. O(1) amortized push/pop from both ends.

```
struct Deque[T]: Collection, Iterable {
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

**Seq conformance:** Deque's ring buffer may wrap around, so it can't always provide a single contiguous `Slice[T]`. Options:

1. Deque conforms to `Collection` but not `Seq` — loses the shared contiguous API
2. Deque provides `asSlices() -> (Slice[T], Slice[T])` as a Deque-specific method
3. Deque linearizes on `asSlice()` — copies into a temp buffer (defeats the purpose)

Recommendation: option 2. Deque conforms to `Collection` and has a `asSlices()` method for direct buffer access. The `Seq` protocol is strictly for single-contiguous-buffer types.

### OrderedDictionary[K, V]

Insertion-order-preserving map. For JSON round-tripping, config files, deterministic output.

```
struct OrderedDictionary[K, V]: Collection where K: Equatable, K: Hash {
    // Dense array of (K, V) pairs (insertion order)
    // Hash table mapping K -> index into dense array
}
```

Same API as Dictionary plus:
- Guaranteed iteration in insertion order
- `subscript(position: Int64) -> (K, V)` — access by insertion position
- `mutating func moveToEnd(key: K)` — reorder

Separate type from Dictionary — different performance characteristics (extra indirection on lookup, but better cache locality on iteration).

### SortedDictionary[K, V] and SortedSet[T]

B-tree backed. O(log n) insert/lookup/delete. Ordered iteration by key. Range queries.

```
struct SortedDictionary[K, V]: Collection where K: Comparable {
    // B-tree backing
}

struct SortedSet[T]: Collection where T: Comparable {
    // B-tree backing (or SortedDictionary[T, Unit])
}
```

Unique capabilities:
- `func range(from: K, to: K) -> SortedDictionarySlice[K, V]` — O(log n + k)
- `func min() -> (K, V)?` — O(log n) or O(1) with cached extrema
- `func max() -> (K, V)?` — O(log n) or O(1) with cached extrema

### Heap[T]

Binary heap backed by Array. Min-heap by default (or parameterized by comparator).

```
struct Heap[T]: Collection where T: Comparable {
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

### BitSet

Compact set of non-negative integers. For flags, membership bitmaps, graph algorithms.

```
struct BitSet: Collection {
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

## Part 11: Future — InlineArray (Fixed-Size, Stack-Allocated)

Every `[1, 2, 3]` currently heap-allocates. For small, fixed-size data (coordinates, colors, small lookup tables), this is significant overhead.

```
struct InlineArray[T; N] {
    // N elements stored inline in the struct, no heap allocation
}
```

This requires const generics in the type system — a significant language feature. Without full const generics, alternatives:

1. **Compiler-supported fixed sizes** (2, 3, 4, 8, 16, 32) — covers most use cases
2. **SmallArray[T]** with a fixed inline capacity + overflow to heap — like SmallVec in Rust

This is explicitly a future item, listed here for completeness. The protocol hierarchy should be designed to accommodate it: `InlineArray[T; N]` would conform to `Seq[T]`.

---

## Migration Path

### Phase 1: Foundation (non-breaking)

1. Add `CowBox[T]` to `std.memory`
2. Add `Collection` protocol to `std.core`
3. Add `Seq[T]` protocol with `asSlice()` kernel
4. Add `extend Seq` with all read-only methods
5. Make Array conform to `Seq[T]`
6. Fix Slice's broken `isEqual` — element-wise comparison
7. Add `Iterable` conformance to Slice
8. Make Slice conform to `Seq[T]`
9. Add `Formattable` conformance to Slice (conditional on `T: Formattable`)

### Phase 2: Views and Builder (non-breaking additions)

10. Add `ChunksView[T]`, `WindowsView[T]`, `ReversedView[T]`, `SplitView[T]`
11. Add `ArrayBuilder[T]`
12. Add `Collectible` protocol and conformances
13. Add collection-returning transforms to `extend Seq` (`map`, `filter`, etc.)

### Phase 3: Index Protocol Unification

14. Add unified `SeqIndex[T]`, `SeqClampable[T]`, `SeqWrappable[T]`
15. Add conformances for `Int64`, `Range[Int64]`, `ClosedRange[Int64]`
16. Wire `Seq` subscripts through the new protocols

### Phase 4: Sort and Internal Improvements

17. Replace insertion sort with introsort
18. Refactor Array, String, Dictionary to use `CowBox[T]`
19. Merge `ArrayIterator` and `SliceIterator` into one type

### Phase 5: New Data Structures

20. Add `Deque[T]`
21. Add `OrderedDictionary[K, V]`
22. Add `Heap[T]`
23. Add `BitSet`
24. Add `SortedDictionary[K, V]` and `SortedSet[T]`

### Phase 6: Breaking Changes and Cleanup

25. Remove old `ArrayIndex[T]` / `SliceIndex[T]` / etc. protocols
26. Remove `ArrayIterator[T]` (replaced by `SliceIterator[T]`)
27. Remove `ChunksIterator[T]` and `WindowsIterator[T]` as direct Array returns (keep as view internals)
28. Move `Slice[T]` to `std.collections` (or re-export from there)
29. Update stdlib code that uses the old API
30. Update tests
