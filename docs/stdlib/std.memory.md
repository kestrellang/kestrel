# std.memory

## protocol `Allocator`

```kestrel
public protocol Allocator
```

Protocol for raw-memory allocators.

`Allocator` is the indirection collections use so they can be parameterised
over allocation strategy (e.g. `Array[T, A]`, `Buffer[T, A]`, custom
arenas). All three methods are `mutating` so stateful allocators (arenas,
pools) can update their bookkeeping; stateless wrappers around `malloc`
don't need to.

### Examples

```
var alloc = SystemAllocator();
if let .Some(p) = alloc.allocate(Layout.of[Int64]()) {
    // ... use p ...
    alloc.deallocate(p, Layout.of[Int64]())
}
```

_Defined in `lang/std/memory/allocator.ks`._

### Members

#### function `allocate`

```kestrel
mutating func allocate(Layout) -> RawPointer?
```

Returns a pointer to a fresh region matching `layout`, or `.None`
when allocation fails. Returned memory is uninitialised.

_Defined in `lang/std/memory/allocator.ks`._

#### function `deallocate`

```kestrel
mutating func deallocate(RawPointer, Layout)
```

Releases memory previously returned by `allocate` / `reallocate`.
`layout` must match the layout used to obtain the pointer.

##### Safety

`ptr` must have been produced by this allocator (or a clone of it)
for `layout`. Mismatching the layout, double-freeing, or freeing a
pointer from another allocator is undefined behavior.

_Defined in `lang/std/memory/allocator.ks`._

#### function `reallocate`

```kestrel
mutating func reallocate(RawPointer, Layout, Layout) -> RawPointer?
```

Resizes the allocation at `ptr` from `oldLayout` to `newLayout`.
On failure the original allocation is left intact and `.None` is
returned. On success the old pointer must not be reused — use the
returned pointer instead.

_Defined in `lang/std/memory/allocator.ks`._

## struct `ArraySlice`

```kestrel
public struct ArraySlice[T] { /* private fields */ }
```

Non-owning view over a contiguous run of `T` values.

`Slice` is the standard "borrow" type for arrays, buffers, and any
other contiguous storage: it stores a pointer + length and provides
safe and unchecked indexing, sub-slicing, iteration, and pattern
matching. The slice does **not** track or extend the lifetime of the
underlying storage — keeping a slice past the end of its source is a
use-after-free.

### Examples

```
let arr = [1, 2, 3, 4];
let s = arr.asSlice();
s[safe: 0]                    // .Some(1)
s[safe: 99]                   // .None
for x in s.iter() { print(x) }
```

### Memory Model

Non-owning. Drop the source (`Array`, `Buffer`, literal scope) and the
slice becomes dangling. Slices freely copy — they're just `(ptr, len)`
pairs.

_Defined in `lang/std/memory/pointer.ks`._

### Members

#### initializer `From Pointer`

```kestrel
public init(pointer: Pointer[T], count: Int64)
```

Builds a slice from an existing pointer and element count. The
caller is responsible for ensuring `count` elements live at `pointer`.

_Defined in `lang/std/memory/pointer.ks`._

#### field `count`

```kestrel
public var count: Int64 { get }
```

Element count.

_Defined in `lang/std/memory/pointer.ks`._

#### function `first`

```kestrel
public func first() -> Optional[T]
```

First element, or `.None` for an empty slice.

_Defined in `lang/std/memory/pointer.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

`true` when `count == 0`.

_Defined in `lang/std/memory/pointer.ks`._

#### function `last`

```kestrel
public func last() -> Optional[T]
```

Last element, or `.None` for an empty slice.

_Defined in `lang/std/memory/pointer.ks`._

#### field `pointer`

```kestrel
public var pointer: Pointer[T] { get }
```

Pointer to the first element. `pointer.offset(by: i)` reaches
element `i` (0-indexed).

_Defined in `lang/std/memory/pointer.ks`._

### Implements `ArrayMatchable`

#### typealias `Element`

```kestrel
type Element = T
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `matchGet`

```kestrel
public func matchGet(Int64) -> T
```

Compiler-driven element read; safe to skip the bounds check
because the matcher emits `index < matchLength()` first.

_Defined in `lang/std/memory/pointer.ks`._

#### function `matchLength`

```kestrel
public func matchLength() -> Int64
```

Element count, exposed to the pattern matcher.

_Defined in `lang/std/memory/pointer.ks`._

#### function `matchSlice`

```kestrel
public func matchSlice(Int64, Int64) -> ArraySlice[T]
```

Sub-slice for rest-pattern bindings (`..rest`). As above, the
matcher guarantees `0 <= from <= to <= matchLength()`.

_Defined in `lang/std/memory/pointer.ks`._

### Implements `Slice`

#### function `all`

```kestrel
public func all(where: (T) -> Bool) -> Bool
```

`true` when every element satisfies `predicate`. O(n).

Short-circuits on the first failure. Vacuously true for empty
collections.

##### Examples

```
[2, 4, 6].all(where: { it % 2 == 0 });  // true
[2, 3, 6].all(where: { it % 2 == 0 });  // false
```

_Defined in `lang/std/collections/slice.ks`._

#### function `any`

```kestrel
public func any(where: (T) -> Bool) -> Bool
```

`true` when at least one element satisfies `predicate`. O(n).

Short-circuits on the first match. Always `false` for empty
collections.

##### Examples

```
[1, 2, 3].any(where: { it > 2 });  // true
[1, 2, 3].any(where: { it > 5 });  // false
```

_Defined in `lang/std/collections/slice.ks`._

#### function `asPointer`

```kestrel
public func asPointer() -> Pointer[T]
```

Pointer to the first element. The pointer aliases the collection's
buffer; do not outlive the source or mutate through it.

##### Safety

Reading past `count` is undefined behavior.

_Defined in `lang/std/collections/slice.ks`._

#### function `asSlice`

```kestrel
public func asSlice() -> ArraySlice[T]
```

Returns `self` — `ArraySlice` is already the borrowed view.

_Defined in `lang/std/memory/pointer.ks`._

#### function `binarySearch`

```kestrel
public func binarySearch(T) -> Int64?
```

Binary search for `element`. Returns its index or `None`. O(log n).

When duplicates exist, which index is returned is unspecified.

##### Safety

The collection must be sorted in ascending order. Calling on
unsorted data won't crash but may produce false negatives.

##### Examples

```
[1, 2, 3, 4, 5].binarySearch(3);  // Some(2)
[1, 2, 3, 4, 5].binarySearch(6);  // None
```

_Defined in `lang/std/collections/slice.ks`._

#### function `chunks`

```kestrel
public func chunks(of: Int64) -> ChunksView[T]
```

Multi-pass lazy view over non-overlapping `size`-sized chunks.

The trailing chunk may be shorter than `size`. Multi-pass: query
`count`, index with `view.get(i)`, and iterate repeatedly without
re-creating the view.

##### Errors

Panics if `size <= 0`.

##### Examples

```
let v = [1, 2, 3, 4, 5].chunks(of: 2);
v.count;          // 3
v.get(2);          // ArraySlice[5]
for c in v { ... }
```

_Defined in `lang/std/collections/slice.ks`._

#### function `compactMap`

```kestrel
public func compactMap[U]((T) -> Optional[U]) -> Array[U]
```

Maps every element through `transform`, dropping `.None` results.
O(n).

##### Examples

```
["1", "x", "3"].compactMap { Int64.parse(it) };  // [1, 3]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `contains`

```kestrel
public func contains(T) -> Bool
```

`true` if the collection contains `element`. O(n).

Linear scan; short-circuits on the first match.

##### Examples

```
[1, 2, 3].contains(2);  // true
[1, 2, 3].contains(5);  // false
```

_Defined in `lang/std/collections/slice.ks`._

#### field `count`

```kestrel
public var count: Int64 { get }
```

Element count. O(1).

##### Examples

```
[1, 2, 3].count;  // 3
[].count;          // 0
```

_Defined in `lang/std/collections/slice.ks`._

#### function `countItems`

```kestrel
public func countItems(where: (T) -> Bool) -> Int64
```

Number of elements for which `predicate` is true. O(n).

##### Examples

```
[1, 2, 3, 4, 5].countItems(where: { it % 2 == 0 });  // 2
```

_Defined in `lang/std/collections/slice.ks`._

#### function `drop`

```kestrel
public func drop(first: Int64) -> ArraySlice[T]
```

Returns a slice with the first `count` elements skipped. O(1).

Complement of `prefix`.

##### Errors

Panics if `count > self.count`.

##### Examples

```
[1, 2, 3, 4, 5].drop(first: 2);  // ArraySlice[3, 4, 5]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `ends`

```kestrel
public func ends[__opaque_0](with: __opaque_0) -> Bool where __opaque_0: Slice[T]
```

`true` if the trailing elements match `suffix`. O(k) where k is
the suffix length. Accepts any `Slice[T]` conformer.

##### Examples

```
[1, 2, 3].ends(with: [2, 3]);  // true
[1, 2, 3].ends(with: [1, 2]);  // false
[1, 2, 3].ends(with: []);       // true (vacuous)
```

_Defined in `lang/std/collections/slice.ks`._

#### function `ensureUnique`

```kestrel
public mutating func ensureUnique()
```

No-op — `ArraySlice` is a non-owning view with no COW barrier.

_Defined in `lang/std/memory/pointer.ks`._

#### function `filter`

```kestrel
public func filter(where: (T) -> Bool) -> Array[T]
```

Returns a new array containing every element matching `predicate`.
O(n). Result size is unknown; uses geometric growth.

##### Examples

```
[1, 2, 3, 4].filter(where: { it % 2 == 0 });  // [2, 4]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `first`

```kestrel
public func first() -> T?
```

First element, or `.None` for an empty collection. O(1).

Read-only — to remove the first element from an `Array`, use
`popFirst()`.

##### Examples

```
[1, 2, 3].first();  // Some(1)
[].first();          // None
```

_Defined in `lang/std/collections/slice.ks`._

#### function `firstIndex`

```kestrel
public func firstIndex(where: (T) -> Bool) -> Int64?
```

Index of the first element matching `predicate`, or `None`. O(n).

Short-circuits on the first match. For value-based search on
`Equatable` collections, use `firstIndex(of:)`.

##### Examples

```
[1, 2, 3, 4, 5].firstIndex(where: { it > 3 });   // Some(3)
[1, 2, 3].firstIndex(where: { it > 10 });         // None
```

_Defined in `lang/std/collections/slice.ks`._

#### function `flatMap`

```kestrel
public func flatMap[U]((T) -> Array[U]) -> Array[U]
```

Maps every element through `transform` and concatenates the results
into one flat array. O(n + total_output).

##### Examples

```
[1, 2, 3].flatMap { [it, it * 10] };  // [1, 10, 2, 20, 3, 30]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `format`

```kestrel
public func format(into: mutating StringBuilder, FormatOptions)
```

Renders as `"[e1, e2, ...]"`. Empty collections render as `"[]"`.

##### Examples

```
[1, 2, 3].format();  // "[1, 2, 3]"
[].format();          // "[]"
```

_Defined in `lang/std/collections/slice.ks`._

#### field `indices`

```kestrel
public var indices: Range[Int64] { get }
```

Half-open range `0..<count`.

##### Examples

```
[10, 20, 30].indices;  // 0..<3
```

_Defined in `lang/std/collections/slice.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

`true` when `count == 0`.

##### Examples

```
[].isEmpty;   // true
[1].isEmpty;  // false
```

_Defined in `lang/std/collections/slice.ks`._

#### function `isEqual`

```kestrel
public func isEqual(to: Self) -> Bool
```

Element-wise equality. O(n).

Short-circuits on the first mismatch. Order matters.

##### Examples

```
[1, 2, 3].isEqual(to: [1, 2, 3]);  // true
[1, 2, 3].isEqual(to: [3, 2, 1]);  // false
```

_Defined in `lang/std/collections/slice.ks`._

#### function `isSorted`

```kestrel
public func isSorted() -> Bool
```

`true` if elements are in non-decreasing order. O(n).

Equal adjacent elements are allowed. Empty and single-element
collections are vacuously sorted.

##### Examples

```
[1, 2, 3].isSorted();  // true
[1, 3, 2].isSorted();  // false
[1, 1, 1].isSorted();  // true
[].isSorted();          // true
```

_Defined in `lang/std/collections/slice.ks`._

#### function `isValidIndex`

```kestrel
public func isValidIndex(Int64) -> Bool
```

`true` if `index` is in `[0, count)`.

##### Examples

```
[10, 20, 30].isValidIndex(2);   // true
[10, 20, 30].isValidIndex(3);   // false
[10, 20, 30].isValidIndex(-1);  // false
```

_Defined in `lang/std/collections/slice.ks`._

#### function `iter`

```kestrel
public func iter() -> ArraySliceIterator[T]
```

Forward iterator over the elements.

##### Examples

```
for item in [1, 2, 3] { ... }
```

_Defined in `lang/std/collections/slice.ks`._

#### function `last`

```kestrel
public func last() -> T?
```

Last element, or `.None` for an empty collection. O(1).

Read-only — to remove the last element from an `Array`, use
`pop()`.

##### Examples

```
[1, 2, 3].last();  // Some(3)
[].last();          // None
```

_Defined in `lang/std/collections/slice.ks`._

#### function `lastIndex`

```kestrel
public func lastIndex(where: (T) -> Bool) -> Int64?
```

Index of the last element matching `predicate`, or `None`. O(n).

Scans from the back; short-circuits on the first match.

##### Examples

```
[1, 2, 3, 2, 1].lastIndex(where: { it == 2 });  // Some(3)
```

_Defined in `lang/std/collections/slice.ks`._

#### function `map`

```kestrel
public func map[U]((T) -> U) -> Array[U]
```

Maps every element through `transform` into a new array. O(n).

Pre-sizes the result buffer to `self.count`, so no growth steps. For
the lazy version that fuses into a chain, use `iter().map { ... }`.

##### Examples

```
[1, 2, 3].map { it * 2 };       // [2, 4, 6]
[1, 2, 3].map { it.format() };  // ["1", "2", "3"]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `max`

```kestrel
public func max() -> T?
```

Largest element, or `None` if empty. O(n).

Ties go to the first occurrence.

##### Examples

```
[3, 1, 4].max();  // Some(4)
[].max();          // None
```

_Defined in `lang/std/collections/slice.ks`._

#### function `min`

```kestrel
public func min() -> T?
```

Smallest element, or `None` if empty. O(n).

Ties go to the first occurrence.

##### Examples

```
[3, 1, 4].min();  // Some(1)
[].min();          // None
```

_Defined in `lang/std/collections/slice.ks`._

#### function `prefix`

```kestrel
public func prefix(Int64) -> ArraySlice[T]
```

Returns a slice over the first `count` elements. O(1).

##### Errors

Panics if `count > self.count`.

##### Examples

```
[1, 2, 3, 4, 5].prefix(3);  // ArraySlice[1, 2, 3]
[1, 2].prefix(0);            // empty slice
```

_Defined in `lang/std/collections/slice.ks`._

#### function `reversed`

```kestrel
public func reversed() -> ReversedView[T]
```

Multi-pass lazy reversed view. Iterates back-to-front and
supports indexed access in O(1).

##### Examples

```
let v = [1, 2, 3].reversed();
v.first();        // Some(3)
v.toArray();       // [3, 2, 1] — eager copy
```

_Defined in `lang/std/collections/slice.ks`._

#### function `sorted`

```kestrel
public func sorted() -> Array[T]
```

Returns a new sorted array; original unchanged. O(n log n).

##### Examples

```
let arr = [3, 1, 4, 1, 5];
arr.sorted();  // [1, 1, 3, 4, 5]
// arr is still [3, 1, 4, 1, 5]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `split`

```kestrel
public func split(where: (T) -> Bool) -> ArraySplitWhereView[T]
```

Multi-pass lazy view over the segments produced by splitting at
each element matching `predicate`. Matching elements are dropped.

##### Examples

```
let v = [1, -1, 2, 3, -1, 4].split(where: { it < 0 });
for seg in v { ... }
```

_Defined in `lang/std/collections/slice.ks`._

#### function `starts`

```kestrel
public func starts[__opaque_0](with: __opaque_0) -> Bool where __opaque_0: Slice[T]
```

`true` if the leading elements match `prefix`. O(k) where k is
the prefix length. Accepts any `Slice[T]` conformer.

##### Examples

```
[1, 2, 3].starts(with: [1, 2]);     // true
[1, 2, 3].starts(with: [2, 3]);     // false
[1, 2, 3].starts(with: []);          // true (vacuous)
```

_Defined in `lang/std/collections/slice.ks`._

#### subscript `subscript`

```kestrel
public subscript[I](I) -> I.SeqOutput { get set }
```

_Defined in `lang/std/collections/slice.ks`._

#### function `suffix`

```kestrel
public func suffix(Int64) -> ArraySlice[T]
```

Returns a slice over the last `count` elements. O(1).

##### Errors

Panics if `count > self.count`.

##### Examples

```
[1, 2, 3, 4, 5].suffix(2);  // ArraySlice[4, 5]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `unique`

```kestrel
public func unique() -> Array[T]
```

Returns a new array with duplicates removed, preserving
first-occurrence order. O(n²).

For the mutating variant on `Array`, see `removeDuplicates()`.

##### Examples

```
[1, 2, 1, 3, 2, 4].unique();  // [1, 2, 3, 4]
```

_Defined in `lang/std/collections/slice.ks`._

#### function `windows`

```kestrel
public func windows(of: Int64) -> WindowsView[T]
```

Multi-pass lazy view over overlapping `size`-sized sliding
windows.

Adjacent windows overlap by `size - 1` elements. Empty when the
source has fewer than `size` elements.

##### Errors

Panics if `size <= 0`.

##### Examples

```
let v = [1, 2, 3, 4].windows(of: 2);
v.count;          // 3
for w in v { ... }
```

_Defined in `lang/std/collections/slice.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/memory/pointer.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = ArraySliceIterator[T]
```

_Defined in `lang/std/memory/pointer.ks`._

#### function `iter`

```kestrel
public func iter() -> ArraySliceIterator[T]
```

Forward iterator over the elements.

_Defined in `lang/std/memory/pointer.ks`._

### Implements `Equatable`

#### typealias `Output`

```kestrel
type Output = Bool
```

_Defined in `lang/std/core/protocols.ks`._

#### function `equal`

```kestrel
public func equal(to: Self) -> Bool
```

Bridges `Equal.equal(to:)` to `Equatable.isEqual(to:)`.

_Defined in `lang/std/core/protocols.ks`._

#### function `isEqual`

```kestrel
func isEqual(to: Self) -> Bool
```

Returns `true` iff `self` and `other` are considered equal. Should
be reflexive, symmetric, and transitive — `Hashable` requires equal
values to hash equal, so don't drift from those laws.

_Defined in `lang/std/core/protocols.ks`._

#### function `notEqual`

```kestrel
public func notEqual(to: Self) -> Bool
```

Default `!=`: delegates to `==` so there's a single source of truth.

_Defined in `lang/std/core/protocols.ks`._

## struct `ArraySliceIterator`

```kestrel
public struct ArraySliceIterator[T] { /* private fields */ }
```

Forward iterator over an `ArraySlice[T]`. Holds a moving pointer and a
remaining count; advancing reads through the pointer.

### Representation

A `Pointer[T]` cursor and an `Int64` countdown.

_Defined in `lang/std/memory/pointer.ks`._

### Members

#### initializer `From Storage`

```kestrel
public init(ptr: Pointer[T], remaining: Int64)
```

Builds an iterator from a starting pointer and remaining count.

_Defined in `lang/std/memory/pointer.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/memory/pointer.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = Self
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `all`

```kestrel
public mutating func all(where: (Item) -> Bool) -> Bool
```

True if every element satisfies `predicate`. Stops at the first
failure. True for an empty iterator (vacuous truth).

##### Examples

```
[2, 4, 6].iter().all { it % 2 == 0 };   // true
[2, 3, 4].iter().all { it % 2 == 0 };   // false (stops at 3)
[].iter().all { false };                // true (empty)
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `any`

```kestrel
public mutating func any(where: (Item) -> Bool) -> Bool
```

True if any element satisfies `predicate`. Stops at the first
match. False for an empty iterator.

##### Examples

```
[1, 2, 3, 4].iter().any { it > 3 };    // true (stops at 4)
[1, 2, 3].iter().any { it > 10 };      // false
[].iter().any { true };                // false
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `chain`

```kestrel
public func chain[Other](Other) -> ChainIterator[Self, Other] where Other: Iterator, Other.Item == Item
```

Yields all of `self`, then all of `other`. Both must produce the
same `Item` type.

##### Examples

```
[1, 2].iter().chain([3, 4].iter()).collect();   // [1, 2, 3, 4]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `collect`

```kestrel
public consuming func collect() -> Array[Item]
```

Drains the iterator into an `Array[Item]`. Eager and `O(n)`. Use
at the end of an adapter chain to materialise the result.

##### Examples

```
[1, 2, 3].iter().filter { it > 1 }.collect();   // [2, 3]
(1..5).iter().map { it * it }.collect();        // [1, 4, 9, 16]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `compactMap`

```kestrel
public func compactMap[T]() -> FilterMapIterator[Self, T] where Item == Optional[T]
```

Drops `None`s and unwraps `Some`s — the identity-transform special
case of `filterMap`. Available when the iterator already yields
optionals.

##### Examples

```
let xs: [Int64?] = [.Some(1), .None, .Some(2), .None, .Some(3)];
xs.iter().compactMap().collect();   // [1, 2, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `contains`

```kestrel
public mutating func contains(Item) -> Bool
```

True if any element equals `element`. Short-circuits.

##### Examples

```
[1, 2, 3].iter().contains(2);   // true
[1, 2, 3].iter().contains(5);   // false
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `count`

```kestrel
public consuming func count() -> Int64
```

Counts the elements by walking the whole iterator. `O(n)` — for
types that already know their length, prefer
`ExactSizeIterator.remaining`.

##### Examples

```
[1, 2, 3, 4, 5].iter().filter { it % 2 == 0 }.count();   // 2
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `cycle`

```kestrel
public func cycle() -> CycleIterator[Self]
```

Restarts iteration from the beginning whenever the inner iterator
is exhausted, producing an infinite sequence. Always combine with
`take` (or another short-circuiting consumer) — otherwise the
result is unbounded.

##### Examples

```
[1, 2, 3].iter().cycle().take(7).collect();
// [1, 2, 3, 1, 2, 3, 1]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `enumerate`

```kestrel
public func enumerate() -> EnumerateIterator[Self]
```

Pairs each element with its zero-based position.

##### Examples

```
for (i, item) in arr.iter().enumerate() {
    print("Index \{i}: \{item}")
};
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `filter`

```kestrel
public func filter(where: (Item) -> Bool) -> FilterIterator[Self]
```

Yields only elements where `predicate` returns `true`. Lazy —
elements are tested as they're pulled.

##### Examples

```
[1, 2, 3, 4, 5].iter().filter { it % 2 == 0 }.collect();   // [2, 4]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `filterMap`

```kestrel
public func filterMap[U](as: (Item) -> U?) -> FilterMapIterator[Self, U]
```

Combined map + filter — `transform` returns `Optional[U]`; `None`
values are skipped. Use over `map(...).filter(...)` when the
transform itself decides whether the element belongs.

##### Examples

```
["1", "two", "3"].iter()
    .filterMap { Int64.parse(it) }
    .collect();   // [1, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `first`

```kestrel
public mutating func first(where: (Item) -> Bool) -> Item?
```

First element matching `predicate`, or `None`. Stops at the first
match.

##### Examples

```
[1, 2, 3, 4, 5].iter().first { it > 3 };   // Some(4)
[1, 2, 3].iter().first { it > 10 };        // None
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `firstIndex`

```kestrel
public mutating func firstIndex(where: (Item) -> Bool) -> Int64?
```

Index of the first element matching `predicate`, or `None`.

##### Examples

```
["a", "b", "c"].iter().firstIndex(where: { it == "b" });   // Some(1)
[1, 2, 3].iter().firstIndex(where: { it > 10 });           // None
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `flatMap`

```kestrel
public func flatMap[U](as: (Item) -> U) -> FlatMapIterator[Self, U] where U: Iterator
```

Maps each element to an iterator and concatenates the results.
The monadic bind for iterators.

##### Examples

```
[[1, 2], [3, 4], [5]].iter()
    .flatMap { it.iter() }
    .collect();   // [1, 2, 3, 4, 5]
```

```
// Conditional expand — drop odd, double even
[1, 2, 3].iter()
    .flatMap { if it % 2 == 0 { [it, it].iter() } else { [].iter() } }
    .collect();   // [2, 2]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `flatten`

```kestrel
public func flatten() -> FlattenIterator[Self]
```

Concatenates the inner iterators into one flat stream. Each inner
iterator is fully drained before moving to the next. The
already-have-iterators counterpart of `flatMap`.

##### Examples

```
let nested = [[1, 2], [3, 4], [5]].iter().map { it.iter() };
nested.flatten().collect();   // [1, 2, 3, 4, 5]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `fold`

```kestrel
public consuming func fold[Acc](from: Acc, by: (Acc, Item) -> Acc) -> Acc
```

Left fold — start at `initial` and walk left to right, applying
`combine(acc, element)`. Returns `initial` for an empty iterator.

##### Examples

```
[1, 2, 3, 4].iter().fold(from: 0) { (acc, x) in acc + x };   // 10
[1, 2, 3].iter().fold(from: 1) { (acc, x) in acc * x };      // 6
[].iter().fold(from: 42) { (acc, x) in acc + x };            // 42
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `forEach`

```kestrel
public consuming func forEach((Item) -> ())
```

Calls `action` on every element, discarding return values. Use
`tryForEach` if you need to short-circuit on failure.

##### Examples

```
[1, 2, 3].iter().forEach { print(it) };
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `fuse`

```kestrel
public func fuse() -> FusedIterator[Self]
```

Locks `None` once seen — protects against iterators that aren't
fused (i.e. that may produce more elements after returning `None`
once). After the first `None`, this adapter returns `None`
forever.

_Defined in `lang/std/iter/iterator.ks`._

#### function `inspect`

```kestrel
public func inspect((Item) -> ()) -> InspectIterator[Self]
```

Calls `inspector` on each element as it flows through, leaving
the value otherwise untouched. Useful for logging or
instrumenting an adapter chain mid-pipeline.

##### Examples

```
[1, 2, 3].iter()
    .inspect { print("before filter: \{it}") }
    .filter { it > 1 }
    .inspect { print("after filter: \{it}") }
    .collect();
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `intersperse`

```kestrel
public func intersperse(with: Item) -> IntersperseIterator[Self]
```

Inserts `separator` between consecutive elements. Empty inputs
stay empty; single-element inputs get no separator.

##### Examples

```
[1, 2, 3].iter().intersperse(with: 0).collect();
// [1, 0, 2, 0, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `intersperseWith`

```kestrel
public func intersperseWith(with: () -> Item) -> IntersperseWithIterator[Self]
```

Like `intersperse`, but builds each separator on demand by calling
`separator()`. Use when the separator is expensive or needs to
vary by call.

##### Examples

```
var counter = 0;
[1, 2, 3].iter()
    .intersperseWith { counter += 1; counter * 10 }
    .collect();   // [1, 10, 2, 20, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `isSorted`

```kestrel
public consuming func isSorted() -> Bool
```

True if elements come out in ascending order. True for empty or
single-element iterators (vacuous). Short-circuits on the first
out-of-order pair.

##### Examples

```
[1, 2, 3, 4, 5].iter().isSorted();   // true
[1, 3, 2, 4, 5].iter().isSorted();   // false
[1, 1, 2, 2, 3].iter().isSorted();   // true (equal allowed)
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `isSortedDescending`

```kestrel
public consuming func isSortedDescending() -> Bool
```

True if elements come out in descending order. Mirror of
`isSorted`.

_Defined in `lang/std/iter/iterator.ks`._

#### function `iter`

```kestrel
func iter() -> Self
```

Returns `self`. The blanket conformance pivot — iterators *are*
iterables.

_Defined in `lang/std/iter/iterator.ks`._

#### function `last`

```kestrel
public consuming func last() -> Item?
```

Last element, or `None` if empty. Consumes the entire iterator —
`O(n)` even for sequences whose last element is cheap to address
directly.

_Defined in `lang/std/iter/iterator.ks`._

#### function `map`

```kestrel
public func map[U](as: (Item) -> U) -> MapIterator[Self, U]
```

Applies `transform` to each element. Lazy — the function only
fires when the downstream pulls a value.

##### Examples

```
[1, 2, 3].iter().map { it * 2 }.collect();         // [2, 4, 6]
["hi", "yo"].iter().map { it.count }.collect();    // [2, 2]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `max`

```kestrel
public consuming func max() -> Item?
```

Largest element, or `None` for an empty iterator. Ties go to the
first occurrence.

_Defined in `lang/std/iter/iterator.ks`._

#### function `min`

```kestrel
public consuming func min() -> Item?
```

Smallest element, or `None` for an empty iterator. Ties go to the
first occurrence.

##### Examples

```
[3, 1, 4, 1, 5].iter().min();   // Some(1)
[].iter().min();                // None
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `next`

```kestrel
public mutating func next() -> Optional[T]
```

Yields the next element, or `.None` when the count reaches zero.

_Defined in `lang/std/memory/pointer.ks`._

#### function `nth`

```kestrel
public mutating func nth(Int64) -> Item?
```

Returns the element at index `n` (zero-based), consuming
everything up to and including it. `None` if `n` is past the end.

##### Examples

```
[10, 20, 30, 40].iter().nth(2);   // Some(30)
[10, 20].iter().nth(5);           // None
[10, 20, 30].iter().nth(0);       // Some(10)
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `peekable`

```kestrel
public func peekable() -> PeekableIterator[Self]
```

Wraps `self` so you can look at the next element without
consuming it.

##### Examples

```
var it = [1, 2, 3].iter().peekable();
it.peek();   // Some(1) — no consumption
it.peek();   // Some(1) — still
it.next();   // Some(1) — now consumed
it.peek();   // Some(2)
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `product`

```kestrel
public consuming func product() -> Item
```

Product of every element. Returns `Item.one` for an empty
iterator.

##### Examples

```
[1, 2, 3, 4, 5].iter().product();   // 120
(1..=5).iter().product();           // 120  (5!)
[].iter().product();                // 1
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `reduce`

```kestrel
public consuming func reduce(by: (Item, Item) -> Item) -> Item?
```

Like `fold`, but seeds the accumulator with the first element
instead of taking an explicit `initial`. Returns `None` for an
empty iterator.

##### Examples

```
[1, 2, 3, 4].iter().reduce { (a, b) in a + b };   // Some(10)
[5].iter().reduce { (a, b) in a + b };            // Some(5)
[].iter().reduce { (a, b) in a + b };             // None
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `scan`

```kestrel
public func scan[Acc](from: Acc, by: (Acc, Item) -> Acc) -> ScanIterator[Self, Acc]
```

Like `fold`, but yields each intermediate accumulator value
instead of just the final one. Useful for prefix sums, running
products, and any "carry state along" pattern.

##### Examples

```
// Running sum
[1, 2, 3, 4].iter()
    .scan(from: 0) { (acc, x) in acc + x }
    .collect();   // [1, 3, 6, 10]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `skip`

```kestrel
public func skip(Int64) -> SkipIterator[Self]
```

Drops the first `count` elements, then yields the rest.

##### Examples

```
[1, 2, 3, 4, 5].iter().skip(2).collect();   // [3, 4, 5]
[1, 2].iter().skip(10).collect();           // []
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `skipWhile`

```kestrel
public func skipWhile(where: (Item) -> Bool) -> SkipWhileIterator[Self]
```

Drops elements while `predicate` is `true`, then yields *every*
remaining element (including ones that would also satisfy the
predicate). Mirror of `takeWhile`.

##### Examples

```
[1, 2, 3, 4, 1, 2].iter()
    .skipWhile { it < 3 }
    .collect();   // [3, 4, 1, 2]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `sorted`

```kestrel
public consuming func sorted() -> Array[Item]
```

Collects into an `Array[Item]`, sorted ascending. Eager and
`O(n log n)` — calls `Array.sort(by:)` after `collect()`.

##### Examples

```
[3, 1, 4, 1, 5].iter().sorted();                       // [1, 1, 3, 4, 5]
[3, 1, 2].iter().filter { it > 1 }.sorted();          // [2, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `stepBy`

```kestrel
public func stepBy(Int64) -> StepByIterator[Self]
```

Yields every `n`-th element, starting at the first. `n == 0` is
undefined (the adapter will spin forever).

##### Examples

```
[0, 1, 2, 3, 4, 5, 6].iter().stepBy(2).collect();   // [0, 2, 4, 6]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `sum`

```kestrel
public consuming func sum() -> Item
```

Sum of every element. Returns `Item.zero` for an empty iterator.

##### Examples

```
[1, 2, 3, 4, 5].iter().sum();    // 15
[1.5, 2.5, 3.0].iter().sum();    // 7.0
[].iter().sum();                 // 0
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `take`

```kestrel
public func take(Int64) -> TakeIterator[Self]
```

Yields at most the first `count` elements; stops early even if
more are available.

##### Examples

```
[1, 2, 3, 4, 5].iter().take(3).collect();   // [1, 2, 3]
[1, 2].iter().take(10).collect();           // [1, 2]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `takeWhile`

```kestrel
public func takeWhile(where: (Item) -> Bool) -> TakeWhileIterator[Self]
```

Yields elements until `predicate` first returns `false`, then
stops. The "first failing" element is *not* yielded.

##### Examples

```
[1, 2, 3, 4, 1, 2].iter()
    .takeWhile { it < 4 }
    .collect();   // [1, 2, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `tryFold`

```kestrel
public mutating func tryFold[Acc, E](from: Acc, by: (Acc, Item) -> Result[Acc, E]) -> Result[Acc, E]
```

Fold with early exit on `Err`. The combine returns `Result`; the
first `Err` halts iteration and is returned. If everything
succeeds, returns `Ok(final accumulator)`.

##### Examples

```
// Stop the moment a parse fails
["1", "2", "3"].iter()
    .tryFold(from: 0) { (acc, s) in
        match Int64.parse(s) {
            .Some(n) => .Ok(acc + n),
            .None    => .Err("parse error")
        }
    };   // Ok(6)

["1", "bad", "3"].iter()
    .tryFold(from: 0) { (acc, s) in
        match Int64.parse(s) {
            .Some(n) => .Ok(acc + n),
            .None    => .Err("parse error")
        }
    };   // Err("parse error")
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `tryForEach`

```kestrel
public mutating func tryForEach[E]((Item) -> Result[(), E]) -> Result[(), E]
```

`forEach` with early exit on `Err`. Mirror of `tryFold` for the
"do something with each element" shape.

##### Examples

```
files.iter().tryForEach { (path) in
    File.delete(path)   // Result[(), IoError]
};   // stops on first failure
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `unzip`

```kestrel
public consuming func unzip[A, B]() -> (Array[A], Array[B]) where Item == (A, B)
```

Splits an iterator of pairs into two parallel arrays. Inverse of
`zip`.

##### Examples

```
let pairs = [(1, "a"), (2, "b"), (3, "c")];
let (nums, strs) = pairs.iter().unzip();
// nums = [1, 2, 3], strs = ["a", "b", "c"]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `zip`

```kestrel
public func zip[Other](Other) -> ZipIterator[Self, Other] where Other: Iterator
```

Pairs elements from `self` and `other`. Stops as soon as either
side runs out.

##### Examples

```
let names = ["Alice", "Bob", "Charlie"];
let ages  = [30, 25, 35];
names.iter().zip(ages.iter()).collect();
// [("Alice", 30), ("Bob", 25), ("Charlie", 35)]
```

_Defined in `lang/std/iter/iterator.ks`._

## struct `Buffer`

```kestrel
public struct Buffer[T, A] where A: Allocator { /* private fields */ }
```

Owning, allocator-parameterised contiguous storage.

`Buffer` is the building block underneath `Array`, `String`, and any
other COW/growable collection. It owns its allocation, deallocates on
drop, and is `not Copyable` to keep ownership unique. For a non-owning
view see `Slice`; for a refcounted owning wrapper see `RcBox`.

### Examples

```
var buf = Buffer[Int64, SystemAllocator](capacity: 4, allocator: SystemAllocator());
buf.write(at: 0, value: 10);
buf.write(at: 1, value: 20);
buf.read(at: 0)              // .Some(10)
buf.resize(to: 8);           // grow in place if possible
```

### Representation

A `Pointer[T]` to the storage, an `Int64` capacity, and the allocator
instance. The buffer's contents are not initialised on construction —
reading an uninitialised slot is undefined behavior.

### Memory Model

Owning, unique. The deinit reclaims storage via the same allocator.
Marked `not Copyable` so an accidental `let b2 = b1` is rejected at
compile time; use a higher-level COW wrapper (e.g. via `RcBox`) for
shared semantics.

_Defined in `lang/std/memory/buffer.ks`._

### Members

#### initializer `With Capacity`

```kestrel
public init(Int64, A)
```

Allocates a buffer holding `capacity` elements. Storage is
uninitialised; the caller is responsible for writing valid `T`s
before reading them.

##### Errors

Panics with `"Buffer allocation failed"` if `allocator.allocate`
returns `.None`.

_Defined in `lang/std/memory/buffer.ks`._

#### function `asSlice`

```kestrel
public func asSlice() -> ArraySlice[T]
```

Returns a `ArraySlice[T]` over the entire buffer. The slice does not
extend the buffer's lifetime; callers must keep the buffer alive
for as long as they use the slice.

_Defined in `lang/std/memory/buffer.ks`._

#### field `capacity`

```kestrel
public var capacity: Int64 { get }
```

Number of element slots — not the count of *initialised* elements.

_Defined in `lang/std/memory/buffer.ks`._

#### field `pointer`

```kestrel
public var pointer: Pointer[T] { get }
```

Pointer to the first slot.

_Defined in `lang/std/memory/buffer.ks`._

#### function `read`

```kestrel
public func read(unchecked: Int64) -> T
```

Reads slot `index` without bounds checking.

##### Safety

`index` must satisfy `0 <= index < capacity`, and the slot must
already hold an initialised `T`. Out-of-range or uninitialised
reads are undefined behavior.

_Defined in `lang/std/memory/buffer.ks`._

#### function `read`

```kestrel
public func read(at: Int64) -> T?
```

Reads slot `index`, returning `.None` when out of range. As with
the unchecked form, the slot must already hold an initialised `T`.

_Defined in `lang/std/memory/buffer.ks`._

#### function `resize`

```kestrel
public mutating func resize(to: Int64)
```

Grows or shrinks the storage to hold `newCapacity` elements via
the allocator's `reallocate`. On success, existing initialised
elements are preserved up to the smaller of the two capacities;
the new pointer becomes the buffer's storage.

##### Errors

Panics with `"Buffer resize failed"` if `reallocate` returns
`.None` (the original allocation is left intact, but the panic
aborts).

_Defined in `lang/std/memory/buffer.ks`._

#### function `slice`

```kestrel
public func slice(from: Int64, to: Int64) -> ArraySlice[T]?
```

Returns a slice over `[start, end)`, or `.None` when the range
falls outside `[0, capacity]`. As with `asSlice`, the slice
borrows from the buffer.

_Defined in `lang/std/memory/buffer.ks`._

#### function `write`

```kestrel
public func write(unchecked: Int64, T)
```

Writes `value` into slot `index` without bounds checking.

##### Safety

Same precondition as `read(unchecked:)` — `0 <= index < capacity`.

_Defined in `lang/std/memory/buffer.ks`._

#### function `write`

```kestrel
public func write(at: Int64, T) -> Bool
```

Writes `value` to slot `index`. Returns `false` (and does
nothing) when out of range.

_Defined in `lang/std/memory/buffer.ks`._

## struct `CowBox`

```kestrel
public struct CowBox[T] where T: Cloneable { /* private fields */ }
```

Copy-on-write wrapper around `RcBox[T]`.

Mutable owners use `CowBox`; read-only shared owners (like
`StringSlice`) hold the inner `RcBox` directly via `shareBox()`.
The mutation protocol is `write()` → modify → `setValue()`.

### Examples

```
var box = CowBox(MyStorage());
var s = box.write();   // COW barrier — clones if shared
s.len = s.len + 1;
box.setValue(s);        // write back
```

### Representation

A single `RcBox[T]` field.

### Memory Model

Same as `RcBox`: non-atomic refcount. Cloning bumps the count;
`write` splits off a private copy when shared.

_Defined in `lang/std/memory/cowbox.ks`._

### Members

#### initializer `From Value`

```kestrel
public init(T)
```

Allocates fresh storage holding `value` with refcount 1.

_Defined in `lang/std/memory/cowbox.ks`._

#### initializer `Inner`

```kestrel
public init(inner: RcBox[T])
```

Adopts an existing `RcBox` without allocating.

_Defined in `lang/std/memory/cowbox.ks`._

#### function `isUnique`

```kestrel
public func isUnique() -> Bool
```

Returns `true` when no other clone shares this storage.

_Defined in `lang/std/memory/cowbox.ks`._

#### function `read`

```kestrel
public func read() -> T
```

Read access — clones the value so the caller gets an independent
copy. getValue() returns a raw bitwise copy from the heap; cloning
ensures owned resources (byte buffers, etc.) are properly duplicated.

_Defined in `lang/std/memory/cowbox.ks`._

#### function `setValue`

```kestrel
public func setValue(consuming T)
```

Writes `value` into the storage in place. Only valid after
a preceding `write()` call (which ensures uniqueness).
Takes `value` by consuming so the drop pass sees the caller's
local as moved (Dead) — prevents double-free of shared buffers.

_Defined in `lang/std/memory/cowbox.ks`._

#### function `shareBox`

```kestrel
public func shareBox() -> RcBox[T]
```

Returns a shared `RcBox` pointing at the same storage
(refcount bumped). Use this to hand read-only access to
types like `StringSlice`.

_Defined in `lang/std/memory/cowbox.ks`._

#### function `write`

```kestrel
public mutating func write() -> T
```

Write access — clones storage if shared, then returns the
(now unique) value. Caller modifies and calls `setValue`.

_Defined in `lang/std/memory/cowbox.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> CowBox[T]
```

Shares storage with the returned clone (refcount bump).

_Defined in `lang/std/memory/cowbox.ks`._

## typealias `GlobalAllocator`

```kestrel
public type GlobalAllocator = SystemAllocator
```

Project-wide default allocator, aliased to `SystemAllocator`. Override
at the project level if a global custom allocator is needed.

_Defined in `lang/std/memory/allocator.ks`._

## struct `Layout`

```kestrel
public struct Layout { /* private fields */ }
```

Size + alignment pair describing the memory footprint of a type.

Allocators take a `Layout` rather than a raw byte count so they can
honour alignment requirements (SIMD types, page-aligned buffers, etc.).
The static `of[T]` and `array[T]` factories cover the common cases;
`merge` and `padToAlign` exist for hand-rolled struct layouts.

### Examples

```
let l = Layout.of[Int64]();           // size 8, alignment 8
let buf = Layout.array[UInt8](1024);  // size 1024, alignment 1
allocator.allocate(l)
```

### Representation

Two `Int64`s — `size` and `alignment`. No invariants enforced at
construction; misaligned layouts are caught (or undefined) at the
allocator level.

_Defined in `lang/std/memory/layout.ks`._

### Members

#### initializer `From Fields`

```kestrel
public init(size: Int64, alignment: Int64)
```

Builds a layout from explicit `size` and `alignment`. Caller is
responsible for keeping `alignment` a power of two.

_Defined in `lang/std/memory/layout.ks`._

#### field `alignment`

```kestrel
public var alignment: Int64
```

Required alignment in bytes — always a power of two for layouts
produced by `of`/`array`.

_Defined in `lang/std/memory/layout.ks`._

#### function `array`

```kestrel
public static func array[T](Int64) -> Layout
```

Layout for `count` contiguous `T` values. Inherits the element's
alignment; size is `sizeof[T] * count` with no inter-element padding
(T is assumed already padded to its own alignment).

_Defined in `lang/std/memory/layout.ks`._

#### function `merge`

```kestrel
public func merge(with: Layout) -> (Layout, Int64)
```

Concatenates `other` after `self`, mimicking how a C struct lays
out its second field. Returns the combined layout and the byte
offset where `other`'s storage starts (handy for building field
access tables by hand).

_Defined in `lang/std/memory/layout.ks`._

#### function `of`

```kestrel
public static func of[T]() -> Layout
```

Layout for a single value of `T` — uses the compiler-known
`sizeof` and `alignof` for the type.

_Defined in `lang/std/memory/layout.ks`._

#### function `padToAlign`

```kestrel
public func padToAlign() -> Layout
```

Rounds `size` up to the next multiple of `alignment`. Use when
emitting a value into a packed array — without padding, element
`i+1` would land at the wrong offset.

_Defined in `lang/std/memory/layout.ks`._

#### field `size`

```kestrel
public var size: Int64
```

Footprint in bytes.

_Defined in `lang/std/memory/layout.ks`._

### Implements `Equatable`

#### typealias `Output`

```kestrel
type Output = Bool
```

_Defined in `lang/std/core/protocols.ks`._

#### function `equal`

```kestrel
public func equal(to: Self) -> Bool
```

Bridges `Equal.equal(to:)` to `Equatable.isEqual(to:)`.

_Defined in `lang/std/core/protocols.ks`._

#### function `isEqual`

```kestrel
public func isEqual(to: Layout) -> Bool
```

Equal when both fields match.

_Defined in `lang/std/memory/layout.ks`._

#### function `notEqual`

```kestrel
public func notEqual(to: Self) -> Bool
```

Default `!=`: delegates to `==` so there's a single source of truth.

_Defined in `lang/std/core/protocols.ks`._

## struct `LiteralSlice`

```kestrel
public struct LiteralSlice[T] { /* private fields */ }
```

Read-only view over the compiler-emitted backing buffer for an array
literal.

User code rarely names this type directly: it appears in
`ExpressibleByArrayLiteral.init(arrayLiteral:)` and friends so that
types accepting `[a, b, c]` literals can iterate the elements without
touching raw pointers. The slice does **not** own the storage — the
compiler keeps the literal alive for the duration of the call.

### Examples

```
// Conforming to ExpressibleByArrayLiteral
public struct MyVec[T]: ExpressibleByArrayLiteral {
    type Element = T
    public init(arrayLiteral lit: LiteralSlice[T]) {
        var v = MyVec();
        for x in lit { v.push(x) }
        self = v
    }
}
```

### Memory Model

Non-owning. The backing storage is compiler-managed and lives for the
scope of the literal expression. Capturing a `LiteralSlice` past that
scope is a use-after-free.

_Defined in `lang/std/memory/literal_slice.ks`._

### Members

#### subscript `Checked Index`

```kestrel
public subscript(checked: Int64) -> T? { get }
```

Reads element `index`, returning `.None` on out-of-bounds.

_Defined in `lang/std/memory/literal_slice.ks`._

#### initializer `From Storage`

```kestrel
public init(pointer: lang.ptr[T], count: lang.i64)
```

Builds the slice from the raw pointer and count the compiler emits.

_Defined in `lang/std/memory/literal_slice.ks`._

#### subscript `Indexed`

```kestrel
public subscript(Int64) -> T { get }
```

Reads element `index`, panicking on out-of-bounds.

The default subscript: trades a single comparison for a guaranteed
trap on bad input. Use `(unchecked:)` inside compiler-emitted init
paths where the index is statically known in range, or
`(checked:)` to handle out-of-range without a panic.

##### Errors

Panics with `"LiteralSlice index out of bounds"` if `index < 0`
or `index >= count`.

_Defined in `lang/std/memory/literal_slice.ks`._

#### subscript `Unchecked Index`

```kestrel
public subscript(unchecked: Int64) -> T { get }
```

Reads element `index` without bounds checking.

##### Safety

Undefined behavior if `index < 0` or `index >= count`. Compiler-
emitted init paths that use this guarantee the index is in range;
do not expose this subscript to user input without checking
`count` first.

_Defined in `lang/std/memory/literal_slice.ks`._

#### field `count`

```kestrel
public var count: Int64 { get }
```

Number of elements in the literal.

_Defined in `lang/std/memory/literal_slice.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

`true` for `[]`.

_Defined in `lang/std/memory/literal_slice.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/memory/literal_slice.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = LiteralSliceIterator[T]
```

_Defined in `lang/std/memory/literal_slice.ks`._

#### function `iter`

```kestrel
public func iter() -> LiteralSliceIterator[T]
```

Iterator over the elements in source order.

_Defined in `lang/std/memory/literal_slice.ks`._

## struct `LiteralSliceIterator`

```kestrel
public struct LiteralSliceIterator[T] { /* private fields */ }
```

Iterator yielded by `LiteralSlice.iter()`. Walks the backing buffer
element-by-element, advancing a typed pointer.

### Representation

A `Pointer[T]` plus a remaining count. No `Slice` indirection — the
iterator is what `LiteralSlice` hands out instead of exposing its
pointer directly.

_Defined in `lang/std/memory/literal_slice.ks`._

### Members

#### initializer `From Storage`

```kestrel
public init(ptr: Pointer[T], remaining: Int64)
```

Builds an iterator from a typed pointer and element count.
Not normally called by user code.

_Defined in `lang/std/memory/literal_slice.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/memory/literal_slice.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = Self
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `all`

```kestrel
public mutating func all(where: (Item) -> Bool) -> Bool
```

True if every element satisfies `predicate`. Stops at the first
failure. True for an empty iterator (vacuous truth).

##### Examples

```
[2, 4, 6].iter().all { it % 2 == 0 };   // true
[2, 3, 4].iter().all { it % 2 == 0 };   // false (stops at 3)
[].iter().all { false };                // true (empty)
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `any`

```kestrel
public mutating func any(where: (Item) -> Bool) -> Bool
```

True if any element satisfies `predicate`. Stops at the first
match. False for an empty iterator.

##### Examples

```
[1, 2, 3, 4].iter().any { it > 3 };    // true (stops at 4)
[1, 2, 3].iter().any { it > 10 };      // false
[].iter().any { true };                // false
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `chain`

```kestrel
public func chain[Other](Other) -> ChainIterator[Self, Other] where Other: Iterator, Other.Item == Item
```

Yields all of `self`, then all of `other`. Both must produce the
same `Item` type.

##### Examples

```
[1, 2].iter().chain([3, 4].iter()).collect();   // [1, 2, 3, 4]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `collect`

```kestrel
public consuming func collect() -> Array[Item]
```

Drains the iterator into an `Array[Item]`. Eager and `O(n)`. Use
at the end of an adapter chain to materialise the result.

##### Examples

```
[1, 2, 3].iter().filter { it > 1 }.collect();   // [2, 3]
(1..5).iter().map { it * it }.collect();        // [1, 4, 9, 16]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `compactMap`

```kestrel
public func compactMap[T]() -> FilterMapIterator[Self, T] where Item == Optional[T]
```

Drops `None`s and unwraps `Some`s — the identity-transform special
case of `filterMap`. Available when the iterator already yields
optionals.

##### Examples

```
let xs: [Int64?] = [.Some(1), .None, .Some(2), .None, .Some(3)];
xs.iter().compactMap().collect();   // [1, 2, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `contains`

```kestrel
public mutating func contains(Item) -> Bool
```

True if any element equals `element`. Short-circuits.

##### Examples

```
[1, 2, 3].iter().contains(2);   // true
[1, 2, 3].iter().contains(5);   // false
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `count`

```kestrel
public consuming func count() -> Int64
```

Counts the elements by walking the whole iterator. `O(n)` — for
types that already know their length, prefer
`ExactSizeIterator.remaining`.

##### Examples

```
[1, 2, 3, 4, 5].iter().filter { it % 2 == 0 }.count();   // 2
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `cycle`

```kestrel
public func cycle() -> CycleIterator[Self]
```

Restarts iteration from the beginning whenever the inner iterator
is exhausted, producing an infinite sequence. Always combine with
`take` (or another short-circuiting consumer) — otherwise the
result is unbounded.

##### Examples

```
[1, 2, 3].iter().cycle().take(7).collect();
// [1, 2, 3, 1, 2, 3, 1]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `enumerate`

```kestrel
public func enumerate() -> EnumerateIterator[Self]
```

Pairs each element with its zero-based position.

##### Examples

```
for (i, item) in arr.iter().enumerate() {
    print("Index \{i}: \{item}")
};
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `filter`

```kestrel
public func filter(where: (Item) -> Bool) -> FilterIterator[Self]
```

Yields only elements where `predicate` returns `true`. Lazy —
elements are tested as they're pulled.

##### Examples

```
[1, 2, 3, 4, 5].iter().filter { it % 2 == 0 }.collect();   // [2, 4]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `filterMap`

```kestrel
public func filterMap[U](as: (Item) -> U?) -> FilterMapIterator[Self, U]
```

Combined map + filter — `transform` returns `Optional[U]`; `None`
values are skipped. Use over `map(...).filter(...)` when the
transform itself decides whether the element belongs.

##### Examples

```
["1", "two", "3"].iter()
    .filterMap { Int64.parse(it) }
    .collect();   // [1, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `first`

```kestrel
public mutating func first(where: (Item) -> Bool) -> Item?
```

First element matching `predicate`, or `None`. Stops at the first
match.

##### Examples

```
[1, 2, 3, 4, 5].iter().first { it > 3 };   // Some(4)
[1, 2, 3].iter().first { it > 10 };        // None
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `firstIndex`

```kestrel
public mutating func firstIndex(where: (Item) -> Bool) -> Int64?
```

Index of the first element matching `predicate`, or `None`.

##### Examples

```
["a", "b", "c"].iter().firstIndex(where: { it == "b" });   // Some(1)
[1, 2, 3].iter().firstIndex(where: { it > 10 });           // None
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `flatMap`

```kestrel
public func flatMap[U](as: (Item) -> U) -> FlatMapIterator[Self, U] where U: Iterator
```

Maps each element to an iterator and concatenates the results.
The monadic bind for iterators.

##### Examples

```
[[1, 2], [3, 4], [5]].iter()
    .flatMap { it.iter() }
    .collect();   // [1, 2, 3, 4, 5]
```

```
// Conditional expand — drop odd, double even
[1, 2, 3].iter()
    .flatMap { if it % 2 == 0 { [it, it].iter() } else { [].iter() } }
    .collect();   // [2, 2]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `flatten`

```kestrel
public func flatten() -> FlattenIterator[Self]
```

Concatenates the inner iterators into one flat stream. Each inner
iterator is fully drained before moving to the next. The
already-have-iterators counterpart of `flatMap`.

##### Examples

```
let nested = [[1, 2], [3, 4], [5]].iter().map { it.iter() };
nested.flatten().collect();   // [1, 2, 3, 4, 5]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `fold`

```kestrel
public consuming func fold[Acc](from: Acc, by: (Acc, Item) -> Acc) -> Acc
```

Left fold — start at `initial` and walk left to right, applying
`combine(acc, element)`. Returns `initial` for an empty iterator.

##### Examples

```
[1, 2, 3, 4].iter().fold(from: 0) { (acc, x) in acc + x };   // 10
[1, 2, 3].iter().fold(from: 1) { (acc, x) in acc * x };      // 6
[].iter().fold(from: 42) { (acc, x) in acc + x };            // 42
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `forEach`

```kestrel
public consuming func forEach((Item) -> ())
```

Calls `action` on every element, discarding return values. Use
`tryForEach` if you need to short-circuit on failure.

##### Examples

```
[1, 2, 3].iter().forEach { print(it) };
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `fuse`

```kestrel
public func fuse() -> FusedIterator[Self]
```

Locks `None` once seen — protects against iterators that aren't
fused (i.e. that may produce more elements after returning `None`
once). After the first `None`, this adapter returns `None`
forever.

_Defined in `lang/std/iter/iterator.ks`._

#### function `inspect`

```kestrel
public func inspect((Item) -> ()) -> InspectIterator[Self]
```

Calls `inspector` on each element as it flows through, leaving
the value otherwise untouched. Useful for logging or
instrumenting an adapter chain mid-pipeline.

##### Examples

```
[1, 2, 3].iter()
    .inspect { print("before filter: \{it}") }
    .filter { it > 1 }
    .inspect { print("after filter: \{it}") }
    .collect();
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `intersperse`

```kestrel
public func intersperse(with: Item) -> IntersperseIterator[Self]
```

Inserts `separator` between consecutive elements. Empty inputs
stay empty; single-element inputs get no separator.

##### Examples

```
[1, 2, 3].iter().intersperse(with: 0).collect();
// [1, 0, 2, 0, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `intersperseWith`

```kestrel
public func intersperseWith(with: () -> Item) -> IntersperseWithIterator[Self]
```

Like `intersperse`, but builds each separator on demand by calling
`separator()`. Use when the separator is expensive or needs to
vary by call.

##### Examples

```
var counter = 0;
[1, 2, 3].iter()
    .intersperseWith { counter += 1; counter * 10 }
    .collect();   // [1, 10, 2, 20, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `isSorted`

```kestrel
public consuming func isSorted() -> Bool
```

True if elements come out in ascending order. True for empty or
single-element iterators (vacuous). Short-circuits on the first
out-of-order pair.

##### Examples

```
[1, 2, 3, 4, 5].iter().isSorted();   // true
[1, 3, 2, 4, 5].iter().isSorted();   // false
[1, 1, 2, 2, 3].iter().isSorted();   // true (equal allowed)
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `isSortedDescending`

```kestrel
public consuming func isSortedDescending() -> Bool
```

True if elements come out in descending order. Mirror of
`isSorted`.

_Defined in `lang/std/iter/iterator.ks`._

#### function `iter`

```kestrel
func iter() -> Self
```

Returns `self`. The blanket conformance pivot — iterators *are*
iterables.

_Defined in `lang/std/iter/iterator.ks`._

#### function `last`

```kestrel
public consuming func last() -> Item?
```

Last element, or `None` if empty. Consumes the entire iterator —
`O(n)` even for sequences whose last element is cheap to address
directly.

_Defined in `lang/std/iter/iterator.ks`._

#### function `map`

```kestrel
public func map[U](as: (Item) -> U) -> MapIterator[Self, U]
```

Applies `transform` to each element. Lazy — the function only
fires when the downstream pulls a value.

##### Examples

```
[1, 2, 3].iter().map { it * 2 }.collect();         // [2, 4, 6]
["hi", "yo"].iter().map { it.count }.collect();    // [2, 2]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `max`

```kestrel
public consuming func max() -> Item?
```

Largest element, or `None` for an empty iterator. Ties go to the
first occurrence.

_Defined in `lang/std/iter/iterator.ks`._

#### function `min`

```kestrel
public consuming func min() -> Item?
```

Smallest element, or `None` for an empty iterator. Ties go to the
first occurrence.

##### Examples

```
[3, 1, 4, 1, 5].iter().min();   // Some(1)
[].iter().min();                // None
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `next`

```kestrel
public mutating func next() -> T?
```

Yields the next element, or `.None` once the buffer is exhausted.

_Defined in `lang/std/memory/literal_slice.ks`._

#### function `nth`

```kestrel
public mutating func nth(Int64) -> Item?
```

Returns the element at index `n` (zero-based), consuming
everything up to and including it. `None` if `n` is past the end.

##### Examples

```
[10, 20, 30, 40].iter().nth(2);   // Some(30)
[10, 20].iter().nth(5);           // None
[10, 20, 30].iter().nth(0);       // Some(10)
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `peekable`

```kestrel
public func peekable() -> PeekableIterator[Self]
```

Wraps `self` so you can look at the next element without
consuming it.

##### Examples

```
var it = [1, 2, 3].iter().peekable();
it.peek();   // Some(1) — no consumption
it.peek();   // Some(1) — still
it.next();   // Some(1) — now consumed
it.peek();   // Some(2)
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `product`

```kestrel
public consuming func product() -> Item
```

Product of every element. Returns `Item.one` for an empty
iterator.

##### Examples

```
[1, 2, 3, 4, 5].iter().product();   // 120
(1..=5).iter().product();           // 120  (5!)
[].iter().product();                // 1
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `reduce`

```kestrel
public consuming func reduce(by: (Item, Item) -> Item) -> Item?
```

Like `fold`, but seeds the accumulator with the first element
instead of taking an explicit `initial`. Returns `None` for an
empty iterator.

##### Examples

```
[1, 2, 3, 4].iter().reduce { (a, b) in a + b };   // Some(10)
[5].iter().reduce { (a, b) in a + b };            // Some(5)
[].iter().reduce { (a, b) in a + b };             // None
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `scan`

```kestrel
public func scan[Acc](from: Acc, by: (Acc, Item) -> Acc) -> ScanIterator[Self, Acc]
```

Like `fold`, but yields each intermediate accumulator value
instead of just the final one. Useful for prefix sums, running
products, and any "carry state along" pattern.

##### Examples

```
// Running sum
[1, 2, 3, 4].iter()
    .scan(from: 0) { (acc, x) in acc + x }
    .collect();   // [1, 3, 6, 10]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `skip`

```kestrel
public func skip(Int64) -> SkipIterator[Self]
```

Drops the first `count` elements, then yields the rest.

##### Examples

```
[1, 2, 3, 4, 5].iter().skip(2).collect();   // [3, 4, 5]
[1, 2].iter().skip(10).collect();           // []
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `skipWhile`

```kestrel
public func skipWhile(where: (Item) -> Bool) -> SkipWhileIterator[Self]
```

Drops elements while `predicate` is `true`, then yields *every*
remaining element (including ones that would also satisfy the
predicate). Mirror of `takeWhile`.

##### Examples

```
[1, 2, 3, 4, 1, 2].iter()
    .skipWhile { it < 3 }
    .collect();   // [3, 4, 1, 2]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `sorted`

```kestrel
public consuming func sorted() -> Array[Item]
```

Collects into an `Array[Item]`, sorted ascending. Eager and
`O(n log n)` — calls `Array.sort(by:)` after `collect()`.

##### Examples

```
[3, 1, 4, 1, 5].iter().sorted();                       // [1, 1, 3, 4, 5]
[3, 1, 2].iter().filter { it > 1 }.sorted();          // [2, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `stepBy`

```kestrel
public func stepBy(Int64) -> StepByIterator[Self]
```

Yields every `n`-th element, starting at the first. `n == 0` is
undefined (the adapter will spin forever).

##### Examples

```
[0, 1, 2, 3, 4, 5, 6].iter().stepBy(2).collect();   // [0, 2, 4, 6]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `sum`

```kestrel
public consuming func sum() -> Item
```

Sum of every element. Returns `Item.zero` for an empty iterator.

##### Examples

```
[1, 2, 3, 4, 5].iter().sum();    // 15
[1.5, 2.5, 3.0].iter().sum();    // 7.0
[].iter().sum();                 // 0
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `take`

```kestrel
public func take(Int64) -> TakeIterator[Self]
```

Yields at most the first `count` elements; stops early even if
more are available.

##### Examples

```
[1, 2, 3, 4, 5].iter().take(3).collect();   // [1, 2, 3]
[1, 2].iter().take(10).collect();           // [1, 2]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `takeWhile`

```kestrel
public func takeWhile(where: (Item) -> Bool) -> TakeWhileIterator[Self]
```

Yields elements until `predicate` first returns `false`, then
stops. The "first failing" element is *not* yielded.

##### Examples

```
[1, 2, 3, 4, 1, 2].iter()
    .takeWhile { it < 4 }
    .collect();   // [1, 2, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `tryFold`

```kestrel
public mutating func tryFold[Acc, E](from: Acc, by: (Acc, Item) -> Result[Acc, E]) -> Result[Acc, E]
```

Fold with early exit on `Err`. The combine returns `Result`; the
first `Err` halts iteration and is returned. If everything
succeeds, returns `Ok(final accumulator)`.

##### Examples

```
// Stop the moment a parse fails
["1", "2", "3"].iter()
    .tryFold(from: 0) { (acc, s) in
        match Int64.parse(s) {
            .Some(n) => .Ok(acc + n),
            .None    => .Err("parse error")
        }
    };   // Ok(6)

["1", "bad", "3"].iter()
    .tryFold(from: 0) { (acc, s) in
        match Int64.parse(s) {
            .Some(n) => .Ok(acc + n),
            .None    => .Err("parse error")
        }
    };   // Err("parse error")
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `tryForEach`

```kestrel
public mutating func tryForEach[E]((Item) -> Result[(), E]) -> Result[(), E]
```

`forEach` with early exit on `Err`. Mirror of `tryFold` for the
"do something with each element" shape.

##### Examples

```
files.iter().tryForEach { (path) in
    File.delete(path)   // Result[(), IoError]
};   // stops on first failure
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `unzip`

```kestrel
public consuming func unzip[A, B]() -> (Array[A], Array[B]) where Item == (A, B)
```

Splits an iterator of pairs into two parallel arrays. Inverse of
`zip`.

##### Examples

```
let pairs = [(1, "a"), (2, "b"), (3, "c")];
let (nums, strs) = pairs.iter().unzip();
// nums = [1, 2, 3], strs = ["a", "b", "c"]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `zip`

```kestrel
public func zip[Other](Other) -> ZipIterator[Self, Other] where Other: Iterator
```

Pairs elements from `self` and `other`. Stops as soon as either
side runs out.

##### Examples

```
let names = ["Alice", "Bob", "Charlie"];
let ages  = [30, 25, 35];
names.iter().zip(ages.iter()).collect();
// [("Alice", 30), ("Bob", 25), ("Charlie", 35)]
```

_Defined in `lang/std/iter/iterator.ks`._

## struct `ManuallyDrop`

```kestrel
public struct ManuallyDrop[T] { /* private fields */ }
```

_Defined in `lang/std/memory/manually_drop.ks`._

### Members

#### initializer `init`

```kestrel
public init(T)
```

_Defined in `lang/std/memory/manually_drop.ks`._

#### field `value`

```kestrel
public var value: T { get }
```

_Defined in `lang/std/memory/manually_drop.ks`._

## struct `Pointer`

```kestrel
public struct Pointer[T] { /* private fields */ }
```

Typed pointer to a single value of `T`.

Element-typed counterpart to `RawPointer`: `offset(by:)` strides in
units of `sizeof[T]`, and `pointee` reads/writes through the address.
`Pointer[T]` is FFI-safe when `T` is.

### Examples

```
var x = 42;
let p = Pointer(to: x);
p.read()                       // 42
p.write(100)                   // x is now 100
p.pointee = 7                  // x is now 7
```

### Representation

One `lang.ptr[T]`. The wrapping struct is purely a typing convenience —
it lowers to a bare machine pointer.

### Memory Model

Non-owning. The pointee's lifetime is the caller's responsibility; the
pointer does not increment any refcount, register with any GC, or
trigger a deinit.

_Defined in `lang/std/memory/pointer.ks`._

### Members

#### initializer `From Raw`

```kestrel
public init(raw: lang.ptr[T])
```

Wraps an existing primitive pointer.

_Defined in `lang/std/memory/pointer.ks`._

#### initializer `To Value`

```kestrel
public init(to: T)
```

Takes the address of `value`. Equivalent to `&value` in C — the
caller must ensure `value` outlives any use of the resulting
pointer.

_Defined in `lang/std/memory/pointer.ks`._

#### field `address`

```kestrel
public var address: UInt64 { get }
```

Numeric address — same value as `asRaw().address`.

_Defined in `lang/std/memory/pointer.ks`._

#### function `asRaw`

```kestrel
public func asRaw() -> RawPointer
```

Drops the type tag, returning a `RawPointer` to the same address.

_Defined in `lang/std/memory/pointer.ks`._

#### function `cast`

```kestrel
public func cast[U]() -> Pointer[U]
```

Reinterprets the address as a `Pointer[U]`.

##### Safety

Same caveats as `RawPointer.cast` — the storage must be valid for
`U` (size, alignment, contents) at the moment of the read/write.

_Defined in `lang/std/memory/pointer.ks`._

#### function `dropInPlace`

```kestrel
public func dropInPlace()
```

Runs T's destructor at this address without copying the value to stack.
The pointer remains valid but the pointee is left in a destroyed state.

_Defined in `lang/std/memory/pointer.ks`._

#### field `isNull`

```kestrel
public var isNull: Bool { get }
```

Convenience for `address == 0`.

_Defined in `lang/std/memory/pointer.ks`._

#### function `nullPointer`

```kestrel
public static func nullPointer() -> Pointer[T]
```

Returns a typed null pointer.

_Defined in `lang/std/memory/pointer.ks`._

#### function `offset`

```kestrel
public func offset(by: Int64) -> Pointer[T]
```

Strides the pointer by `n` *elements* (multiplied by `sizeof[T]`).
Compare with `RawPointer.offset`, which strides by raw bytes.

_Defined in `lang/std/memory/pointer.ks`._

#### field `pointee`

```kestrel
public var pointee: T { get set }
```

Live view of the value at the address. `get` reads through the
pointer; `set` writes. Both are unchecked — see `# Safety`.

##### Safety

The pointer must be non-null and the storage must hold a valid
initialised `T`. Reading past the end of an allocation, after
the pointee has been freed, or through a dangling pointer is
undefined behavior.

_Defined in `lang/std/memory/pointer.ks`._

#### field `raw`

```kestrel
public var raw: lang.ptr[T] { get }
```

The wrapped primitive pointer.

_Defined in `lang/std/memory/pointer.ks`._

#### function `read`

```kestrel
public func read() -> T
```

Reads `T` from the address. Same safety preconditions as `pointee.get`.

_Defined in `lang/std/memory/pointer.ks`._

#### function `write`

```kestrel
public func write(consuming T)
```

Writes `value` through the pointer. Same safety preconditions as
`pointee.set`.

_Defined in `lang/std/memory/pointer.ks`._

### Implements `Equatable`

#### typealias `Output`

```kestrel
type Output = Bool
```

_Defined in `lang/std/core/protocols.ks`._

#### function `equal`

```kestrel
public func equal(to: Self) -> Bool
```

Bridges `Equal.equal(to:)` to `Equatable.isEqual(to:)`.

_Defined in `lang/std/core/protocols.ks`._

#### function `isEqual`

```kestrel
public func isEqual(to: Pointer[T]) -> Bool
```

Address-based equality.

_Defined in `lang/std/memory/pointer.ks`._

#### function `notEqual`

```kestrel
public func notEqual(to: Self) -> Bool
```

Default `!=`: delegates to `==` so there's a single source of truth.

_Defined in `lang/std/core/protocols.ks`._

### Implements `Hashable`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Hashes the underlying address.

Heap allocations cluster on alignment boundaries, so the raw
address has predictable low bits. We run the address through
Murmur3's `fmix64` finalizer (two rounds of `xor-shift /
multiply`) before hashing so every input bit avalanches across
the 64-bit output. Without this, pointer-keyed maps see
collision clustering driven by the allocator's stride.

_Defined in `lang/std/memory/pointer.ks`._

## struct `RawPointer`

```kestrel
public struct RawPointer { /* private fields */ }
```

Untyped pointer to raw memory — `void*` in C terms.

Used at FFI boundaries and as an intermediate when casting between
typed pointers. `RawPointer` deliberately exposes no read/write methods
of its own; cast to `Pointer[T]` first via `cast[T]()`. Equality and
hashing are address-based.

### Examples

```
let p = RawPointer.nullPointer();
p.isNull                                // true
let typed: Pointer[Int64] = p.cast[Int64]()
```

### Representation

One `lang.ptr[lang.i8]`. FFI-safe — passes as a single machine pointer.

_Defined in `lang/std/memory/pointer.ks`._

### Members

#### initializer `From Address`

```kestrel
public init(address: UInt64)
```

Reconstructs a pointer from a numeric address. Useful for
platform-specific encodings (handles, MMIO addresses); incorrect
addresses produce a pointer that dereferences to undefined memory.

_Defined in `lang/std/memory/pointer.ks`._

#### initializer `From Raw`

```kestrel
public init(raw: lang.ptr[lang.i8])
```

Wraps an existing primitive pointer.

_Defined in `lang/std/memory/pointer.ks`._

#### field `address`

```kestrel
public var address: UInt64 { get }
```

Numeric address of the pointee. Round-trips through
`RawPointer(address:)`.

_Defined in `lang/std/memory/pointer.ks`._

#### function `cast`

```kestrel
public func cast[T]() -> Pointer[T]
```

Reinterprets the address as a `Pointer[T]`.

##### Safety

The caller must ensure the address holds a valid `T` (correct size,
alignment, and initialised contents) before reading through the
returned pointer.

_Defined in `lang/std/memory/pointer.ks`._

#### field `isNull`

```kestrel
public var isNull: Bool { get }
```

Convenience for `address == 0`.

_Defined in `lang/std/memory/pointer.ks`._

#### function `nullPointer`

```kestrel
public static func nullPointer() -> RawPointer
```

Returns the canonical null pointer.

_Defined in `lang/std/memory/pointer.ks`._

#### function `offset`

```kestrel
public func offset(by: Int64) -> RawPointer
```

Adds `bytes` to the address (no element-size scaling — this is
raw byte arithmetic). Use `Pointer[T].offset` for element-typed
strides.

_Defined in `lang/std/memory/pointer.ks`._

#### field `raw`

```kestrel
public var raw: lang.ptr[lang.i8]
```

The wrapped primitive `i8*`.

_Defined in `lang/std/memory/pointer.ks`._

### Implements `Equatable`

#### typealias `Output`

```kestrel
type Output = Bool
```

_Defined in `lang/std/core/protocols.ks`._

#### function `equal`

```kestrel
public func equal(to: Self) -> Bool
```

Bridges `Equal.equal(to:)` to `Equatable.isEqual(to:)`.

_Defined in `lang/std/core/protocols.ks`._

#### function `isEqual`

```kestrel
public func isEqual(to: RawPointer) -> Bool
```

Address-based equality. Two `RawPointer`s pointing into different
allocations are equal iff their addresses coincide.

_Defined in `lang/std/memory/pointer.ks`._

#### function `notEqual`

```kestrel
public func notEqual(to: Self) -> Bool
```

Default `!=`: delegates to `==` so there's a single source of truth.

_Defined in `lang/std/core/protocols.ks`._

### Implements `Hashable`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Hashes the underlying address.

Heap allocations cluster on alignment boundaries, so the raw
address has predictable low bits. We run the address through
Murmur3's `fmix64` finalizer (two rounds of `xor-shift /
multiply`) before hashing so every input bit avalanches across
the 64-bit output. Without this, pointer-keyed maps see
collision clustering driven by the allocator's stride.

_Defined in `lang/std/memory/pointer.ks`._

## struct `RcBox`

```kestrel
public struct RcBox[T] { /* private fields */ }
```

Heap allocation with a strong-reference count, used as the underlying
storage for the stdlib's copy-on-write types.

`String`, `Array`, and `Dictionary` all wrap an `RcBox` so that a
plain assignment shares storage and only the first mutating call pays
for a deep copy. Reach for `RcBox` directly when building a similar
COW type; for plain shared ownership without mutation prefer a more
purpose-built container.

### Examples

```
let a = RcBox(value: [1, 2, 3]);
let b = a.clone();          // shares storage; refCount == 2
if b.isUnique() { ... } else { let c = b.deepClone(); /* ... */ }
```

### Representation

One `Pointer[RcBoxStorage[T]]`. The pointed-to block holds an `Int64`
refcount followed by the `T` value, allocated via `SystemAllocator`.

### Memory Model

Reference-counted, non-atomic (today — see TODOs). `clone()` increments
the count and shares storage; `deinit` decrements and frees on zero.
`deepClone()` allocates a fresh `RcBox` carrying a copied value.

### Guarantees

- `isUnique()` returning `true` means in-place mutation is safe; this is
  how COW types decide whether to copy.
- The refcount is currently **not** atomic, so `RcBox` is not safe to
  share across threads.

_Defined in `lang/std/memory/rcbox.ks`._

### Members

#### initializer `From Value`

```kestrel
public init(T)
```

Allocates fresh storage holding `value` with refcount 1. Panics if
the underlying `SystemAllocator` returns `.None`.

##### Errors

Panics with `"RcBox allocation failed"` on allocation failure.

_Defined in `lang/std/memory/rcbox.ks`._

#### function `deepClone`

```kestrel
public func deepClone() -> RcBox[T]
```

Allocates fresh storage with a copy of the value. Used by COW
types when `isUnique()` returns `false` — splits off a private
copy so the caller can mutate without affecting other clones.

_Defined in `lang/std/memory/rcbox.ks`._

#### function `getValue`

```kestrel
public func getValue() -> T
```

Reads the wrapped value out of storage. Returns a copy — the
underlying `T` is read through a pointer, so callers see a
snapshot, not a live reference.

_Defined in `lang/std/memory/rcbox.ks`._

#### function `isUnique`

```kestrel
public func isUnique() -> Bool
```

Returns `true` when no other clone is sharing storage. The litmus
test for "safe to mutate in place" in COW collections.

_Defined in `lang/std/memory/rcbox.ks`._

#### function `refCount`

```kestrel
public func refCount() -> Int64
```

Current strong reference count. Mostly useful for tests and
diagnostics; production COW logic should branch on `isUnique`.

_Defined in `lang/std/memory/rcbox.ks`._

#### function `setValue`

```kestrel
public func setValue(consuming T)
```

Overwrites the wrapped value in place. Safe only when this is the
unique owner (`isUnique() == true`); otherwise other clones see the
new value, defeating COW. The COW types check `isUnique` before
calling this and `deepClone` otherwise.
Takes `value` by consuming — the caller's copy is dead after this.

_Defined in `lang/std/memory/rcbox.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> RcBox[T]
```

Bumps the refcount and returns a second `RcBox` pointing at the
same storage. The receiver and the returned box now both reference
the value; the next mutation should test `isUnique`.

_Defined in `lang/std/memory/rcbox.ks`._

## struct `SystemAllocator`

```kestrel
public struct SystemAllocator { /* private fields */ }
```

`Allocator` backed by libc `malloc`/`free`/`realloc`. Used as the
default `GlobalAllocator` and by every collection that doesn't pick a
custom allocator.

### Memory Model

Stateless: the struct holds no fields. All bookkeeping lives in libc's
heap. Cloning or copying the allocator has no effect on the heap state.

_Defined in `lang/std/memory/allocator.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Builds a stateless system allocator. No heap interaction occurs here.

_Defined in `lang/std/memory/allocator.ks`._

### Implements `Allocator`

#### function `allocate`

```kestrel
public mutating func allocate(Layout) -> RawPointer?
```

Calls `malloc(layout.size)`. Alignment beyond `malloc`'s natural
alignment (typically 16) is **not** honoured — types that need
larger alignment should use a different allocator.

_Defined in `lang/std/memory/allocator.ks`._

#### function `deallocate`

```kestrel
public mutating func deallocate(RawPointer, Layout)
```

Calls `free(ptr)`. The `layout` argument is ignored — kept for
protocol conformance; allocators that need it (arenas) use it.

_Defined in `lang/std/memory/allocator.ks`._

#### function `reallocate`

```kestrel
public mutating func reallocate(RawPointer, Layout, Layout) -> RawPointer?
```

Calls `realloc(ptr, newLayout.size)`. As with `allocate`, only
`malloc`-natural alignment is guaranteed.

_Defined in `lang/std/memory/allocator.ks`._

