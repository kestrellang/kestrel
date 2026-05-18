# std.collections

## struct `Array`

```kestrel
public struct Array[T] { /* private fields */ }
```

A dynamic, growable, contiguous-buffer array with copy-on-write storage.

`Array[T]` is the standard ordered-collection type. It supports
constant-time random access, amortized constant-time `append`, and
arbitrary-position insert/remove via shifting. Storage is shared between
copies until one of them mutates, at which point that copy lazily clones
the buffer (see "Memory Model" below). For non-owning views over an
existing buffer use `ArraySlice[T]`; for fixed-size or set-like collections
see `ArraySlice[T]`, `Set`, or `Dictionary`.

### Examples

```
let evens = [2, 4, 6, 8];
var names = Array[String]();
names.append("Alice");
names.append("Bob");

let copy = names;      // O(1) — shares storage with `names`
names.append("Carol"); // O(1) clone happens here, `copy` is unchanged

for n in names.iter() { ... }
let pivot = names.partition(by: { (n) in n.count > 3 });
```

### Indexing

The default subscript `arr(i)` panics on out-of-bounds. Variants exist
for every common policy: `arr(checked: i)` returns `T?`,
`arr(unchecked: i)` skips the bounds check (UB on OOB),
`arr(wrapped: i)` wraps with modulo (and supports negative indices),
and `arr(clamped: i)` clamps to `[0, count-1]`. Range arguments use the
same labels — `arr(0..<3)`, `arr(checked: r)`, `arr(unchecked: r)`,
`arr(clamped: r)` — dispatched through the unified `SeqIndex[T]`,
`SeqClampable[T]`, and `SeqWrappable[T]` protocols. `Int64` and range
types share each label; the result type varies (`T?` vs `ArraySlice[T]`
for `clamped:`).

### Capacity & Reallocation

`count` is the number of elements; `capacity` is how many can fit
without reallocating. When `append` would exceed capacity the buffer
doubles (starting from 4 if previously zero). Use
`reserveCapacity(minimumCapacity:)` to pre-allocate, and
`shrinkToFit()` to release excess.

### Representation

Holds a single `CowBox[ArrayStorage[T]]` field. The storage is a
`(ptr, len, cap)` triple over a heap-allocated buffer.

### Memory Model

Reference-counted storage with copy-on-write *value* semantics. Copying
an `Array` is O(1) and shares the buffer; the next mutation on a shared
`Array` triggers `makeUnique()`, which deep-clones the buffer so the
mutation is invisible to other copies. The user-visible behavior is
indistinguishable from deep-copying on assignment.

### Guarantees

- Elements are stored contiguously and are accessible via `asPointer()`
  for FFI; the pointer is invalidated by any mutation that may
  reallocate.
- `count <= capacity` always.
- Iteration order is insertion order.
- Operations marked O(1) are amortized; growth is geometric.

_Defined in `lang/std/collections/array.ks`._

### Members

#### initializer `Array Literal`

```kestrel
public init(arrayLiteral: LiteralSlice[T])
```

Creates an array containing every element of the supplied literal
slice.

Allocates a buffer sized exactly to the literal's element count
(so `capacity == count` after construction) and copies the
elements over. An empty slice yields an empty unallocated array.
Panics if allocation fails.

##### Examples

```
// Triggered by the array-literal syntax:
let arr: Array[Int64] = [10, 20, 30];
```

_Defined in `lang/std/collections/array.ks`._

#### initializer `Empty`

```kestrel
public init()
```

Creates an empty array with no allocation.

Capacity starts at zero; the first `append` allocates a small
buffer (currently 4 elements). Use `init(capacity:)` if you can
pre-size to avoid the early growth steps.

##### Examples

```
var arr = Array[Int64]();
arr.count;     // 0
arr.capacity;  // 0
```

_Defined in `lang/std/collections/array.ks`._

#### initializer `From Generator`

```kestrel
public init(of: Int64, generatedBy: (Int64) -> T)
```

Creates an array of `count` elements computed by a per-index closure.

Allocates exactly `count` slots and invokes `gen(i)` once for each
`i` in `0..<count`. `count <= 0` produces an empty array. Use this
when each slot is a function of its index; for a constant value,
prefer `init(repeating:count:)`.

##### Examples

```
let squares = Array(of: 5, generatedBy: { (i) in i * i });  // [0, 1, 4, 9, 16]
let indices = Array(of: 3, generatedBy: { (i) in i });      // [0, 1, 2]
let empty   = Array(of: 0, generatedBy: { (i) in i });      // []
```

_Defined in `lang/std/collections/array.ks`._

#### initializer `From Iterable`

```kestrel
public init[I](from: I) where I: Iterable, I.Item == T
```

Creates an array by collecting every element produced by an iterable.

Drains `iterable` to completion via `append`, so the resulting
capacity is whatever the growth policy lands on (not necessarily
equal to `count`). For a sized source you can shave reallocations
by following with `shrinkToFit()`. See also `append(from:)`
to add elements to an existing array.

##### Examples

```
let fromRange = Array(from: 1..<5);         // [1, 2, 3, 4]
let fromSet   = Array(from: mySet);         // arbitrary order
let collected = Array(from: lines.iter());  // exhausts the iterator
```

_Defined in `lang/std/collections/array.ks`._

#### initializer `From Storage`

```kestrel
init(storage: CowBox[ArrayStorage[T]])
```

Wraps an existing storage box in a new `Array`.

Module-internal — used by `clone()`, `ArrayBuilder.build()`, and
other `std.collections` code that constructs arrays from raw
storage.

_Defined in `lang/std/collections/array.ks`._

#### initializer `Literal Bridge`

```kestrel
public init(_arrayLiteralPointer: consuming lang.ptr[T], _arrayLiteralCount: consuming lang.i64)
```

Compiler-emitted bridge initializer for `[a, b, c]` array literals.

Not called by user code directly — the parser lowers literal
expressions into a `(ptr, count)` pair which this constructor wraps
in a `LiteralSlice` and forwards to `init(arrayLiteral:)`.

##### Safety

The compiler guarantees `_arrayLiteralPointer` points to exactly
`_arrayLiteralCount` initialized elements of `T`.

##### Examples

```
let arr = [1, 2, 3];   // emitted by the compiler as a call to this init
```

_Defined in `lang/std/collections/array.ks`._

#### initializer `Repeating Value`

```kestrel
public init(repeating: T, count: Int64)
```

Creates an array of `count` identical copies of `value`.

Allocates exactly `count` slots and writes the same value into each.
`count <= 0` produces an empty array. Useful for initializing
fixed-size buffers; if you instead want each slot computed, use
`init(count:generator:)`.

##### Examples

```
let zeros = Array(repeating: 0, count: 5);    // [0, 0, 0, 0, 0]
let empty = Array(repeating: "x", count: 0);  // []
let pad   = Array(repeating: " ", count: 3);  // [" ", " ", " "]
```

_Defined in `lang/std/collections/array.ks`._

#### initializer `With Capacity`

```kestrel
public init(capacity: Int64)
```

Creates an empty array with at least the requested capacity reserved.

Equivalent to `Array()` followed by `reserveCapacity(...)`, but
done in a single allocation. A non-positive `capacity` behaves
like `init()` (no allocation). Panics if allocation fails.

##### Examples

```
var arr = Array[Int64](capacity: 1000);
arr.count;     // 0
arr.capacity;  // >= 1000 — no reallocation for first 1000 appends
```

_Defined in `lang/std/collections/array.ks`._

#### function `append`

```kestrel
public mutating func append(T)
```

Appends `element` to the end of the array.

Amortized O(1). Triggers a reallocation (and COW if storage is
shared) when `count == capacity`. For appending many elements,
`reserveCapacity(...)` first to avoid intermediate growths; for
adding multiple elements at once see `append(contentsOf:)` or
`append(from:)`.

##### Examples

```
var arr = [1, 2];
arr.append(3);  // [1, 2, 3]
```

_Defined in `lang/std/collections/array.ks`._

#### function `append`

```kestrel
public mutating func append[__opaque_0](contentsOf: __opaque_0) where __opaque_0: Slice[T]
```

Appends every element of `other` to the end of this array.

Reserves the exact required capacity in one growth step then
copies the elements over, so it's faster than calling `append`
in a loop. Sharing semantics: `other` is read-only here, but if
`self` shares storage with anything else, COW fires once at the
start. See also `append(from:)` for arbitrary iterable
sources.

##### Examples

```
var arr = [1, 2];
arr.append(contentsOf: [3, 4]);  // [1, 2, 3, 4]
arr.append(contentsOf: []);      // [1, 2, 3, 4]  — no-op
```

_Defined in `lang/std/collections/array.ks`._

#### function `append`

```kestrel
public mutating func append[I](from: I) where I: Iterable, I.Item == T
```

Appends every element produced by an arbitrary iterable.

Drains the iterable via `append`, so capacity grows geometrically
rather than to an exact target — for sized sources like another
`Array`, prefer `append(contentsOf:)`.

##### Examples

```
var arr = [1, 2];
arr.append(from: 3..<6);  // [1, 2, 3, 4, 5]
```

_Defined in `lang/std/collections/array.ks`._

#### field `capacity`

```kestrel
public var capacity: Int64 { get }
```

The number of elements the buffer can hold without reallocating.

_Defined in `lang/std/collections/array.ks`._

#### function `clear`

```kestrel
public mutating func clear()
```

Removes every element from the array, leaving capacity untouched.

O(1). The buffer is kept so subsequent appends don't reallocate
— if you want the memory back, follow with `shrinkToFit()`.

##### Examples

```
var arr = [1, 2, 3];
arr.clear();    // arr is []
arr.capacity;   // unchanged
```

_Defined in `lang/std/collections/array.ks`._

#### function `dedup`

```kestrel
public mutating func dedup()
```

Removes runs of consecutive equal elements, in place.

Only adjacent duplicates collapse — non-adjacent equal values are
kept. To deduplicate globally, `sort()` first or, for `Hashable`
elements, use the `unique()` / `removeDuplicates()` extension
methods. The non-mutating variant is `deduped()`.

##### Examples

```
var arr = [1, 1, 2, 2, 2, 3, 1, 1];
arr.dedup();  // [1, 2, 3, 1] — trailing 1s survive (not adjacent to first run)
```

_Defined in `lang/std/collections/array.ks`._

#### function `deduped`

```kestrel
public func deduped() -> Array[T]
```

Returns a new array with consecutive duplicates removed; original
is unchanged.

Non-mutating mirror of `dedup()`. Same caveat: only adjacent
duplicates collapse.

##### Examples

```
[1, 1, 2, 2, 3].deduped();        // [1, 2, 3]
[1, 2, 1, 2].deduped();           // [1, 2, 1, 2] — none are adjacent
```

_Defined in `lang/std/collections/array.ks`._

#### function `flatten`

```kestrel
public func flatten() -> Array[T.Item]
```

Concatenates each element's iterator into a single
`Array[T.Item]`.

Drains every inner iterator in order. Empty inner sequences
disappear without affecting the surrounding ones. Element type
of the result is `T.Item`, the inner iterable's item type.

##### Examples

```
let nested = [[1, 2], [3, 4], [5]];
nested.flatten();  // [1, 2, 3, 4, 5]

let mixed = [[1], [], [2, 3]];
mixed.flatten();   // [1, 2, 3]
```

_Defined in `lang/std/collections/array.ks`._

#### function `insert`

```kestrel
public mutating func insert(T, at: Int64)
```

Inserts `element` at `index`, shifting later elements right by one.

O(n) in the number of elements after `index`. `index == count`
behaves like `append`. Triggers COW and may reallocate. For bulk
insertion at one location, prefer
`replaceSubrange(i..<i, with: ...)`.

##### Errors

Panics with `"Array.insert: index out of bounds"` if `index < 0`
or `index > count`.

##### Examples

```
var arr = [1, 3];
arr.insert(2, at: 1);  // [1, 2, 3]
arr.insert(0, at: 0);  // [0, 1, 2, 3]
arr.insert(4, at: 4);  // [0, 1, 2, 3, 4]  — append-equivalent
arr.insert(9, at: 99); // PANIC
```

_Defined in `lang/std/collections/array.ks`._

#### function `joined`

```kestrel
public func joined(String) -> String
```

Concatenates each element's string representation, separated by
`separator`.

Each element is rendered with its `format()` method using default
`FormatOptions`. The default `separator` is empty (raw
concatenation). Empty arrays produce `""`. For the bracketed
debug form (`"[1, 2, 3]"`), use `format()` directly.

##### Examples

```
[1, 2, 3].joined(", ");  // "1, 2, 3"
[1, 2, 3].joined();       // "123"
["a", "b"].joined("-");   // "a-b"
[].joined(", ");          // ""
```

_Defined in `lang/std/collections/array.ks`._

#### function `partition`

```kestrel
public mutating func partition(by: (T) -> Bool) -> Int64
```

Reorders elements in place so that all matching elements come
before all non-matching elements; returns the partition point.

The returned index is the count of matching elements (and the
index of the first non-matching one). This is an *unstable*
partition — relative order within each side is not preserved.
For a stable, allocating variant that returns two arrays, use
`partitioned(by:)`.

##### Examples

```
var arr = [1, 2, 3, 4, 5];
let pivot = arr.partition(by: { (x) in x % 2 == 0 });
// arr might be [2, 4, 3, 1, 5] (or another valid permutation)
// pivot == 2 — first two elements satisfy the predicate
```

_Defined in `lang/std/collections/array.ks`._

#### function `partitioned`

```kestrel
public func partitioned(by: (T) -> Bool) -> (Array[T], Array[T])
```

Returns two new arrays: elements matching `predicate` first, then
elements that don't.

Stable: relative order within each side is preserved. Allocates
two new arrays — use `partition(by:)` for an in-place, unstable
reordering that avoids the allocation.

##### Examples

```
let (evens, odds) = [1, 2, 3, 4, 5].partitioned(by: { (x) in x % 2 == 0 });
// evens = [2, 4]
// odds  = [1, 3, 5]
```

_Defined in `lang/std/collections/array.ks`._

#### function `pop`

```kestrel
public mutating func pop() -> T?
```

Removes and returns the last element, or `None` if the array is empty.

O(1). Capacity is retained for reuse — only `len` is decremented.
The mirror operation `popFirst()` is O(n) because it must shift
the remainder. To inspect the last element without removing, use
`last()`.

##### Examples

```
var arr = [1, 2, 3];
arr.pop();  // Some(3), arr is [1, 2]
arr.pop();  // Some(2), arr is [1]
arr.pop();  // Some(1), arr is []
arr.pop();  // None,    arr is still []
```

_Defined in `lang/std/collections/array.ks`._

#### function `popFirst`

```kestrel
public mutating func popFirst() -> T?
```

Removes and returns the first element, or `None` if the array is
empty.

O(n) — every following element shifts left by one. If you can
tolerate it, `pop()` from the back is O(1). For inspection
without removal, use `first()`.

##### Examples

```
var arr = [1, 2, 3];
arr.popFirst();  // Some(1), arr is [2, 3]
arr.popFirst();  // Some(2), arr is [3]
```

_Defined in `lang/std/collections/array.ks`._

#### function `remove`

```kestrel
public mutating func remove(at: Int64) -> T
```

Removes and returns the element at `index`, shifting later
elements left.

O(n - index). Capacity is retained. For removing many elements at
once, prefer `removeSubrange(range:)`. To remove the *first*
element by *value* see the `Equatable` extension's
`remove(element:)`.

##### Errors

Panics with `"Array.remove: index out of bounds"` if `index < 0`
or `index >= count`.

##### Examples

```
var arr = [1, 2, 3, 4];
arr.remove(at: 1);  // returns 2; arr is [1, 3, 4]
arr.remove(at: 9);  // PANIC
```

_Defined in `lang/std/collections/array.ks`._

#### function `remove`

```kestrel
public mutating func remove(T) -> Bool
```

Removes the first element equal to `element`. Returns whether a
removal occurred.

Performs `firstIndex(of:)` then `remove(at:)`. To strip every
occurrence in one pass, use `removeAll(element:)`.

##### Examples

```
var arr = [1, 2, 3, 2];
arr.remove(2);  // true; arr is [1, 3, 2]
arr.remove(5);  // false; arr unchanged
```

_Defined in `lang/std/collections/array.ks`._

#### function `removeAll`

```kestrel
public mutating func removeAll(where: (T) -> Bool)
```

Removes every element for which `predicate` returns true.

The inverse of `retain(where:)` — implemented as
`retain` over the negated predicate. O(n), stable.

##### Examples

```
var arr = [1, 2, 3, 4, 5];
arr.removeAll(where: { (x) in x % 2 == 0 });  // [1, 3, 5]

var names = ["Alice", "", "Bob", ""];
names.removeAll(where: { (s) in s.isEmpty });  // ["Alice", "Bob"]
```

_Defined in `lang/std/collections/array.ks`._

#### function `removeAll`

```kestrel
public mutating func removeAll(T)
```

Removes every element equal to `element`.

Implemented as `retain` with a negated equality predicate —
O(n), single pass, stable. To remove only the first occurrence
use `remove(element:)`.

##### Examples

```
var arr = [1, 2, 3, 2, 4, 2];
arr.removeAll(2);  // [1, 3, 4]
```

_Defined in `lang/std/collections/array.ks`._

#### function `removeDuplicates`

```kestrel
public mutating func removeDuplicates()
```

Removes every duplicate in place, keeping the first occurrence.

Implemented by replacing storage with the result of `unique()`,
so the same O(n²) caveat applies. The non-mutating mirror is
`unique()`.

##### Examples

```
var arr = [1, 2, 1, 3, 2];
arr.removeDuplicates();  // [1, 2, 3]
```

_Defined in `lang/std/collections/array.ks`._

#### function `removeSubrange`

```kestrel
public mutating func removeSubrange[R](R) where R: SeqRange
```

Removes every element in `range`, shifting later elements left.

O(count - range.end + range.length). Empty ranges are no-ops.
Capacity is retained — call `shrinkToFit()` to release it. For
"remove these and put others back" use `replaceSubrange(...)`.

##### Errors

Panics with `"Array.removeSubrange: range out of bounds"` if
`range.start < 0`, `range.end > count`, or
`range.start > range.end`.

##### Examples

```
var arr = [1, 2, 3, 4, 5];
arr.removeSubrange(1..<4);  // arr is [1, 5]
arr.removeSubrange(0..<0);  // no-op
```

_Defined in `lang/std/collections/array.ks`._

#### function `replaceSubrange`

```kestrel
public mutating func replaceSubrange[R](R, with: Array[T]) where R: SeqRange
```

Replaces the elements in `range` with the elements of `replacement`.

`replacement.count` need not equal the range length — the array
shrinks or grows accordingly, shifting the trailing elements once.
Use `range == i..<i` to insert without removing, or
`replacement == []` to remove without inserting (equivalent to
`removeSubrange(...)`). May reallocate; triggers COW.

##### Errors

Panics with `"Array.replaceSubrange: range out of bounds"` if
`range.start < 0`, `range.end > count`, or
`range.start > range.end`.

##### Examples

```
var arr = [1, 2, 3, 4, 5];
arr.replaceSubrange(1..<4, with: [20, 30]);    // [1, 20, 30, 5]
arr.replaceSubrange(1..<1, with: [9, 9]);      // insert: [1, 9, 9, 20, 30, 5]
arr.replaceSubrange(0..<2, with: Array[Int64]());  // remove: [9, 20, 30, 5]
```

_Defined in `lang/std/collections/array.ks`._

#### function `reserveCapacity`

```kestrel
public mutating func reserveCapacity(Int64)
```

Reserves enough capacity to hold at least `minimumCapacity` elements.

A no-op when capacity already suffices. The actual capacity after
the call may exceed the request because growth rounds up via the
doubling policy. Pair with bulk inserts to skip intermediate
reallocations. The opposite operation is `shrinkToFit()`.

##### Examples

```
var arr = Array[Int64]();
arr.reserveCapacity(1000);
for i in 0..<1000 {
        arr.append(i);  // no reallocations
}
```

_Defined in `lang/std/collections/array.ks`._

#### function `retain`

```kestrel
public mutating func retain(where: (T) -> Bool)
```

Keeps only elements for which `predicate` returns true; removes
the rest in place.

O(n), single pass, stable (relative order preserved). The mirror
operation is `removeAll(where:)`. For a copy instead of an
in-place edit, use `iter().filter(...).collect()`.

##### Examples

```
var arr = [1, 2, 3, 4, 5];
arr.retain(where: { (x) in x % 2 == 0 });  // [2, 4]
```

_Defined in `lang/std/collections/array.ks`._

#### function `reverse`

```kestrel
public mutating func reverse()
```

Reverses the order of elements in place.

O(n). Triggers COW. For a non-mutating variant returning a new
array, use `reversed()`.

##### Examples

```
var arr = [1, 2, 3];
arr.reverse();  // [3, 2, 1]
```

_Defined in `lang/std/collections/array.ks`._

#### function `rotate`

```kestrel
public mutating func rotate(by: Int64)
```

Rotates the elements in place by `amount` positions to the left.

Implemented with the three-reversal algorithm — O(n) time,
O(1) extra space. Negative `amount` rotates right; the actual
rotation is `amount mod count`, so very large amounts wrap. A
no-op when `count <= 1` or the normalized amount is zero.

##### Examples

```
var arr = [1, 2, 3, 4, 5];
arr.rotate(by:  2);  // [3, 4, 5, 1, 2]
arr.rotate(by: -1);  // [2, 3, 4, 5, 1]
arr.rotate(by:  7);  // same as rotate(by: 2) for count == 5
```

_Defined in `lang/std/collections/array.ks`._

#### function `shrinkToFit`

```kestrel
public mutating func shrinkToFit()
```

Releases unused capacity by reallocating to fit `count` exactly.

Useful after a bulk removal or when you've finished building a
large array. A no-op when `capacity == count`. For an empty
array, fully deallocates the buffer (capacity drops to 0).
Triggers COW.

##### Examples

```
var arr = Array[Int64](capacity: 1000);
arr.append(1);
arr.shrinkToFit();   // capacity reduced to 1
arr.clear();
arr.shrinkToFit();   // capacity reduced to 0, buffer freed
```

_Defined in `lang/std/collections/array.ks`._

#### function `shuffle`

```kestrel
public mutating func shuffle[__opaque_0](using: __opaque_0) where __opaque_0: RandomNumberGenerator
```

Shuffles the array in place using `rng`.

Uses the Fisher-Yates algorithm — every permutation is equally
likely, given a uniform RNG. Passing the same seeded `rng`
produces a deterministic shuffle, which is the usual reason to
reach for this overload over the no-arg `shuffle()`.

##### Examples

```
var arr = [1, 2, 3, 4, 5];
var rng = Lcg64(seed: 42);
arr.shuffle(using: rng);  // deterministic for the seed
```

_Defined in `lang/std/collections/array.ks`._

#### function `shuffle`

```kestrel
public mutating func shuffle()
```

Shuffles the array in place using a fresh default RNG.

Convenience over `shuffle(using:)`. The result is non-deterministic
across calls — pass an explicit `Lcg64(seed: ...)` (or other
`RandomNumberGenerator`) when you need reproducibility.

##### Examples

```
var arr = [1, 2, 3, 4, 5];
arr.shuffle();  // e.g. [3, 1, 5, 2, 4]
```

_Defined in `lang/std/collections/array.ks`._

#### function `shuffled`

```kestrel
public func shuffled[__opaque_0](using: __opaque_0) -> Array[T] where __opaque_0: RandomNumberGenerator
```

Returns a new array shuffled with `rng`. The original is unchanged.

The non-mutating mirror of `shuffle(using:)`. Internally clones via
COW (cheap until the next mutation) and shuffles the copy.

##### Examples

```
let arr = [1, 2, 3, 4, 5];
var rng = Lcg64(seed: 42);
let result = arr.shuffled(using: rng);
// arr is still [1, 2, 3, 4, 5]
```

_Defined in `lang/std/collections/array.ks`._

#### function `shuffled`

```kestrel
public func shuffled() -> Array[T]
```

Returns a new array shuffled with a default RNG. Original unchanged.

Convenience over `shuffled(using:)`. Non-deterministic between
calls.

##### Examples

```
let arr = [1, 2, 3, 4, 5];
let shuffled = arr.shuffled();  // e.g. [4, 2, 5, 1, 3]
// arr is still [1, 2, 3, 4, 5]
```

_Defined in `lang/std/collections/array.ks`._

#### function `sort`

```kestrel
public mutating func sort()
```

Sorts the array in ascending order using the natural `<` ordering.

Uses introsort — O(n log n) worst-case. For descending or custom
orderings pass a comparator to `sort(by:)`. Non-mutating variant:
`sorted()`.

##### Examples

```
var arr = [3, 1, 4, 1, 5];
arr.sort();  // [1, 1, 3, 4, 5]
```

_Defined in `lang/std/collections/array.ks`._

#### function `sort`

```kestrel
public mutating func sort(by: (T, T) -> Bool)
```

Sorts the array in place using a `<`-style comparator.

The comparator returns `true` when its first argument should come
before the second. Uses introsort — quicksort with heapsort
fallback when recursion exceeds 2·log₂(n), and insertion sort for
partitions ≤ 16 elements. O(n log n) worst-case. Pass a reversed
comparator for descending order.

##### Examples

```
var arr = [1, 5, 3, 2, 4];
arr.sort(by: { (a, b) in a > b });  // [5, 4, 3, 2, 1] descending
```

_Defined in `lang/std/collections/array.ks`._

#### function `sort`

```kestrel
public mutating func sort[K](byKey: (T) -> K) where K: Comparable
```

Sorts the array in place by an extracted `Comparable` key.

##### Examples

```
var people = [Person("Alice", 30), Person("Bob", 25)];
people.sort(byKey: { (p) in p.age });
```

_Defined in `lang/std/collections/array.ks`._

#### function `sorted`

```kestrel
public func sorted(by: (T, T) -> Bool) -> Array[T]
```

Returns a new array sorted by a custom comparator. Original unchanged.

##### Examples

```
let arr = [3, 1, 2];
let desc = arr.sorted(by: { (a, b) in a > b });  // [3, 2, 1]
```

_Defined in `lang/std/collections/array.ks`._

#### function `sorted`

```kestrel
public func sorted[K](byKey: (T) -> K) -> Array[T] where K: Comparable
```

Returns a new array sorted by an extracted `Comparable` key;
original unchanged.

##### Examples

```
let words = ["hi", "hello", "hey"];
let byLen = words.sorted(byKey: { (w) in w.count });
```

_Defined in `lang/std/collections/array.ks`._

#### function `swap`

```kestrel
public mutating func swap(at: Int64, with: Int64)
```

Swaps the elements at indices `i` and `j` in place.

O(1). A no-op when `i == j`. Triggers COW.

##### Errors

Panics with `"Array.swap: index out of bounds"` if either index
is `< 0` or `>= count`.

##### Examples

```
var arr = [1, 2, 3];
arr.swap(at: 0, with: 2);  // [3, 2, 1]
arr.swap(at: 1, with: 1);  // [3, 2, 1] — no-op
arr.swap(at: 0, with: 9);  // PANIC
```

_Defined in `lang/std/collections/array.ks`._

### Implements `Slice`

#### function `asSlice`

```kestrel
public func asSlice() -> ArraySlice[T]
```

Slice protocol kernel — borrows the array's buffer as an ArraySlice.

_Defined in `lang/std/collections/array.ks`._

#### function `ensureUnique`

```kestrel
public mutating func ensureUnique()
```

COW write barrier — deep-copies storage if shared.

_Defined in `lang/std/collections/array.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = T
```

`Iterable` element type — the element produced by `iter().next()`.

_Defined in `lang/std/collections/array.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = ArraySliceIterator[T]
```

`Iterable` iterator type — the concrete iterator returned by `iter()`.

_Defined in `lang/std/collections/array.ks`._

#### function `iter`

```kestrel
public func iter() -> ArraySliceIterator[T]
```

Returns a forward iterator over the array's elements.

_Defined in `lang/std/collections/array.ks`._

### Implements `ExpressibleByArrayLiteral`

#### initializer `Array Literal`

```kestrel
init(arrayLiteral: LiteralSlice[Element])
```

Builds an instance from a literal slice of elements.

_Defined in `lang/std/core/literals.ks`._

### Implements `_ExpressibleByArrayLiteral`

#### typealias `Element`

```kestrel
type Element = T
```

Pattern-matching element type — used by `ArrayMatchable` for
`[a, b, ..rest]` patterns.

_Defined in `lang/std/collections/array.ks`._

#### typealias `Element`

```kestrel
type Element = T
```

`ArrayMatchable` element type — what the pattern bindings extract.

_Defined in `lang/std/collections/array.ks`._

#### initializer `Literal Bridge`

```kestrel
init(_arrayLiteralPointer: consuming lang.ptr[Element], _arrayLiteralCount: consuming lang.i64)
```

Compiler-emitted init taking a raw pointer and count.

Both params are `consuming`: the compiler hands ownership of the
stack buffer's address (and the count) over to the implementation,
which stores them in its own storage. This convention is what the
MIR lowering's structural predicate looks for — implementations
that deviate will be silently skipped during literal lowering.

_Defined in `lang/std/core/literals.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> Array[T]
```

Returns an `Array[T]` sharing the same storage; the deep copy is
deferred until either side mutates.

O(1) — just bumps the storage `CowBox`'s refcount. The first
mutation on either the original or the clone triggers
`makeUnique()`, which deep-copies the buffer so the two arrays
diverge.

##### Examples

```
let a = [1, 2, 3];
var b = a.clone();  // O(1), shares storage
b.append(4);        // b deep-copies here; a is unchanged
```

_Defined in `lang/std/collections/array.ks`._

### Implements `Defaultable`

#### initializer `Default`

```kestrel
init()
```

Builds the default-valued instance.

_Defined in `lang/std/core/protocols.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
func isEqual(to: Self) -> Bool
```

Returns `true` iff `self` and `other` are considered equal. Should
be reflexive, symmetric, and transitive — `Hashable` requires equal
values to hash equal, so don't drift from those laws.

_Defined in `lang/std/core/protocols.ks`._

### Implements `ArrayMatchable`

#### typealias `Element`

```kestrel
type Element
```

_Defined in `lang/std/core/protocols.ks`._

#### function `matchGet`

```kestrel
public func matchGet(Int64) -> T
```

Pattern-matcher hook reading the element at `index` (no bounds
check).

##### Safety

The matcher only calls this with indices it has already validated
against `matchLength()`, so the unchecked read is safe in that
context.

_Defined in `lang/std/collections/array.ks`._

#### function `matchLength`

```kestrel
public func matchLength() -> Int64
```

Pattern-matcher hook returning the array's `count`.

Used by the matcher to decide whether the scrutinee has enough
elements for a fixed-arity pattern.

_Defined in `lang/std/collections/array.ks`._

#### function `matchSlice`

```kestrel
public func matchSlice(Int64, Int64) -> ArraySlice[T]
```

Pattern-matcher hook returning the half-open `[from, to)` slice.

Used to bind `..rest` segments. The matcher guarantees the
indices are in range.

_Defined in `lang/std/collections/array.ks`._

## struct `ArrayBuilder`

```kestrel
public struct ArrayBuilder[T] { /* private fields */ }
```

Write-only buffer for efficient array construction. No COW, no
`RcBox`, no `isUnique` checks — every append writes directly through
the pointer.

`build()` transfers ownership of the buffer into a new `Array[T]`
without copying. The builder resets to empty and can be reused.

### Examples

```
var b = ArrayBuilder[Int64](capacity: 3);
b.append(1);
b.append(2);
b.append(3);
let arr = b.build();   // [1, 2, 3], zero-copy
```

### Representation

`(ptr: Pointer[T], len: Int64, cap: Int64)`.

### Memory Model

Owns its buffer directly — no reference counting during
construction. `build()` donates the buffer to an `Array[T]` and
leaves the builder empty. `deinit` frees the buffer if `build()`
was never called.

_Defined in `lang/std/collections/builder.ks`._

### Members

#### initializer `Empty`

```kestrel
public init()
```

Creates an empty builder with no allocation.

_Defined in `lang/std/collections/builder.ks`._

#### initializer `With Capacity`

```kestrel
public init(capacity: Int64)
```

Creates an empty builder with at least `capacity` elements
preallocated.

_Defined in `lang/std/collections/builder.ks`._

#### function `append`

```kestrel
public mutating func append(T)
```

Appends a single element.

_Defined in `lang/std/collections/builder.ks`._

#### function `append`

```kestrel
public mutating func append(contentsOf: ArraySlice[T])
```

Appends every element of `slice`.

_Defined in `lang/std/collections/builder.ks`._

#### function `append`

```kestrel
public mutating func append[I](from: I) where I: Iterable, I.Item == T
```

Appends every element produced by `iterable`.

_Defined in `lang/std/collections/builder.ks`._

#### function `build`

```kestrel
public mutating func build() -> Array[T]
```

Transfers the buffer into a new `Array[T]` without copying. The
builder resets to empty and can be reused.

_Defined in `lang/std/collections/builder.ks`._

#### field `capacity`

```kestrel
public var capacity: Int64 { get }
```

Allocated capacity in elements.

_Defined in `lang/std/collections/builder.ks`._

#### function `clear`

```kestrel
public mutating func clear()
```

Resets length to zero, keeping the allocated buffer for reuse.

_Defined in `lang/std/collections/builder.ks`._

#### field `count`

```kestrel
public var count: Int64 { get }
```

Number of elements written so far.

_Defined in `lang/std/collections/builder.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

True when nothing has been written.

_Defined in `lang/std/collections/builder.ks`._

## struct `ArraySplitIterator`

```kestrel
public struct ArraySplitIterator[T] where T: Equatable { /* private fields */ }
```

_Defined in `lang/std/collections/views.ks`._

### Members

#### initializer `init`

```kestrel
public init(ptr: Pointer[T], remaining: Int64, separator: T, done: Bool)
```

_Defined in `lang/std/collections/views.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = ArraySlice[T]
```

_Defined in `lang/std/collections/views.ks`._

#### function `next`

```kestrel
public mutating func next() -> Optional[ArraySlice[T]]
```

_Defined in `lang/std/collections/views.ks`._

## struct `ArraySplitView`

```kestrel
public struct ArraySplitView[T] where T: Equatable { /* private fields */ }
```

Multi-pass lazy view over the segments produced by splitting on each
occurrence of a separator value. (Named `ArraySplitView` to avoid
collision with `std.text.SplitView`.)

_Defined in `lang/std/collections/views.ks`._

### Members

#### field `count`

```kestrel
public var count: Int64 { get }
```

_Defined in `lang/std/collections/views.ks`._

#### initializer `init`

```kestrel
public init(slice: ArraySlice[T], separator: T)
```

_Defined in `lang/std/collections/views.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

_Defined in `lang/std/collections/views.ks`._

#### function `toArray`

```kestrel
public func toArray() -> Array[ArraySlice[T]]
```

_Defined in `lang/std/collections/views.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = ArraySlice[T]
```

_Defined in `lang/std/collections/views.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = ArraySplitIterator[T]
```

_Defined in `lang/std/collections/views.ks`._

#### function `iter`

```kestrel
public func iter() -> ArraySplitIterator[T]
```

_Defined in `lang/std/collections/views.ks`._

## struct `ArraySplitWhereIterator`

```kestrel
public struct ArraySplitWhereIterator[T] { /* private fields */ }
```

_Defined in `lang/std/collections/views.ks`._

### Members

#### initializer `init`

```kestrel
public init(ptr: Pointer[T], remaining: Int64, predicate: (T) -> Bool, done: Bool)
```

_Defined in `lang/std/collections/views.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = ArraySlice[T]
```

_Defined in `lang/std/collections/views.ks`._

#### function `next`

```kestrel
public mutating func next() -> Optional[ArraySlice[T]]
```

_Defined in `lang/std/collections/views.ks`._

## struct `ArraySplitWhereView`

```kestrel
public struct ArraySplitWhereView[T] { /* private fields */ }
```

Multi-pass lazy view over the segments produced by splitting on each
element matching a predicate. No `Equatable` requirement.
(Named `ArraySplitWhereView` to avoid collision with
`std.text.SplitWhereView`.)

_Defined in `lang/std/collections/views.ks`._

### Members

#### field `count`

```kestrel
public var count: Int64 { get }
```

_Defined in `lang/std/collections/views.ks`._

#### initializer `init`

```kestrel
public init(slice: ArraySlice[T], predicate: (T) -> Bool)
```

_Defined in `lang/std/collections/views.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

_Defined in `lang/std/collections/views.ks`._

#### function `toArray`

```kestrel
public func toArray() -> Array[ArraySlice[T]]
```

_Defined in `lang/std/collections/views.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = ArraySlice[T]
```

_Defined in `lang/std/collections/views.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = ArraySplitWhereIterator[T]
```

_Defined in `lang/std/collections/views.ks`._

#### function `iter`

```kestrel
public func iter() -> ArraySplitWhereIterator[T]
```

_Defined in `lang/std/collections/views.ks`._

## typealias `ArrayTypeOperator`

```kestrel
public type ArrayTypeOperator[T] = Array[T]
```

Compiler-recognized type alias that lets `[T]` desugar to `Array[T]`.

Allows annotations like `let xs: [Int64] = [1, 2, 3]` instead of
requiring the user to spell out `Array[Int64]`. Not intended for
direct use — the parser inserts it automatically when it sees the
`[T]` shorthand in a type position.

### Examples

```
let xs: [Int64] = [1, 2, 3];   // same as: Array[Int64]
func sum(of values: [Float]) -> Float { ... }
```

_Defined in `lang/std/collections/array.ks`._

## struct `ChunksIterator`

```kestrel
public struct ChunksIterator[T] { /* private fields */ }
```

_Defined in `lang/std/collections/views.ks`._

### Members

#### initializer `init`

```kestrel
public init(ptr: Pointer[T], remaining: Int64, chunkSize: Int64)
```

_Defined in `lang/std/collections/views.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = ArraySlice[T]
```

_Defined in `lang/std/collections/views.ks`._

#### function `next`

```kestrel
public mutating func next() -> Optional[ArraySlice[T]]
```

_Defined in `lang/std/collections/views.ks`._

## struct `ChunksView`

```kestrel
public struct ChunksView[T] { /* private fields */ }
```

Multi-pass lazy view over non-overlapping `chunkSize`-sized
`ArraySlice[T]` segments.

_Defined in `lang/std/collections/views.ks`._

### Members

#### field `count`

```kestrel
public var count: Int64 { get }
```

_Defined in `lang/std/collections/views.ks`._

#### field `first`

```kestrel
public var first: Optional[ArraySlice[T]] { get }
```

_Defined in `lang/std/collections/views.ks`._

#### initializer `init`

```kestrel
public init(slice: ArraySlice[T], chunkSize: Int64)
```

_Defined in `lang/std/collections/views.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

_Defined in `lang/std/collections/views.ks`._

#### field `last`

```kestrel
public var last: Optional[ArraySlice[T]] { get }
```

_Defined in `lang/std/collections/views.ks`._

#### subscript `subscript`

```kestrel
public subscript(Int64) -> ArraySlice[T] { get }
```

_Defined in `lang/std/collections/views.ks`._

#### function `toArray`

```kestrel
public func toArray() -> Array[ArraySlice[T]]
```

_Defined in `lang/std/collections/views.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = ArraySlice[T]
```

_Defined in `lang/std/collections/views.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = ChunksIterator[T]
```

_Defined in `lang/std/collections/views.ks`._

#### function `iter`

```kestrel
public func iter() -> ChunksIterator[T]
```

_Defined in `lang/std/collections/views.ks`._

## struct `DefaultHasher`

```kestrel
public struct DefaultHasher { /* private fields */ }
```

The standard `Hasher` implementation, backed by a wyhash-derived
per-byte mixer.

Used by `Dictionary` and `Set` whenever the user doesn't pick a
specific hasher. Each byte folds into a 64-bit running state via
`state = (state ^ byte) * MULT`; `finish()` runs Murmur3's fmix64
finalizer to scramble the result so every input bit avalanches
across the output.

**Not adversarially safe.** The mixer is unkeyed, so an attacker
who can choose keys can craft collisions. For HashDoS resistance,
swap in a keyed hasher (planned: `SipHasher13`) by spelling out
`Dictionary[K, V, SipHasher13]` directly. For non-adversarial
workloads — internal IDs, parser symbols, config values — this
hasher is faster and has better distribution than FNV-1a.

### Examples

```
var h = DefaultHasher();
"hello".hash(into: h);
let hash = h.finish();  // 64-bit hash of "hello"

// Used implicitly through the dictionary type alias:
let d: [String: Int64] = ["a": 1];   // DefaultHasher under the hood
```

### Algorithm

Initialization seeds `state` with the wyhash secret
`0x9e3779b97f4a7c15` (the "golden ratio" constant SplitMix64 uses).
Each byte updates the state with `state = (state ^ byte) *
0x100000001b3`, which combines wyhash's mixing constant with
FNV-1a's prime so every bit of the byte propagates across the
64-bit state. `finish()` runs Murmur3's `fmix64` finalizer
(xor-shift-multiply twice) so consecutive integer keys produce
non-clustered hashes.

### Representation

One `UInt64` field, `state`, holding the running digest.

_Defined in `lang/std/collections/hashing.ks`._

### Members

#### initializer `Empty`

```kestrel
public init()
```

Creates a fresh hasher seeded with the SplitMix64 golden-ratio
constant `0x9e3779b97f4a7c15`.

The same input fed to two new hashers always produces the same
`finish()` value — this hasher is deterministic across runs (no
random seeding).

_Defined in `lang/std/collections/hashing.ks`._

### Implements `Hasher`

#### function `finish`

```kestrel
public mutating func finish() -> UInt64
```

Returns the finalized 64-bit digest.

Runs Murmur3's `fmix64` finalizer over the running state — two
rounds of xor-shift-multiply that avalanche every input bit
across the output. Consecutive integer keys (a common bucket
query pattern) emerge well-distributed despite the simple
mixer, which would otherwise leak the input's low-bit
regularity into the bucket index.

`finish()` mutates `state`; calling it twice on the same hasher
is undefined — construct a fresh `DefaultHasher()` per logical
hash.

_Defined in `lang/std/collections/hashing.ks`._

#### function `write`

```kestrel
public mutating func write(ArraySlice[UInt8])
```

Folds every byte of `bytes` into the running hash state.

May be called any number of times before `finish()`; the result
is identical to having received all the bytes in a single call.
Safe to call with an empty slice (no-op).

##### Examples

```
var h = DefaultHasher();
h.write(bytes: "hello".utf8Bytes());
h.write(bytes: " world".utf8Bytes());
// Equivalent to a single write of "hello world".utf8Bytes()
```

_Defined in `lang/std/collections/hashing.ks`._

### Implements `Defaultable`

#### initializer `Default`

```kestrel
init()
```

Builds the default-valued instance.

_Defined in `lang/std/core/protocols.ks`._

## struct `Deque`

```kestrel
public struct Deque[T] { /* private fields */ }
```

A double-ended queue backed by a ring buffer with copy-on-write storage.

O(1) amortized `pushBack`/`pushFront`/`popBack`/`popFront` and O(1)
random access by index. Storage is shared between copies until one
mutates, at which point the COW barrier fires.

### Examples

```
var d = Deque[Int64]();
d.pushBack(1);
d.pushFront(0);
d.pushBack(2);
d.popFront();  // .Some(0)
d.popBack();   // .Some(2)
```

### Representation

Holds a `CowBox[DequeStorage[T]]`. The storage is a `(ptr, len, cap,
head)` quad over a heap-allocated ring buffer.

### Memory Model

Reference-counted storage with copy-on-write value semantics via
`CowBox`. Copying a `Deque` is O(1); the first mutation on a shared
copy triggers a deep clone that linearizes the ring buffer.

### Guarantees

- `pushBack`/`pushFront` are O(1) amortized; growth is geometric.
- `popBack`/`popFront` are O(1).
- Subscript access is O(1).
- Iteration order is front-to-back.

_Defined in `lang/std/collections/deque.ks`._

### Members

#### initializer `Empty`

```kestrel
public init()
```

Creates an empty deque with no allocation.

No heap memory is allocated until the first `pushBack` or
`pushFront`. Use `Deque(capacity:)` to pre-allocate when the
expected size is known.

##### Examples

```
var d = Deque[Int64]();
d.isEmpty;  // true
```

_Defined in `lang/std/collections/deque.ks`._

#### initializer `From Iterable`

```kestrel
public init[I](from: I) where I: Iterable, I.Item == T
```

Creates a deque by collecting every element from an iterable.

Elements are appended via `pushBack`, so iteration order of the
source is preserved as front-to-back order in the deque.

##### Examples

```
let d = Deque[Int64](from: 1..<5);
d.count;  // 4
d.first();  // .Some(1)
d.last();   // .Some(4)
```

_Defined in `lang/std/collections/deque.ks`._

#### subscript `Indexed`

```kestrel
subscript(Int64) -> T { get set }
```

O(1) random access by logical index.

Logical index 0 is the front element, `count - 1` is the back.
The ring-buffer offset is computed internally. Both get and set
are O(1).

##### Errors

Panics with `"Deque: index out of bounds"` when `index < 0` or
`index >= count`.

##### Examples

```
var d = Deque[Int64](from: [10, 20, 30]);
d(0);  // 10
d(2);  // 30
d(1) = 99;
d(1);  // 99
```

_Defined in `lang/std/collections/deque.ks`._

#### initializer `With Capacity`

```kestrel
public init(capacity: Int64)
```

Creates an empty deque with at least `capacity` slots reserved.

Allocates a ring buffer that can hold `capacity` elements before
needing to grow. Passing 0 is equivalent to `Deque()`.

##### Examples

```
var d = Deque[Int64](capacity: 100);
d.count;  // 0
d.capacity;  // 100
```

_Defined in `lang/std/collections/deque.ks`._

#### function `asSlices`

```kestrel
public func asSlices() -> (ArraySlice[T], ArraySlice[T])
```

Returns the two contiguous slices that make up the ring buffer.

If the buffer doesn't wrap, the first slice contains all elements
and the second is empty. If it wraps, the first slice covers
head-to-end and the second covers start-to-tail. Useful for
bulk operations that need pointer-contiguous access without
copying.

##### Examples

```
var d = Deque[Int64](from: [1, 2, 3]);
let (a, b) = d.asSlices();
// non-wrapping: a contains all 3 elements, b is empty
```

_Defined in `lang/std/collections/deque.ks`._

#### field `capacity`

```kestrel
public var capacity: Int64 { get }
```

Number of elements the buffer can hold without reallocating.

The deque automatically grows when `count` exceeds `capacity`,
so this is mainly useful for pre-sizing via `reserveCapacity()`.

##### Examples

```
var d = Deque[Int64](capacity: 16);
d.capacity;  // 16
```

_Defined in `lang/std/collections/deque.ks`._

#### function `clear`

```kestrel
public mutating func clear()
```

Removes all elements, retaining allocated capacity.

After calling `clear()`, `count` is 0 and `head` resets to 0, but
the buffer stays allocated so subsequent pushes avoid reallocation.

##### Examples

```
var d = Deque[Int64](from: [1, 2, 3]);
d.clear();
d.isEmpty;  // true
```

_Defined in `lang/std/collections/deque.ks`._

#### field `count`

```kestrel
public var count: Int64 { get }
```

Number of elements in the deque.

##### Examples

```
let d = Deque[Int64](from: [1, 2, 3]);
d.count;  // 3
```

_Defined in `lang/std/collections/deque.ks`._

#### function `first`

```kestrel
public func first() -> T?
```

Returns the front element without removing it, or `.None` if empty.

O(1). The removing counterpart is `popFront()`.

##### Examples

```
let d = Deque[Int64](from: [10, 20, 30]);
d.first();  // .Some(10)
Deque[Int64]().first();  // .None
```

_Defined in `lang/std/collections/deque.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

True when the deque contains no elements.

##### Examples

```
Deque[Int64]().isEmpty;  // true
Deque[Int64](from: [1]).isEmpty;  // false
```

_Defined in `lang/std/collections/deque.ks`._

#### function `last`

```kestrel
public func last() -> T?
```

Returns the back element without removing it, or `.None` if empty.

O(1). The removing counterpart is `popBack()`.

##### Examples

```
let d = Deque[Int64](from: [10, 20, 30]);
d.last();  // .Some(30)
Deque[Int64]().last();  // .None
```

_Defined in `lang/std/collections/deque.ks`._

#### function `popBack`

```kestrel
public mutating func popBack() -> T?
```

Removes and returns the back element, or `.None` if empty. O(1).

Retracts the logical tail by one slot. The non-removing mirror
is `last()`. The front-end counterpart is `popFront()`.

##### Examples

```
var d = Deque[Int64](from: [1, 2, 3]);
d.popBack();  // .Some(3)
d.popBack();  // .Some(2)
d.popBack();  // .Some(1)
d.popBack();  // .None
```

_Defined in `lang/std/collections/deque.ks`._

#### function `popFront`

```kestrel
public mutating func popFront() -> T?
```

Removes and returns the front element, or `.None` if empty. O(1).

Advances the ring-buffer head by one slot. The non-removing mirror
is `first()`. The back-end counterpart is `popBack()`.

##### Examples

```
var d = Deque[Int64](from: [1, 2, 3]);
d.popFront();  // .Some(1)
d.popFront();  // .Some(2)
d.popFront();  // .Some(3)
d.popFront();  // .None
```

_Defined in `lang/std/collections/deque.ks`._

#### function `pushBack`

```kestrel
public mutating func pushBack(T)
```

Appends `element` to the back of the deque. O(1) amortized.

Grows the ring buffer geometrically when full. The counterpart
for the front end is `pushFront`.

##### Examples

```
var d = Deque[Int64]();
d.pushBack(1);
d.pushBack(2);
d.popFront();  // .Some(1)
```

_Defined in `lang/std/collections/deque.ks`._

#### function `pushFront`

```kestrel
public mutating func pushFront(T)
```

Prepends `element` to the front of the deque. O(1) amortized.

Grows the ring buffer geometrically when full. The counterpart
for the back end is `pushBack`.

##### Examples

```
var d = Deque[Int64]();
d.pushFront(1);
d.pushFront(0);
d.popFront();  // .Some(0)
```

_Defined in `lang/std/collections/deque.ks`._

#### function `reserveCapacity`

```kestrel
public mutating func reserveCapacity(minimumCapacity: Int64)
```

Ensures the buffer can hold at least `capacity` elements without
reallocating.

If the current capacity already meets or exceeds `capacity`, this
is a no-op. Otherwise the ring buffer is reallocated and
linearized. Useful before a burst of `pushBack`/`pushFront` calls
when the final size is known in advance.

##### Examples

```
var d = Deque[Int64]();
d.reserveCapacity(minimumCapacity: 100);
```

_Defined in `lang/std/collections/deque.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = T
```

`Iterable` element type.

_Defined in `lang/std/collections/deque.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = DequeIterator[T]
```

`Iterable` iterator type.

_Defined in `lang/std/collections/deque.ks`._

#### function `iter`

```kestrel
public func iter() -> DequeIterator[T]
```

Returns an iterator that yields elements front-to-back.

The iterator walks the ring buffer from `head` through `count`
elements, wrapping around the buffer boundary transparently.

##### Examples

```
var d = Deque[Int64](from: [10, 20, 30]);
for x in d {
    // yields 10, 20, 30
}
```

_Defined in `lang/std/collections/deque.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> Deque[T]
```

Returns a shallow copy of this deque. O(1).

The `CowBox` storage is shared until either copy mutates, at
which point the COW barrier fires a deep clone that linearizes
the ring buffer.

##### Examples

```
let d = Deque[Int64](from: [1, 2, 3]);
var d2 = d.clone();
d2.pushBack(4);
d.count;   // 3 -- original unchanged
d2.count;  // 4
```

_Defined in `lang/std/collections/deque.ks`._

## struct `DequeIterator`

```kestrel
public struct DequeIterator[T] { /* private fields */ }
```

Iterator over a `Deque[T]`, walking the ring buffer from head through
`remaining` elements.

Created by `Deque.iter()`. Yields elements front-to-back, wrapping
around the ring buffer boundary transparently.

### Representation

Holds a raw pointer into the deque's ring buffer, the buffer
capacity, the current physical position, and a remaining-element
count. Does not own the storage.

_Defined in `lang/std/collections/deque.ks`._

### Members

#### initializer `From Fields`

```kestrel
public init(ptr: Pointer[T], cap: Int64, pos: Int64, remaining: Int64)
```

Constructs an iterator from the ring buffer's raw state.

Called internally by `Deque.iter()`.

_Defined in `lang/std/collections/deque.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = T
```

`Iterator` element type.

_Defined in `lang/std/collections/deque.ks`._

#### function `next`

```kestrel
public mutating func next() -> T?
```

Advances the iterator and returns the next element, or `.None`
when all elements have been consumed.

Handles the ring-buffer wrap-around: when `pos` reaches `cap` it
resets to 0.

##### Examples

```
var d = Deque[Int64](from: [10, 20]);
var it = d.iter();
it.next();  // .Some(10)
it.next();  // .Some(20)
it.next();  // .None
```

_Defined in `lang/std/collections/deque.ks`._

## struct `Dictionary`

```kestrel
public struct Dictionary[K, V, H = DefaultHasher] where K: Hashable, H: Hasher, H: Defaultable { /* private fields */ }
```

An unordered hash map keyed by any `K: Hashable`, parameterized over the
hasher type `H` (defaults to `DefaultHasher`).

Uses open addressing with linear probing and a 75% load-factor
threshold for resizes; capacity always grows to the next power of
two. Storage is reference-counted with copy-on-write, so copying a
`Dictionary` is O(1) and only the next mutation pays for the deep
clone. Iteration order is unspecified and may change between
versions or after any mutation. For ordered alternatives consider
keeping an ordered key list separately; for set-only behavior see
`Set`.

### Examples

```
var ages: [String: Int64] = [:];
ages("Alice") = 30;
ages("Bob")   = 25;

ages("Alice");                // Some(30)
ages("Carol", default: 0);    // 0

for (name, age) in ages.iter() { ... }
let sum = ages.values.iter().sum();
```

### Hashing

The hash for each key is cached in its bucket so resizes don't
recompute it. Replacing the hasher (`H`) lets you swap in
`SipHasher`, `FxHasher`, etc.; the default is `DefaultHasher` and
resolves through the `[K: V]` shorthand.

### Capacity & Reallocation

`count` is live entries; `capacity` is total slots. The table
resizes (doubling capacity, starting from 8) once `count` reaches
75% of `capacity`. Use `reserveCapacity(...)` to pre-grow and
`shrinkToFit()` to release excess.

### Representation

One field: an `RcBox[DictionaryStorage[K, V, H]]` holding
`(buckets, len, cap)` over a heap bucket array.

### Memory Model

Reference-counted storage with copy-on-write *value* semantics.
Copying a `Dictionary` is O(1) and shares the bucket array; the
next mutation on a shared dictionary triggers `makeUnique()`,
which deep-clones via `DictionaryStorage.clone()` so the mutation
is invisible to other copies.

### Guarantees

- Every key satisfies `K: Hashable`. The cached hash is computed once
  per insert and reused on resize.
- `count <= capacity * 3 / 4` after every mutation (the resize
  threshold).
- Removing a key leaves a `.Deleted` tombstone; lookups still
  work but tombstones reduce effective capacity until the next
  resize.
- Iteration order is **not** specified.

_Defined in `lang/std/collections/dictionary.ks`._

### Members

#### initializer `Dictionary Literal`

```kestrel
public init(dictionaryLiteral: std.memory.LiteralSlice[(K, V)])
```

Creates a dictionary by inserting every `(K, V)` pair from a
literal slice in order.

Last-write-wins on duplicate keys (same as `init(from:)`). An
empty literal yields an empty unallocated dictionary.

##### Examples

```
// Triggered by the dictionary-literal syntax:
let dict: [String: Int64] = ["a": 1, "b": 2];
```

_Defined in `lang/std/collections/dictionary.ks`._

#### initializer `Empty`

```kestrel
public init()
```

Creates an empty dictionary with no allocation.

Capacity starts at zero; the first insert allocates the smallest
bucket array (currently 8 slots). For pre-sized creation use
`init(capacity:)`.

##### Examples

```
var d = Dictionary[String, Int64]();
d.count;     // 0
d.capacity;  // 0
```

_Defined in `lang/std/collections/dictionary.ks`._

#### initializer `From Pairs`

```kestrel
public init[I](from: I) where I: Iterable, I.Item == (K, V)
```

Creates a dictionary by inserting every `(key, value)` pair
produced by an iterable.

Last write wins for duplicate keys. For a panic-on-duplicate
variant use `init(uniquePairs:)`. Capacity grows
geometrically as inserts arrive — for sized sources, follow up
with `shrinkToFit()` if memory matters.

##### Examples

```
let pairs = [("a", 1), ("b", 2)];
let dict = Dictionary(from: pairs);              // ["a": 1, "b": 2]
let dups = Dictionary(from: [("a", 1), ("a", 2)]);  // ["a": 2] — later wins
```

_Defined in `lang/std/collections/dictionary.ks`._

#### initializer `Grouping`

```kestrel
public init[I, E](grouping: I, by: (E) -> K) where I: Iterable, I.Item == E, V == Array[E]
```

Buckets each element of an iterable into an array under the key
derived from `keyFunc`.

The value type is constrained to `Array[E]`: each bucket
accumulates the elements that mapped to it, in insertion order
within that bucket. Useful for building "index-by" tables from a
flat collection. The `keyFunc` runs once per element.

##### Examples

```
let words = ["apple", "apricot", "banana", "blueberry"];
let grouped = Dictionary(grouping: words) { (w) in w.chars.first().unwrap() };
// ["a": ["apple", "apricot"], "b": ["banana", "blueberry"]]

let nums = [1, 2, 3, 4, 5];
let parity = Dictionary(grouping: nums) { (n) in n % 2 };
// [0: [2, 4], 1: [1, 3, 5]]
```

_Defined in `lang/std/collections/dictionary.ks`._

#### initializer `Literal Bridge`

```kestrel
public init(consuming lang.ptr[(K, V)], consuming lang.i64)
```

Compiler-emitted bridge for `[k: v, ...]` literals.

Not called by user code directly — the parser lowers literal
expressions into a `(ptr, count)` pair which this constructor
wraps in a `LiteralSlice` and forwards to
`init(dictionaryLiteral:)`.

##### Safety

The compiler guarantees `_dictionaryLiteralPointer` points to
exactly `_dictionaryLiteralCount` initialized `(K, V)` pairs.

_Defined in `lang/std/collections/dictionary.ks`._

#### subscript `Lookup`

```kestrel
public subscript(K) -> V? { get set }
```

Reads the value for `key` (or `None` if absent), or assigns
to insert/remove the entry.

The assignment form treats `Some(v)` as insert/update and
`None` as delete — so `dict(k) = None` is the inline form of
`dict.remove(k)`. For a non-`Optional` getter use
`dict(key, default: ...)` or `dict(unwrap: key)`.

##### Examples

```
var dict = ["a": 1, "b": 2];
dict("a");           // Some(1)
dict("z");           // None
dict("c") = 3;       // inserts "c": 3
dict("a") = None;    // removes "a"
```

_Defined in `lang/std/collections/dictionary.ks`._

#### initializer `Unique Keys`

```kestrel
public init[I](uniquePairs: I) where I: Iterable, I.Item == (K, V)
```

Creates a dictionary from key-value pairs, panicking on any
duplicate key.

Use this when duplicate keys would indicate a bug in upstream
data; for last-write-wins semantics use `init(from:)`. Each pair
triggers a `contains` check before insertion, so it's slower
than `init(from:)` for large inputs.

##### Errors

Panics with `"Dictionary(uniquePairs:): duplicate key"`
the first time `pairs` yields a key already in the dictionary.

##### Examples

```
let dict = Dictionary(uniquePairs: [("a", 1), ("b", 2)]);
Dictionary(uniquePairs: [("a", 1), ("a", 2)]);
// PANIC: Dictionary(uniquePairs:): duplicate key
```

_Defined in `lang/std/collections/dictionary.ks`._

#### subscript `Unwrap`

```kestrel
public subscript(unwrap: K) -> V { get set }
```

Reads or writes the value for `key`, panicking on the read
when the key is absent.

Use when you've already verified the key exists (or when its
absence indicates a bug). The setter is equivalent to
`insert(key, newValue)` and never panics. For a non-panicking
read use `dict(key)` or `dict(key, default: ...)`.

##### Errors

Read panics with
`"Dictionary subscript(unwrap:): key not found"` when the key
is absent.

##### Examples

```
let dict = ["a": 1, "b": 2];
dict(unwrap: "a");  // 1
dict(unwrap: "z");  // PANIC: key not found
```

_Defined in `lang/std/collections/dictionary.ks`._

#### initializer `With Capacity`

```kestrel
public init(capacity: Int64)
```

Creates an empty dictionary sized to hold at least the requested
number of entries without resizing.

The actual allocated capacity is the next power of two `>= capacity`
(minimum 8). A non-positive `capacity` behaves like `init()` (no
allocation). Panics on allocation failure.

##### Examples

```
var d = Dictionary[String, Int64](capacity: 100);
d.capacity;   // 128 (next power of two)
d.count;      // 0
```

_Defined in `lang/std/collections/dictionary.ks`._

#### subscript `With Default`

```kestrel
public subscript(K, default: V) -> V { get }
```

Reads the value for `key`, falling back to `defaultValue` when
the key is absent.

Read-only and *non-inserting* — the default value is returned
but never stored. To upsert with a default, use `upsert(...)`
or `update(...)`.

##### Examples

```
let dict = ["a": 1, "b": 2];
dict("a", default: 0);  // 1
dict("z", default: 0);  // 0
dict("z");              // still None — default wasn't stored
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `all`

```kestrel
public func all(where: (K, V) -> Bool) -> Bool
```

`true` when every entry satisfies `predicate(key, value)`
(vacuously true for empty).

Short-circuits on the first failure. Dual of `any(where:)`.

##### Examples

```
["a": 2, "b": 4].all { (k, v) in v % 2 == 0 };  // true
["a": 1, "b": 2].all { (k, v) in v % 2 == 0 };  // false
[:].all { (k, v) in false };                    // true (vacuous)
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `allKeys`

```kestrel
public func allKeys(of: V) -> Array[K]
```

Returns every key whose value equals `value`.

O(capacity), allocates an `Array[K]`. Result order matches
bucket layout and is therefore unspecified. Empty array if no
matches.

##### Examples

```
["a": 1, "b": 2, "c": 1].allKeys(of: 1);  // ["a", "c"]  — order unspecified
["a": 1].allKeys(of: 99);                  // []
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `any`

```kestrel
public func any(where: (K, V) -> Bool) -> Bool
```

`true` when at least one entry satisfies `predicate(key, value)`.

Alias for `contains(where:)` — the two names exist so
predicate-style code reads naturally regardless of context.
Short-circuits on the first match.

##### Examples

```
["a": 1, "b": 5].any { (k, v) in v > 3 };  // true
[:].any { (k, v) in true };                // false (empty)
```

_Defined in `lang/std/collections/dictionary.ks`._

#### field `capacity`

```kestrel
public var capacity: Int64 { get }
```

Total slots in the bucket array — always `>= count`. Read-only.

Resizes (doubling) trigger when `count` reaches 75% of
`capacity`. Tombstones count against the threshold even though
they don't count toward `count`. The actual value after
`init(capacity:)` rounds up to the next power of two.

##### Examples

```
let d = Dictionary[String, Int64](capacity: 100);
d.capacity;  // 128
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `clear`

```kestrel
public mutating func clear()
```

Removes every entry, leaving the bucket array allocated and
reset to all-`.Empty`.

O(capacity). The buffer is kept so subsequent inserts don't
reallocate; follow with `shrinkToFit()` to release it.

##### Examples

```
var dict = ["a": 1, "b": 2];
dict.clear();    // dict = [:]
dict.capacity;   // unchanged
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `compactMapValues`

```kestrel
public func compactMapValues[U]((V) -> U?) -> Dictionary[K, U, H]
```

Returns a new dictionary with each value run through `transform`;
entries whose `transform(value)` is `None` are dropped.

Useful for parse-or-skip patterns. The result is unsized at
construction (since the final count isn't known until the
pass completes); for fixed transforms that always succeed,
`mapValues(...)` avoids the allocation policy difference.

##### Examples

```
let dict = ["a": "1", "b": "two", "c": "3"];
let parsed = dict.compactMapValues { (s) in Int64.parse(s) };
// ["a": 1, "c": 3] — "two" failed to parse
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `contains`

```kestrel
public func contains(K) -> Bool
```

`true` if `key` is present in the dictionary.

Wraps `findEntry`. For value-based search use the `V: Equatable`
extension's `containsValue(value:)`.

##### Examples

```
["a": 1, "b": 2].contains("a");  // true
["a": 1, "b": 2].contains("z");  // false
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `contains`

```kestrel
public func contains(where: (K, V) -> Bool) -> Bool
```

`true` if any entry satisfies `predicate(key, value)`.

Linear scan; short-circuits on the first match. `false` for
empty dictionaries. The aliased shape `any(satisfy:)` exists
for symmetry with `Array`.

##### Examples

```
["a": 1, "b": 5].contains { (k, v) in v > 3 };  // true
["a": 1, "b": 2].contains { (k, v) in v > 3 };  // false
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `containsValue`

```kestrel
public func containsValue(V) -> Bool
```

`true` if any entry's value equals `value`.

O(capacity) — every bucket is inspected because the dictionary
is keyed on `K`, not `V`. For `O(1)` checks against a small
set of values, build a `Set[V]` instead.

##### Examples

```
["a": 1, "b": 2].containsValue(2);  // true
["a": 1, "b": 2].containsValue(5);  // false
```

_Defined in `lang/std/collections/dictionary.ks`._

#### field `count`

```kestrel
public var count: Int64 { get }
```

Number of live (`.Occupied`) entries. Read-only; O(1).

Excludes tombstones — `count` only reflects what
`iter()`/`contains(...)` would see.

##### Examples

```
["a": 1, "b": 2].count;  // 2
[:].count;               // 0
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `countItems`

```kestrel
public func countItems(where: (K, V) -> Bool) -> Int64
```

Returns the number of entries for which
`predicate(key, value)` is true.

Linear scan, no short-circuit. For just a presence check use
`any(where:)`; for a yes/no on every entry,
`all(where:)`.

##### Examples

```
["a": 1, "b": 2, "c": 3].countItems { (k, v) in v > 1 };  // 2
[:].countItems { (k, v) in true };                        // 0
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `deepClone`

```kestrel
public func deepClone() -> Dictionary[K, V, H]
```

Returns a fully-detached copy of the dictionary, with no shared
storage.

Walks every bucket and re-inserts the live entries into a
freshly-sized table. Use over `clone()` when you specifically
want to avoid the lazy COW share — for example, before passing
the copy to another thread or system that might race with
further mutations.

##### Examples

```
let a = ["x": [1, 2, 3]];
let b = a.deepClone();  // fully independent copy
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `filter`

```kestrel
public func filter(where: (K, V) -> Bool) -> Dictionary[K, V, H]
```

Returns a new dictionary containing only entries for which
`predicate(key, value)` is true.

Non-mutating mirror of `retain(where:)`. Allocates a fresh
dictionary; for in-place filtering use `retain` or
`removeAll(where:)`.

##### Examples

```
let dict = ["a": 1, "b": 2, "c": 3];
let big = dict.filter { (k, v) in v > 1 };  // ["b": 2, "c": 3]
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `first`

```kestrel
public func first(where: (K, V) -> Bool) -> (K, V)?
```

Returns *some* entry matching `predicate(key, value)`, or
`None`.

"First" is determined by bucket order, which is hash-dependent
and unspecified — treat the result as arbitrary among matching
entries. Short-circuits on the first match.

##### Examples

```
let dict = ["a": 1, "b": 5, "c": 3];
dict.first { (k, v) in v > 2 };  // Some entry with v > 2
dict.first { (k, v) in v > 99 }; // None
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `firstKey`

```kestrel
public func firstKey(of: V) -> K?
```

Returns *some* key mapping to `value`, or `None`.

O(capacity); short-circuits on the first match. "First" is
determined by bucket order and is unspecified — for an
exhaustive list use `allKeys(of:)`.

##### Examples

```
["a": 1, "b": 2].firstKey(of: 2);  // Some("b")
["a": 1, "b": 2].firstKey(of: 5);  // None
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `insert`

```kestrel
public mutating func insert(K, V) -> V?
```

Inserts `(key, value)`, replacing any existing entry for `key`,
and returns the old value (or `None`) on update.

Triggers `ensureCapacity()` and may resize before the insert
lands. The cached hash is computed once here. For
transform-based updates see `update(...)` and `upsert(...)`.

##### Examples

```
var dict = ["a": 1];
dict.insert("b", 2);  // None;     dict = ["a": 1, "b": 2]
dict.insert("a", 9);  // Some(1);  dict = ["a": 9, "b": 2]
```

_Defined in `lang/std/collections/dictionary.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

`true` when the dictionary holds no live entries; equivalent to
`count == 0`.

Reads more naturally than the comparison.

##### Examples

```
[:].isEmpty;           // true
["a": 1].isEmpty;      // false
```

_Defined in `lang/std/collections/dictionary.ks`._

#### field `keys`

```kestrel
public var keys: KeysView[K, V] { get }
```

Lazy view of the dictionary's keys, iterable in unspecified
order.

Constructing the view is O(1) — it shares the bucket pointer
and skips empty/deleted slots during iteration. The view is
invalidated by any mutation that may reallocate (insertion past
the load threshold, `reserveCapacity`, `shrinkToFit`).

##### Examples

```
let dict = ["a": 1, "b": 2, "c": 3];
for key in dict.keys { print(key) }
let keyArray = Array(from: dict.keys);
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `mapValues`

```kestrel
public func mapValues[U]((V) -> U) -> Dictionary[K, U, H]
```

Returns a new dictionary with each value run through `transform`,
keys unchanged.

Pre-sized to `self.capacity` so the first build avoids
resizing. The result's value type can change (`V → U`); for a
version that drops `None` results see `compactMapValues(...)`.

##### Examples

```
let dict = ["a": 1, "b": 2];
let doubled = dict.mapValues { (v) in v * 2 };
// ["a": 2, "b": 4]
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `merge`

```kestrel
public mutating func merge(Dictionary[K, V, H], uniquingKeysWith: (V, V) -> V)
```

Merges every entry of `other` into `self`, calling `combine`
to resolve key collisions.

`combine(existing, incoming)` is invoked exactly once per
collision — pick one, return both summed, or use `(_, new)` for
last-write-wins. New keys are inserted directly. For a
non-mutating variant use `merging(...)`.

##### Examples

```
var a = ["x": 1, "y": 2];
let b = ["y": 20, "z": 30];
a.merge(b) { (old, new) in old + new };
// a == ["x": 1, "y": 22, "z": 30]
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `merge`

```kestrel
public mutating func merge[I](from: I, uniquingKeysWith: (V, V) -> V) where I: Iterable, I.Item == (K, V)
```

Merges every `(key, value)` pair from an arbitrary iterable into
`self`, calling `combine` on collisions.

Same semantics as `merge(...)` but accepts any iterable of
pairs — useful for arrays of tuples, generator output, or
streamed sources.

##### Examples

```
var dict = ["a": 1];
dict.merge(from: [("b", 2), ("c", 3)]) { (_, new) in new };
// dict == ["a": 1, "b": 2, "c": 3]
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `merging`

```kestrel
public func merging(Dictionary[K, V, H], uniquingKeysWith: (V, V) -> V) -> Dictionary[K, V, H]
```

Returns a new dictionary that is `self` merged with `other`,
resolving collisions via `combine`.

Non-mutating mirror of `merge(...)`. Internally clones via COW
(cheap until the next mutation) and merges into the copy.

##### Examples

```
let a = ["x": 1, "y": 2];
let b = ["y": 20, "z": 30];
let merged = a.merging(b) { (_, new) in new };
// merged == ["x": 1, "y": 20, "z": 30]
// a is unchanged
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `remove`

```kestrel
public mutating func remove(K) -> V?
```

Removes `key` and returns its value, or `None` if absent.

Replaces the bucket with a `.Deleted` tombstone so existing
probe chains stay intact. Tombstones are reclaimed by the next
resize. Triggers COW only when an entry is actually removed.

##### Examples

```
var dict = ["a": 1, "b": 2];
dict.remove("a");  // Some(1); dict = ["b": 2]
dict.remove("z");  // None;    dict unchanged
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `removeAll`

```kestrel
public mutating func removeAll(where: (K, V) -> Bool)
```

Removes every entry for which `predicate(key, value)` is true.

Inverse of `retain(where:)`; implemented as `retain` over
the negated predicate. Same tombstone caveat applies — consider
`shrinkToFit()` after large removals.

##### Examples

```
var dict = ["a": 1, "b": 2, "c": 3];
dict.removeAll { (k, v) in v < 2 };  // ["b": 2, "c": 3]
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `reserveCapacity`

```kestrel
public mutating func reserveCapacity(Int64)
```

Grows the bucket array so at least `minimumCapacity` entries
fit without resizing.

No-op when current capacity already suffices. The actual new
capacity rounds up to the next power of two and accounts for
the 75% load factor (so target = `nextPowerOfTwo(min * 4 / 3)`).
The opposite operation is `shrinkToFit()`.

##### Examples

```
var dict = Dictionary[String, Int64]();
dict.reserveCapacity(1000);
// No reallocations for the first ~750 inserts.
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `retain`

```kestrel
public mutating func retain(where: (K, V) -> Bool)
```

Keeps only entries for which `predicate(key, value)` is true.

Two-pass implementation: collects keys to remove, then deletes
them. Each removal leaves a tombstone — call `shrinkToFit()`
afterwards if you've removed a large fraction. The mirror is
`removeAll(where:)`.

##### Examples

```
var dict = ["a": 1, "b": 2, "c": 3];
dict.retain { (k, v) in v > 1 };  // ["b": 2, "c": 3]
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `shrinkToFit`

```kestrel
public mutating func shrinkToFit()
```

Reduces capacity to the smallest power of two that still
satisfies the load factor for the current `count`.

Frees excess memory and reclaims tombstone space (the resize
rebuilds the table without them). Empty dictionaries fall
through to `clear()`. No-op when the table is already at the
minimum acceptable capacity.

##### Examples

```
var dict = Dictionary[String, Int64](capacity: 1000);
dict("a") = 1;
dict.shrinkToFit();  // capacity drops from 1024 to 8
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `sumValues`

```kestrel
public func sumValues() -> V
```

Returns the sum of every value, starting from `V()` (the
default-constructed zero).

Empty dictionaries return `V()` — for `Int64` that's `0`, for
`String` that's `""`, etc. Linear in `count`.

##### Examples

```
["a": 1, "b": 2, "c": 3].sumValues();  // 6
[:].sumValues();                        // 0 — V's default
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `update`

```kestrel
public mutating func update(K, with: (V) -> V) -> Bool
```

Applies `transform` to the existing value for `key` and writes
the result back; returns whether the key was found.

No-op when the key is absent — for "update or insert" semantics
use `upsert(...)`. Internally re-uses `insert(...)`, so the
hash is recomputed.

##### Examples

```
var dict = ["a": 1, "b": 2];
dict.update("a") { (v) in v * 10 };  // true;  dict("a") == Some(10)
dict.update("z") { (v) in v * 10 };  // false; dict unchanged
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `upsert`

```kestrel
public mutating func upsert(K, default: V, with: (V) -> V)
```

Inserts `transform(defaultValue)` for a new key, or
`transform(existing)` for an existing one.

The classic "increment-or-set-to-1" pattern. `defaultValue` is
passed through `transform` even on the insert path, so the same
closure handles both branches uniformly. For the no-insert
variant see `update(...)`.

##### Examples

```
var counts: [String: Int64] = [:];
counts.upsert("apple", default: 0) { (n) in n + 1 };
counts.upsert("apple", default: 0) { (n) in n + 1 };
counts("apple");  // Some(2)
```

_Defined in `lang/std/collections/dictionary.ks`._

#### field `values`

```kestrel
public var values: ValuesView[K, V] { get }
```

Lazy view of the dictionary's values, iterable in unspecified
order.

Same iteration order as `keys` — the two views walk the
buckets in lockstep, so `zip(dict.keys, dict.values)` yields
pairs equivalent to `dict.iter()`. Invalidated by any
mutation that may reallocate.

##### Examples

```
let dict = ["a": 1, "b": 2, "c": 3];
for value in dict.values { print(value) }
let sum = dict.values.iter().sum();  // 6
```

_Defined in `lang/std/collections/dictionary.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = (K, V)
```

`Iterable` element type — a `(key, value)` tuple.

_Defined in `lang/std/collections/dictionary.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = DictionaryIterator[K, V]
```

Concrete iterator type returned by `iter()`.

_Defined in `lang/std/collections/dictionary.ks`._

#### function `iter`

```kestrel
public func iter() -> DictionaryIterator[K, V]
```

Returns a `DictionaryIterator[K, V]` over the live entries.

Order is unspecified and may change between mutations. The
iterator borrows the bucket array; do not mutate the
dictionary while iterating. For key- or value-only iteration,
use `keys.iter()` / `values.iter()`.

##### Examples

```
for (k, v) in dict.iter() { ... }
let entries = Array(from: dict.iter());
```

_Defined in `lang/std/collections/dictionary.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> Dictionary[K, V, H]
```

Returns a `Dictionary` sharing the same storage; the deep copy
is deferred until either side mutates.

O(1) — just bumps the storage `RcBox`'s refcount. The first
mutation on either side triggers `makeUnique()`, which
deep-clones via `DictionaryStorage.clone()`. For an immediate
deep copy use `deepClone()` (defined in the unconditional
extension below).

##### Examples

```
let a: [String: Int64] = ["x": 1];
var b = a.clone();  // O(1), shares storage
b("y") = 2;         // b deep-copies here; a is unchanged
```

_Defined in `lang/std/collections/dictionary.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Dictionary[K, V, H]) -> Bool
```

Order-independent equality: dictionaries are equal iff they have
the same `count` and every key in `self` is present in `other`
with an equal value.

Short-circuits on the first mismatch. Insertion order does not
matter — only the multiset of `(key, value)` pairs does.

##### Examples

```
["a": 1, "b": 2].isEqual(to: ["b": 2, "a": 1]);  // true
["a": 1].isEqual(to: ["a": 2]);                  // false
["a": 1].isEqual(to: [:]);                       // false
```

_Defined in `lang/std/collections/dictionary.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating StringBuilder, FormatOptions)
```

Renders the dictionary as `"{" + entries.joined(", ") + "}"`,
passing `options` to each key and value's `format`.

##### Examples

```
["a": 1, "b": 2].format();  // "{a: 1, b: 2}"  — order unspecified
Dictionary[String, Int64]().format();  // "{}"
"\{["a": 1, "b": 2]}";      // "{a: 1, b: 2}"  via interpolation
```

_Defined in `lang/std/collections/dictionary.ks`._

### Implements `_ExpressibleByDictionaryLiteral`

#### typealias `Key`

```kestrel
type Key = K
```

Key type for the literal protocol — matches `K`.

_Defined in `lang/std/collections/dictionary.ks`._

#### initializer `Literal Bridge`

```kestrel
init(consuming lang.ptr[(Key, Value)], consuming lang.i64)
```

Compiler-emitted init taking a raw `(Key, Value)` pointer and count.

Both params are `consuming` for the same reason as the array
bridge: the compiler hands ownership of the stack buffer to the
implementation. MIR lowering matches on the unwrapped param
shape, so an impl that deviates from this convention will be
skipped during literal lowering.

_Defined in `lang/std/core/literals.ks`._

#### typealias `Value`

```kestrel
type Value = V
```

Value type for the literal protocol — matches `V`.

_Defined in `lang/std/collections/dictionary.ks`._

### Implements `ExpressibleByDictionaryLiteral`

#### initializer `Dictionary Literal`

```kestrel
init(dictionaryLiteral: LiteralSlice[(Key, Value)])
```

Builds an instance from a literal slice of key-value pairs.

_Defined in `lang/std/core/literals.ks`._

### Implements `Defaultable`

#### initializer `Default`

```kestrel
init()
```

Builds the default-valued instance.

_Defined in `lang/std/core/protocols.ks`._

## struct `DictionaryIterator`

```kestrel
public struct DictionaryIterator[K, V] { /* private fields */ }
```

Single-pass forward iterator over the `(key, value)` entries of a
`Dictionary[K, V, H]`.

Produced by `Dictionary.iter()`. Walks the bucket array once, skipping
`.Empty` and `.Deleted` slots, and yields each occupied entry as a
tuple. Iteration order matches bucket layout, which depends on the
hash and probe sequence — treat it as unspecified. For key- or
value-only views see `KeysIterator` and `ValuesIterator`.

### Examples

```
let dict = ["a": 1, "b": 2];
var it = dict.iter();
it.next();  // Some(("a", 1))   — order is unspecified
it.next();  // Some(("b", 2))
it.next();  // None
```

### Representation

A `(buckets, capacity, index)` triple — pointer to the bucket array,
total slots, and the current scan position.

### Memory Model

Value type. The pointer aliases dictionary storage; do not retain an
iterator across mutations of the source dictionary.

_Defined in `lang/std/collections/dictionary.ks`._

### Members

#### initializer `From Buckets`

```kestrel
init(buckets: Pointer[Bucket[K, V]], capacity: Int64)
```

Constructs an iterator over a raw bucket pointer of the given
capacity.

Prefer `Dictionary.iter()` over calling this directly. The
pointer must outlive the iterator.

##### Safety

`buckets` must point to at least `capacity` initialized
`Bucket[K, V]` slots and remain valid for the iterator's
lifetime.

_Defined in `lang/std/collections/dictionary.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = (K, V)
```

Element type yielded by `next()` — a `(key, value)` tuple.

_Defined in `lang/std/collections/dictionary.ks`._

#### function `next`

```kestrel
public mutating func next() -> (K, V)?
```

Advances the scan to the next occupied slot and returns its
entry, or `None` when no more remain.

Skips `.Empty` and `.Deleted` slots silently. Once `None` is
returned the iterator stays exhausted.

##### Examples

```
var it = ["a": 1].iter();
it.next();  // Some(("a", 1))
it.next();  // None
```

_Defined in `lang/std/collections/dictionary.ks`._

## typealias `DictionaryTypeOperator`

```kestrel
public type DictionaryTypeOperator[K, V] = Dictionary[K, V, DefaultHasher]
```

Compiler-recognized type alias that lets `[K: V]` desugar to
`Dictionary[K, V, DefaultHasher]`.

Allows annotations like `let m: [String: Int64] = [:]` instead of
requiring the user to spell out `Dictionary[String, Int64]`. The
hasher is fixed to `DefaultHasher`; for custom hashers, write the
`Dictionary[...]` form explicitly.

### Examples

```
let counts: [String: Int64] = [:];
func tally(of words: [String: Int64]) -> Int64 { ... }
```

_Defined in `lang/std/collections/dictionary.ks`._

## struct `Heap`

```kestrel
public struct Heap[T] where T: Comparable { /* private fields */ }
```

Binary min-heap backed by `Array[T]`.

O(log n) `push`/`pop`, O(1) `peek` at the minimum element. Builds
from an existing array in O(n) via Floyd's heapify. Iteration yields
elements in storage order (NOT sorted order).

### Examples

```
var h = Heap[Int64]();
h.push(5);
h.push(1);
h.push(3);
h.peek();   // .Some(1)
h.pop();    // .Some(1)
h.pop();    // .Some(3)
```

### Representation

A single `Array[T]` field in standard binary-heap layout: the minimum
lives at index 0, children of node `i` are at `2i + 1` and `2i + 2`.

### Memory Model

Delegates storage to `Array[T]`, inheriting its COW value semantics.
Copying a `Heap` is O(1); the first mutation on a shared copy triggers
the array's copy-on-write barrier.

### Guarantees

- `peek()` always returns the minimum element.
- After `pop()`, the next-smallest element becomes the new minimum.
- Iteration order is unspecified (internal heap layout).

_Defined in `lang/std/collections/heap.ks`._

### Members

#### initializer `Empty`

```kestrel
public init()
```

Creates an empty min-heap with no allocation.

No heap memory is allocated until the first `push`. Use
`Heap(capacity:)` to pre-allocate when the expected size is known.

##### Examples

```
var h = Heap[Int64]();
h.isEmpty;  // true
```

_Defined in `lang/std/collections/heap.ks`._

#### initializer `From Data`

```kestrel
init(data: Array[T])
```

Internal constructor that wraps an existing array as the heap's
backing store. Used by `clone()` to avoid a redundant heapify.

_Defined in `lang/std/collections/heap.ks`._

#### initializer `From Iterable`

```kestrel
public init[I](from: I) where I: Iterable, I.Item == T
```

Builds a min-heap by collecting elements then heapifying in O(n).

All elements are first appended to the backing array, then
Floyd's algorithm establishes the heap invariant in a single
bottom-up pass. This is faster than pushing elements one at a
time (O(n) vs O(n log n)).

##### Examples

```
let h = Heap(from: [5, 3, 1, 4, 2]);
h.peek();   // .Some(1)
h.count;    // 5
```

_Defined in `lang/std/collections/heap.ks`._

#### initializer `With Capacity`

```kestrel
public init(capacity: Int64)
```

Creates an empty min-heap with at least `capacity` slots reserved.

Pre-allocates the backing array so that up to `capacity` elements
can be pushed without triggering a reallocation.

##### Examples

```
var h = Heap[Int64](capacity: 100);
h.count;  // 0
```

_Defined in `lang/std/collections/heap.ks`._

#### function `clear`

```kestrel
public mutating func clear()
```

Removes all elements, retaining allocated capacity.

After calling `clear()`, `count` is 0 but the backing array keeps
its buffer so subsequent `push` calls avoid reallocation.

##### Examples

```
var h = Heap(from: [3, 1, 2]);
h.clear();
h.isEmpty;  // true
```

_Defined in `lang/std/collections/heap.ks`._

#### field `count`

```kestrel
public var count: Int64 { get }
```

Number of elements in the heap.

##### Examples

```
let h = Heap(from: [3, 1, 2]);
h.count;  // 3
```

_Defined in `lang/std/collections/heap.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

True when the heap contains no elements.

##### Examples

```
Heap[Int64]().isEmpty;  // true
Heap(from: [1]).isEmpty;  // false
```

_Defined in `lang/std/collections/heap.ks`._

#### function `peek`

```kestrel
public func peek() -> T?
```

Returns the minimum element without removing it. O(1).

Returns `.None` on an empty heap. The removing counterpart is
`pop()`.

##### Examples

```
let h = Heap(from: [3, 1, 2]);
h.peek();  // .Some(1)
Heap[Int64]().peek();  // .None
```

_Defined in `lang/std/collections/heap.ks`._

#### function `pop`

```kestrel
public mutating func pop() -> T?
```

Removes and returns the minimum element, or `.None` if empty. O(log n).

Swaps the root with the last element, removes the last, then sifts
the new root down to restore the heap invariant. The non-removing
mirror is `peek()`.

##### Examples

```
var h = Heap(from: [3, 1, 2]);
h.pop();  // .Some(1)
h.pop();  // .Some(2)
h.pop();  // .Some(3)
h.pop();  // .None
```

_Defined in `lang/std/collections/heap.ks`._

#### function `push`

```kestrel
public mutating func push(T)
```

Inserts `element`, maintaining the min-heap invariant. O(log n).

The element is appended to the backing array then sifted up to
restore the heap property. Amortized O(log n) because the array
may occasionally grow its buffer.

##### Examples

```
var h = Heap[Int64]();
h.push(3);
h.push(1);
h.peek();  // .Some(1)
```

_Defined in `lang/std/collections/heap.ks`._

#### function `reserveCapacity`

```kestrel
public mutating func reserveCapacity(minimumCapacity: Int64)
```

Ensures the backing array can hold at least `capacity` elements
without reallocating.

If the current capacity already meets or exceeds `capacity`, this
is a no-op. Otherwise the backing array grows to accommodate the
requested number of slots.

##### Examples

```
var h = Heap[Int64]();
h.reserveCapacity(minimumCapacity: 100);
```

_Defined in `lang/std/collections/heap.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = T
```

`Iterable` element type.

_Defined in `lang/std/collections/heap.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = ArraySliceIterator[T]
```

`Iterable` iterator type — reuses the backing array's iterator.

_Defined in `lang/std/collections/heap.ks`._

#### function `iter`

```kestrel
public func iter() -> ArraySliceIterator[T]
```

Returns an iterator over elements in storage order (NOT sorted).

The iteration order reflects the internal heap layout, not the
sorted order. To consume elements smallest-first, use `pop()` in
a loop instead.

##### Examples

```
let h = Heap(from: [5, 3, 1]);
var iter = h.iter();
// yields elements in heap-array order, not 1, 3, 5
```

_Defined in `lang/std/collections/heap.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> Heap[T]
```

Returns a shallow copy of this heap. O(1).

The backing array's copy-on-write semantics mean the actual deep
copy is deferred until the first mutation on either the original
or the clone.

##### Examples

```
let h = Heap(from: [3, 1, 2]);
var h2 = h.clone();
h2.push(0);
h.peek();   // .Some(1) -- original unchanged
h2.peek();  // .Some(0)
```

_Defined in `lang/std/collections/heap.ks`._

## struct `KeysIterator`

```kestrel
public struct KeysIterator[K, V] where K: Hashable { /* private fields */ }
```

Single-pass iterator yielding only the keys of a dictionary.

Wraps a `DictionaryIterator[K, V]` and discards the value half of
each entry. Order matches the underlying entry iteration and is
unspecified.

### Examples

```
var it = ["a": 1, "b": 2].keys.iter();
it.next();  // Some("a")  — order unspecified
it.next();  // Some("b")
it.next();  // None
```

### Representation

Wraps a `DictionaryIterator[K, V]`.

### Memory Model

Value type. Aliases dictionary storage; do not retain across
mutations.

_Defined in `lang/std/collections/dictionary.ks`._

### Members

#### initializer `From Dict`

```kestrel
public init(dictIter: DictionaryIterator[K, V])
```

Wraps a `DictionaryIterator` to yield only its keys.

Prefer `KeysView.iter()` over calling this directly.

_Defined in `lang/std/collections/dictionary.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = K
```

Element type yielded by `next()` — `K`.

_Defined in `lang/std/collections/dictionary.ks`._

#### function `next`

```kestrel
public mutating func next() -> K?
```

Returns the next key, or `None` when the underlying iterator
is exhausted.

##### Examples

```
var it = ["a": 1].keys.iter();
it.next();  // Some("a")
it.next();  // None
```

_Defined in `lang/std/collections/dictionary.ks`._

## struct `KeysView`

```kestrel
public struct KeysView[K, V] where K: Hashable { /* private fields */ }
```

Lazy `Iterable` view over the keys of a dictionary.

Returned by `Dictionary.keys`. Constructing the view is O(1) — it
stores the bucket pointer and capacity. The view is invalidated by
any mutation that may reallocate.

### Examples

```
let dict = ["a": 1, "b": 2];
for k in dict.keys { print(k) }
let arr = Array(from: dict.keys);
```

### Representation

`(buckets, capacity)` — a pointer into the source dictionary's
bucket array plus the total slot count.

### Memory Model

Value type that borrows the source dictionary's buffer.

_Defined in `lang/std/collections/dictionary.ks`._

### Members

#### initializer `From Buckets`

```kestrel
init(buckets: Pointer[Bucket[K, V]], capacity: Int64)
```

Internal — constructs a view from a bucket pointer and capacity.
Use `Dictionary.keys` to obtain a view.

_Defined in `lang/std/collections/dictionary.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = K
```

`Iterable` element type — `K`.

_Defined in `lang/std/collections/dictionary.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = KeysIterator[K, V]
```

Concrete iterator type returned by `iter()`.

_Defined in `lang/std/collections/dictionary.ks`._

#### function `iter`

```kestrel
public func iter() -> KeysIterator[K, V]
```

Returns a fresh `KeysIterator[K, V]` over the view.

Each call returns a new iterator starting at the beginning of
the bucket array.

_Defined in `lang/std/collections/dictionary.ks`._

## struct `ReversedSliceIterator`

```kestrel
public struct ReversedSliceIterator[T] { /* private fields */ }
```

_Defined in `lang/std/collections/views.ks`._

### Members

#### initializer `init`

```kestrel
public init(ptr: Pointer[T], remaining: Int64)
```

_Defined in `lang/std/collections/views.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/collections/views.ks`._

#### function `next`

```kestrel
public mutating func next() -> Optional[T]
```

_Defined in `lang/std/collections/views.ks`._

## struct `ReversedView`

```kestrel
public struct ReversedView[T] { /* private fields */ }
```

Multi-pass lazy view that iterates a contiguous collection
back-to-front without allocating.

_Defined in `lang/std/collections/views.ks`._

### Members

#### field `count`

```kestrel
public var count: Int64 { get }
```

_Defined in `lang/std/collections/views.ks`._

#### field `first`

```kestrel
public var first: Optional[T] { get }
```

_Defined in `lang/std/collections/views.ks`._

#### initializer `init`

```kestrel
public init(slice: ArraySlice[T])
```

_Defined in `lang/std/collections/views.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

_Defined in `lang/std/collections/views.ks`._

#### field `last`

```kestrel
public var last: Optional[T] { get }
```

_Defined in `lang/std/collections/views.ks`._

#### subscript `subscript`

```kestrel
public subscript(Int64) -> T { get }
```

_Defined in `lang/std/collections/views.ks`._

#### function `toArray`

```kestrel
public func toArray() -> Array[T]
```

_Defined in `lang/std/collections/views.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/collections/views.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = ReversedSliceIterator[T]
```

_Defined in `lang/std/collections/views.ks`._

#### function `iter`

```kestrel
public func iter() -> ReversedSliceIterator[T]
```

_Defined in `lang/std/collections/views.ks`._

## protocol `SeqRange`

```kestrel
public protocol SeqRange
```

Resolves any range-like type to a half-open `Range[Int64]` given a
collection length. Used by `removeSubrange` and `replaceSubrange` so
they accept `Range`, `ClosedRange`, `RangeFrom`, `RangeUpTo`, and
`RangeThrough` through a single generic parameter.

_Defined in `lang/std/collections/slice.ks`._

### Members

#### function `resolve`

```kestrel
func resolve(Int64) -> Range[Int64]
```

_Defined in `lang/std/collections/slice.ks`._

## struct `Set`

```kestrel
public struct Set[T, H = DefaultHasher] where T: Hashable, H: Hasher, H: Defaultable { /* private fields */ }
```

An unordered hash set of unique elements, parameterized over the
hasher type `H` (defaults to `DefaultHasher`).

Backed by a `Dictionary[T, Unit, H]` — the dictionary's keys are
the set's elements, and `Unit` fills the value slot. Inherits
O(1) average-case lookup, insertion, and removal, plus
copy-on-write storage from the underlying dictionary: copying a
`Set` is O(1), with the deep clone deferred until either side
mutates. Iteration order is unspecified. For ordered or
associative-style storage, see `Array[T]` and `Dictionary[K, V]`.

### Examples

```
var fruits: Set = ["apple", "banana", "cherry"];
fruits.insert("date");
fruits.contains("apple");   // true
fruits.remove("banana");

let a: Set = [1, 2, 3];
let b: Set = [3, 4, 5];
a.union(b);                  // {1, 2, 3, 4, 5}
a.intersection(b);           // {3}
a.isSubset(of: b);           // false
```

### Set Literals

Sets share array-literal syntax — you tell the compiler which one
you want via the type annotation:

```
let empty: Set[Int64] = [];
let numbers: Set = [1, 2, 3];
let strings: Set[String] = ["a", "b", "c"];
```

### Hashing

Each element's hash is computed via `T: Hashable` and stored in the
underlying dictionary's bucket. Swap the hasher type by writing
`Set[T, SipHasher]` etc.; the default `DefaultHasher` is FNV-1a
(see `DefaultHasher` for caveats around adversarial inputs).

### Representation

One field, `dict: Dictionary[T, Unit, H]`. All set operations
delegate to the dictionary.

### Memory Model

Reference-counted storage with copy-on-write *value* semantics —
inherited from the backing `Dictionary`. Copying a `Set` is O(1)
and shares storage; the next mutation triggers the deep clone so
the change is invisible to other copies.

### Guarantees

- Elements are unique by `Hashable`/`Equatable` equality.
- Iteration order is **not** specified.
- Operations marked O(1) are amortized; the underlying dictionary
  resizes geometrically.

_Defined in `lang/std/collections/set.ks`._

### Members

#### initializer `Array Literal`

```kestrel
public init(arrayLiteral: LiteralSlice[T])
```

Creates a set from an array literal slice — emitted by the
compiler when you write `let s: Set = [1, 2, 3]`.

Pre-allocates capacity to the literal's element count (so the
build avoids resizing) and inserts each element. Duplicates
collapse.

##### Examples

```
// Triggered by the array-literal-with-Set-annotation syntax:
let nums: Set = [1, 2, 3];
```

_Defined in `lang/std/collections/set.ks`._

#### typealias `Element`

```kestrel
type Element = T
```

`ExpressibleByArrayLiteral` element type — equals `T`.

_Defined in `lang/std/collections/set.ks`._

#### initializer `Empty`

```kestrel
public init()
```

Creates an empty set with no allocation.

The first insert allocates the smallest dictionary bucket
array (currently 8 slots). For pre-sized creation, use
`init(capacity:)`.

##### Examples

```
let set = Set[String]();
set.isEmpty;   // true
set.capacity;  // 0
```

_Defined in `lang/std/collections/set.ks`._

#### initializer `From Iterable`

```kestrel
public init[I](from: I) where I: Iterable, I.Item == T
```

Creates a set by inserting every element produced by an
iterable.

Duplicates collapse silently (insert returns `false` for the
already-present case). Capacity grows geometrically as
inserts arrive — for sized sources, follow up with
`shrinkToFit()` if memory matters.

##### Examples

```
let arr = [1, 2, 2, 3, 3, 3];
let set = Set(from: arr);    // {1, 2, 3}
let r   = Set(from: 1..<4);  // {1, 2, 3}
```

_Defined in `lang/std/collections/set.ks`._

#### initializer `Literal Bridge`

```kestrel
public init(_arrayLiteralPointer: consuming lang.ptr[T], _arrayLiteralCount: consuming lang.i64)
```

Compiler-emitted bridge for `[a, b, c]` literals constructing
a `Set`.

Forwards to `init(arrayLiteral:)` after wrapping the raw
`(ptr, count)` in a `LiteralSlice`. Not called by user code.

##### Safety

The compiler guarantees `_arrayLiteralPointer` covers exactly
`_arrayLiteralCount` initialized elements of `T`.

_Defined in `lang/std/collections/set.ks`._

#### initializer `With Capacity`

```kestrel
public init(capacity: Int64)
```

Creates an empty set sized to hold at least `capacity` elements
without resizing.

The actual allocated capacity rounds up to the next power of
two (minimum 8) per the underlying dictionary policy. A
non-positive `capacity` behaves like `init()`. Panics on
allocation failure.

##### Examples

```
var set = Set[String](capacity: 1000);
set.capacity;  // 1024
set.count;     // 0
```

_Defined in `lang/std/collections/set.ks`._

#### function `all`

```kestrel
public func all(where: (T) -> Bool) -> Bool
```

`true` when every element satisfies `predicate` (vacuously
true for empty sets).

Short-circuits on the first failure. Dual of
`any { ... }`.

##### Examples

```
Set([2, 4, 6]).all { (x) in x % 2 == 0 };  // true
Set([1, 2, 4]).all { (x) in x % 2 == 0 };  // false
Set[Int64]().all { (x) in false };           // true (vacuous)
```

_Defined in `lang/std/collections/set.ks`._

#### function `any`

```kestrel
public func any(where: (T) -> Bool) -> Bool
```

`true` when at least one element satisfies `predicate`.

Alias for `contains { ... }` — both names exist so
predicate-style code reads naturally regardless of context.
Short-circuits.

##### Examples

```
Set([1, 2, 3]).any { (x) in x > 2 };  // true
Set[Int64]().any { (x) in true };     // false (empty)
```

_Defined in `lang/std/collections/set.ks`._

#### field `capacity`

```kestrel
public var capacity: Int64 { get }
```

Total bucket capacity in the backing dictionary — always
`>= count`.

Resizes (via the dictionary's 75% load policy) trigger the
next insert past the threshold. Use `reserveCapacity(...)` to
pre-grow and `shrinkToFit()` to release excess.

##### Examples

```
let set = Set[String](capacity: 100);
set.capacity;  // 128
```

_Defined in `lang/std/collections/set.ks`._

#### function `clear`

```kestrel
public mutating func clear()
```

Removes every element, leaving capacity untouched.

Forwards to the dictionary's `clear()`. Follow with
`shrinkToFit()` to release the buffer.

##### Examples

```
var set: Set = [1, 2, 3];
set.clear();      // set == {}
set.capacity;     // unchanged
```

_Defined in `lang/std/collections/set.ks`._

#### function `compactMap`

```kestrel
public func compactMap[U]((T) -> U?) -> Set[U, H] where U: Hashable
```

Returns a new set with each element run through `transform`,
dropping any `None` results.

Useful for parse-or-skip patterns. Same uniqueness caveat as
`map(...)` — collisions in the transformed values
collapse.

##### Examples

```
let set: Set = ["1", "two", "3"];
let nums = set.compactMap { (s) in Int64.parse(s) };
// {1, 3}  — "two" failed to parse
```

_Defined in `lang/std/collections/set.ks`._

#### function `contains`

```kestrel
public func contains(T) -> Bool
```

`true` if `element` is a member of the set; O(1) average.

Forwards to the dictionary's key lookup. For predicate-based
search use `contains { ... }`.

##### Examples

```
let set: Set = [1, 2, 3];
set.contains(2);  // true
set.contains(5);  // false
```

_Defined in `lang/std/collections/set.ks`._

#### function `contains`

```kestrel
public func contains(where: (T) -> Bool) -> Bool
```

`true` if any element satisfies `predicate`.

Linear scan; short-circuits on the first match. `false` for
empty sets. The aliased shape `any { ... }` exists for
symmetry with `Array`.

##### Examples

```
Set([1, 2, 3]).contains { (x) in x > 2 };  // true
Set([1, 2, 3]).contains { (x) in x > 5 };  // false
```

_Defined in `lang/std/collections/set.ks`._

#### field `count`

```kestrel
public var count: Int64 { get }
```

Number of unique elements; O(1).

Forwards to the backing dictionary's `count`.

##### Examples

```
Set([1, 2, 3]).count;   // 3
Set[Int64]().count;     // 0
```

_Defined in `lang/std/collections/set.ks`._

#### function `countItems`

```kestrel
public func countItems(where: (T) -> Bool) -> Int64
```

Returns the number of elements for which `predicate` is true.

Linear scan, no short-circuit. For just a presence check use
`any { ... }`; for a yes/no on every element,
`all { ... }`.

##### Examples

```
Set([1, 2, 3, 4, 5]).countItems { (x) in x % 2 == 0 };  // 2
Set[Int64]().countItems { (x) in true };                // 0
```

_Defined in `lang/std/collections/set.ks`._

#### function `deepClone`

```kestrel
public func deepClone() -> Set[T, H]
```

Returns a fully-detached copy of the set with no shared
storage; every element is also `clone()`-d.

Use over `clone()` when you specifically want to break the
lazy COW share — for example, before passing the copy to
another thread or system that might race with further
mutations.

##### Examples

```
let a: Set = [[1, 2], [3, 4]];  // Set of arrays
let b = a.deepClone();          // fully independent copy
```

_Defined in `lang/std/collections/set.ks`._

#### field `dict`

```kestrel
var dict: Dictionary[T, Unit, H]
```

Backing dictionary. Keys are the set's elements; values are
always `Unit()`.

_Defined in `lang/std/collections/set.ks`._

#### function `difference`

```kestrel
public func difference(Set[T, H]) -> Set[T, H]
```

Returns a new set of every element in `self` that is **not**
in `other` — the set difference, "self minus other".

Non-mutating mirror of `formDifference(...)`. Order of
arguments matters: `a.difference(b)` is generally not equal
to `b.difference(a)`.

##### Examples

```
let a: Set = [1, 2, 3];
let b: Set = [2, 3, 4];
a.difference(b);  // {1}
b.difference(a);  // {4}
```

_Defined in `lang/std/collections/set.ks`._

#### function `filter`

```kestrel
public func filter(where: (T) -> Bool) -> Set[T, H]
```

Returns a new set containing only elements for which
`predicate` is true.

Non-mutating mirror of `retain { ... }`. Allocates a fresh
set; for in-place filtering use `retain` or
`removeAll { ... }`.

##### Examples

```
let set: Set = [1, 2, 3, 4, 5];
let evens = set.filter { (x) in x % 2 == 0 };  // {2, 4}
```

_Defined in `lang/std/collections/set.ks`._

#### function `first`

```kestrel
public func first(where: (T) -> Bool) -> T?
```

Returns *some* element matching `predicate`, or `None`.

"First" is determined by iteration order, which is
unspecified — treat the result as arbitrary among matching
elements. Short-circuits on the first match.

##### Examples

```
let set: Set = [1, 2, 3, 4, 5];
set.first { (x) in x > 3 };   // Some(4) or Some(5)
set.first { (x) in x > 99 };  // None
```

_Defined in `lang/std/collections/set.ks`._

#### function `flatMap`

```kestrel
public func flatMap[U]((T) -> Set[U, H]) -> Set[U, H] where U: Hashable
```

Returns a new set formed by unioning every set produced by
`transform`.

Each element maps to a `Set[U, H]`; those sets are merged
together. The result holds the unique union — duplicates
across sub-sets collapse, as with all set operations.

##### Examples

```
let set: Set = [1, 2];
let expanded = set.flatMap { (x) in Set([x, x * 10]) };
// {1, 10, 2, 20}
```

_Defined in `lang/std/collections/set.ks`._

#### function `formDifference`

```kestrel
public mutating func formDifference(Set[T, H])
```

In-place difference: removes every element of `self` that **is**
in `other`.

Mutating mirror of `difference(...)`. The result is "self
minus other".

##### Examples

```
var a: Set = [1, 2, 3];
let b: Set = [2, 3, 4];
a.formDifference(b);  // a == {1}
```

_Defined in `lang/std/collections/set.ks`._

#### function `formIntersection`

```kestrel
public mutating func formIntersection(Set[T, H])
```

In-place intersection: removes every element of `self` that
is **not** in `other`.

Mutating mirror of `intersection(...)`. Iterates over
`self`, so the cost scales with `self.count`, not
`other.count`.

##### Examples

```
var a: Set = [1, 2, 3];
let b: Set = [2, 3, 4];
a.formIntersection(b);  // a == {2, 3}
```

_Defined in `lang/std/collections/set.ks`._

#### function `formSymmetricDifference`

```kestrel
public mutating func formSymmetricDifference(Set[T, H])
```

In-place symmetric difference: keeps elements in exactly one
of `self` or `other`.

Mutating mirror of `symmetricDifference(...)`. Two passes:
removes shared elements, then inserts elements unique to
`other`.

##### Examples

```
var a: Set = [1, 2, 3];
let b: Set = [2, 3, 4];
a.formSymmetricDifference(b);  // a == {1, 4}
```

_Defined in `lang/std/collections/set.ks`._

#### function `formUnion`

```kestrel
public mutating func formUnion(Set[T, H])
```

In-place union: adds every element of `other` to `self`.

Mutating mirror of `union(...)`. For multi-source unions,
chain calls or use `insert(contentsOf:)` over the elements.

##### Examples

```
var a: Set = [1, 2];
let b: Set = [2, 3];
a.formUnion(b);  // a == {1, 2, 3}
```

_Defined in `lang/std/collections/set.ks`._

#### function `getDict`

```kestrel
func getDict() -> Dictionary[T, Unit, H]
```

Returns the backing `Dictionary[T, Unit, H]`. Internal helper
for extensions that need direct dictionary access.

_Defined in `lang/std/collections/set.ks`._

#### function `insert`

```kestrel
public mutating func insert(T) -> Bool
```

Inserts `element`, returning whether it was newly added.

Returns `true` if the element was added, `false` if it was
already present (in which case the set is unchanged). May
trigger a dictionary resize and COW. For bulk inserts, see
`insert(contentsOf:)`.

##### Examples

```
var set: Set = [1, 2];
set.insert(3);  // true; set == {1, 2, 3}
set.insert(2);  // false; already present
```

_Defined in `lang/std/collections/set.ks`._

#### function `insert`

```kestrel
public mutating func insert[I](contentsOf: I) where I: Iterable, I.Item == T
```

Inserts every element produced by an iterable; duplicates
collapse silently.

Sugar for "insert in a loop". For union with another `Set`,
prefer `formUnion(...)` — it's the same semantically but
reads more naturally.

##### Examples

```
var set: Set = [1, 2];
set.insert(contentsOf: [3, 4, 5]);  // {1, 2, 3, 4, 5}
set.insert(contentsOf: 5..<8);      // {1, 2, 3, 4, 5, 6, 7}
```

_Defined in `lang/std/collections/set.ks`._

#### function `intersection`

```kestrel
public func intersection(Set[T, H]) -> Set[T, H]
```

Returns a new set containing only elements present in both
`self` and `other`.

Non-mutating mirror of `formIntersection(...)`. For
efficiency, iterates over `self`; pass the smaller set as the
receiver if it matters.

##### Examples

```
let a: Set = [1, 2, 3];
let b: Set = [2, 3, 4];
a.intersection(b);  // {2, 3}
```

_Defined in `lang/std/collections/set.ks`._

#### function `isDisjoint`

```kestrel
public func isDisjoint(with: Set[T, H]) -> Bool
```

`true` if `self` and `other` share no elements.

Iterates over the smaller set for efficiency (swaps the
arguments internally if needed). Empty sets are disjoint
from anything, including each other.

##### Examples

```
let a: Set = [1, 2];
let b: Set = [3, 4];
let c: Set = [2, 3];
a.isDisjoint(with: b);  // true
a.isDisjoint(with: c);  // false (share 2)
```

_Defined in `lang/std/collections/set.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

`true` when the set has no elements; equivalent to
`count == 0`.

##### Examples

```
Set[Int64]().isEmpty;   // true
Set([1]).isEmpty;       // false
```

_Defined in `lang/std/collections/set.ks`._

#### function `isStrictSubset`

```kestrel
public func isStrictSubset(of: Set[T, H]) -> Bool
```

`true` if `self` is a subset of `other` and the two sets are
not equal.

Strict (proper) subset — excludes the case where the sets are
equal. Mirror of `isStrictSuperset(of:)`.

##### Examples

```
let a: Set = [1, 2];
let b: Set = [1, 2, 3];
a.isStrictSubset(of: b);  // true
a.isStrictSubset(of: a);  // false (equal, not strict)
```

_Defined in `lang/std/collections/set.ks`._

#### function `isStrictSuperset`

```kestrel
public func isStrictSuperset(of: Set[T, H]) -> Bool
```

`true` if `self` is a superset of `other` and the two sets
are not equal.

Strict (proper) superset. Mirror of `isStrictSubset(of:)`.

##### Examples

```
let a: Set = [1, 2, 3];
let b: Set = [1, 2];
a.isStrictSuperset(of: b);  // true
a.isStrictSuperset(of: a);  // false (equal, not strict)
```

_Defined in `lang/std/collections/set.ks`._

#### function `isSubset`

```kestrel
public func isSubset(of: Set[T, H]) -> Bool
```

`true` if every element of `self` appears in `other`.

A set is always a subset of itself (reflexive). Short-circuits
on the first missing element, and skips the inner scan when
`self.count > other.count`. For "subset but not equal" use
`isStrictSubset(of:)`.

##### Examples

```
let a: Set = [1, 2];
let b: Set = [1, 2, 3];
a.isSubset(of: b);  // true
b.isSubset(of: a);  // false
a.isSubset(of: a);  // true
```

_Defined in `lang/std/collections/set.ks`._

#### function `isSuperset`

```kestrel
public func isSuperset(of: Set[T, H]) -> Bool
```

`true` if every element of `other` appears in `self`.

Reflexive (a set is its own superset). Implemented as
`other.isSubset(of: self)` for code reuse.

##### Examples

```
let a: Set = [1, 2, 3];
let b: Set = [1, 2];
a.isSuperset(of: b);  // true
b.isSuperset(of: a);  // false
```

_Defined in `lang/std/collections/set.ks`._

#### function `map`

```kestrel
public func map[U]((T) -> U) -> Set[U, H] where U: Hashable
```

Returns a new set with each element run through `transform`.

**Cardinality may shrink**: if `transform` maps two distinct
elements to the same output, the result holds only one copy
(sets are unique). For an `Optional`-aware variant that drops
`None`, use `compactMap(...)`.

##### Examples

```
let set: Set = [1, 2, 3];
let doubled = set.map { (x) in x * 2 };
// {2, 4, 6}

let words: Set = ["Hello", "WORLD"];
let lower = words.map { (s) in s.lowercase() };
// {"hello", "world"} — even though both originals lowercase to distinct strings
```

_Defined in `lang/std/collections/set.ks`._

#### function `max`

```kestrel
public func max() -> T?
```

Returns the largest element, or `None` for an empty set.

Single linear pass. Mirror of `min()`.

##### Examples

```
Set([3, 1, 4]).max();  // Some(4)
Set[Int64]().max();    // None
```

_Defined in `lang/std/collections/set.ks`._

#### function `min`

```kestrel
public func min() -> T?
```

Returns the smallest element, or `None` for an empty set.

Single linear pass; ties go to the first occurrence in
iteration order (which is unspecified, so equally-minimal
elements compare equal anyway).

##### Examples

```
Set([3, 1, 4]).min();  // Some(1)
Set[Int64]().min();    // None
```

_Defined in `lang/std/collections/set.ks`._

#### function `remove`

```kestrel
public mutating func remove(T) -> Bool
```

Removes `element` if present; returns whether anything was
removed.

Leaves a tombstone in the backing dictionary — see
`Dictionary.remove`. Tombstones are reclaimed by the next
resize. Triggers COW only when an element is actually removed.

##### Examples

```
var set: Set = [1, 2, 3];
set.remove(2);  // true; set == {1, 3}
set.remove(5);  // false; set unchanged
```

_Defined in `lang/std/collections/set.ks`._

#### function `removeAll`

```kestrel
public mutating func removeAll(where: (T) -> Bool)
```

Removes every element for which `predicate` is true.

Inverse of `retain { ... }`. Same two-pass structure.

##### Examples

```
var set: Set = [1, 2, 3, 4, 5];
set.removeAll { (x) in x % 2 == 0 };  // {1, 3, 5}
```

_Defined in `lang/std/collections/set.ks`._

#### function `reserveCapacity`

```kestrel
public mutating func reserveCapacity(Int64)
```

Grows the backing dictionary so at least `minimumCapacity`
elements fit without resizing.

No-op when current capacity already suffices. Implemented by
rebuilding the underlying dictionary at the new capacity (a
little heavier than `Dictionary.reserveCapacity` directly,
since it reinserts each element). Opposite of `shrinkToFit()`.

##### Examples

```
var set = Set[String]();
set.reserveCapacity(1000);
// No reallocations for the first ~750 inserts.
```

_Defined in `lang/std/collections/set.ks`._

#### function `retain`

```kestrel
public mutating func retain(where: (T) -> Bool)
```

Keeps only elements for which `predicate` is true.

Two-pass implementation: collects elements to remove, then
deletes each. Stable in iteration semantics (set is unordered
anyway). Mirror is `removeAll { ... }`.

##### Examples

```
var set: Set = [1, 2, 3, 4, 5];
set.retain { (x) in x % 2 == 0 };  // {2, 4}
```

_Defined in `lang/std/collections/set.ks`._

#### function `shrinkToFit`

```kestrel
public mutating func shrinkToFit()
```

Reduces backing-dictionary capacity to fit the current count.

Rebuilds the dictionary at a smaller capacity, dropping any
tombstones. No-op when capacity already matches. Useful after
large removals.

##### Examples

```
var set = Set[String](capacity: 1000);
set.insert("a");
set.shrinkToFit();  // capacity drops toward count
```

_Defined in `lang/std/collections/set.ks`._

#### function `sorted`

```kestrel
public func sorted() -> Array[T]
```

Returns the set's elements as an ascending-sorted `Array[T]`.

Convenience for "I want this set as an ordered list". Duplicates
have already collapsed in the set, so the result has no repeats.

##### Examples

```
Set([3, 1, 4, 1, 5]).sorted();  // [1, 3, 4, 5]
```

_Defined in `lang/std/collections/set.ks`._

#### function `sum`

```kestrel
public func sum() -> T
```

Returns the sum of every element, starting from `T()` (the
default-constructed zero).

Empty sets return `T()` — `0` for `Int64`, `""` for `String`,
etc. Linear in `count`.

##### Examples

```
Set([1, 2, 3]).sum();  // 6
Set[Int64]().sum();    // 0
```

_Defined in `lang/std/collections/set.ks`._

#### function `symmetricDifference`

```kestrel
public func symmetricDifference(Set[T, H]) -> Set[T, H]
```

Returns a new set of elements in exactly one of `self` or
`other`.

Non-mutating mirror of `formSymmetricDifference(...)`.
Equivalent to `union(...) - intersection(...)`. The
operation is commutative — order of arguments doesn't change
the result.

##### Examples

```
let a: Set = [1, 2, 3];
let b: Set = [2, 3, 4];
a.symmetricDifference(b);  // {1, 4}
```

_Defined in `lang/std/collections/set.ks`._

#### function `toArray`

```kestrel
public func toArray() -> Array[T]
```

Returns an `Array[T]` with every element of the set.

Order matches iteration order (i.e. unspecified). Capacity is
pre-reserved to `count` so the build avoids reallocations.
For an ordering, follow with `Array.sort()` or
`sorted()` (in the `T: Comparable` extension below).

##### Examples

```
let set: Set = [1, 2, 3];
let arr = set.toArray();  // [1, 2, 3] in some order
```

_Defined in `lang/std/collections/set.ks`._

#### function `union`

```kestrel
public func union(Set[T, H]) -> Set[T, H]
```

Returns a new set containing every element from `self` and
`other`.

Non-mutating mirror of `formUnion(...)`. Internally clones
`self` (cheap COW) and adds `other` into the copy.

##### Examples

```
let a: Set = [1, 2, 3];
let b: Set = [3, 4, 5];
a.union(b);  // {1, 2, 3, 4, 5}
```

_Defined in `lang/std/collections/set.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = T
```

`Iterable` element type — `T`.

_Defined in `lang/std/collections/set.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = SetIterator[T, H]
```

Concrete iterator type returned by `iter()`.

_Defined in `lang/std/collections/set.ks`._

#### function `iter`

```kestrel
public func iter() -> SetIterator[T, H]
```

Returns a single-pass `SetIterator[T, H]` over the elements.

Order is unspecified and may change between mutations. The
iterator borrows the underlying buffer; do not mutate the
set while iterating.

##### Examples

```
for item in set.iter() { print(item); }
let arr = Array(from: set.iter());
```

_Defined in `lang/std/collections/set.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> Set[T, H]
```

Returns a `Set` sharing the same storage; the deep copy is
deferred until either side mutates.

O(1) — bumps the backing dictionary's `RcBox` refcount. The
first mutation on either side triggers the deep clone. For
an immediate eager copy, use `deepClone()` (in the
`T: Cloneable` extension below).

##### Examples

```
let a: Set = [1, 2, 3];
var b = a.clone();   // O(1), shares storage
b.insert(4);         // b deep-copies here; a is unchanged
```

_Defined in `lang/std/collections/set.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Set[T, H]) -> Bool
```

`true` when `self` and `other` contain exactly the same
elements.

Order-independent (sets are unordered). Implemented as
"equal counts and `self.isSubset(of: other)`" — short-circuits
at the count check.

##### Examples

```
Set([1, 2, 3]).isEqual(to: Set([3, 2, 1]));  // true
Set([1, 2]).isEqual(to: Set([1, 2, 3]));     // false
```

_Defined in `lang/std/collections/set.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating StringBuilder, FormatOptions)
```

Renders the set as `"{" + elements.joined(", ") + "}"`,
passing `options` to each element's `format`.

##### Examples

```
Set([1, 2, 3]).format();  // "{1, 2, 3}" — order unspecified
Set[Int64]().format();    // "{}"
"\{Set([1, 2, 3])}";      // "{1, 2, 3}" via interpolation
```

_Defined in `lang/std/collections/set.ks`._

### Implements `ExpressibleByArrayLiteral`

#### initializer `Array Literal`

```kestrel
init(arrayLiteral: LiteralSlice[Element])
```

Builds an instance from a literal slice of elements.

_Defined in `lang/std/core/literals.ks`._

## struct `SetIterator`

```kestrel
public struct SetIterator[T, H = DefaultHasher] where T: Hashable, H: Hasher, H: Defaultable { /* private fields */ }
```

Single-pass forward iterator over the elements of a `Set[T, H]`.

Returned by `Set.iter()`. Wraps the underlying
`DictionaryIterator[T, Unit]` and discards the (unused) value
half of each entry, yielding only the key. Iteration order
matches the underlying bucket layout and is unspecified.

### Examples

```
let set: Set = [1, 2, 3];
for item in set { print(item); }
```

### Representation

Wraps a `DictionaryIterator[T, Unit]`.

### Memory Model

Value type. Aliases the source set's bucket array; do not retain
across mutations of the set.

_Defined in `lang/std/collections/set.ks`._

### Members

#### initializer `From Dict`

```kestrel
public init(dictIter: DictionaryIterator[T, Unit])
```

Wraps a `DictionaryIterator` to yield only its keys.

Low-level — prefer `Set.iter()` over calling this directly.

_Defined in `lang/std/collections/set.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = T
```

Element type yielded by `next()` — `T`.

_Defined in `lang/std/collections/set.ks`._

#### function `next`

```kestrel
public mutating func next() -> T?
```

Returns the next element, or `None` when the underlying
iterator is exhausted.

Once exhausted, the iterator stays exhausted.

##### Examples

```
var it = Set([1, 2]).iter();
it.next();  // Some(1)  — order unspecified
it.next();  // Some(2)
it.next();  // None
```

_Defined in `lang/std/collections/set.ks`._

## protocol `Slice`

```kestrel
public protocol Slice[T]
```

Shared read-only protocol for contiguous collections.

`Slice[T]` is the contiguous-collection counterpart to `Str` in
`std.text`: one kernel method (`asSlice`), all read-only logic in a
protocol extension. Both `Array[T]` and `ArraySlice[T]` conform, so
generic code constrained to `S: Slice[T]` accepts either without
overloading.

### Examples

```
func sum[S](s: S) -> Int64 where S: Slice[Int64] {
    var total: Int64 = 0;
    for elem in s { total = total + elem }
    total
}
sum([1, 2, 3]);              // works with Array
sum([1, 2, 3].asSlice());    // works with ArraySlice
```

_Defined in `lang/std/collections/slice.ks`._

### Members

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
func asSlice() -> ArraySlice[T]
```

_Defined in `lang/std/collections/slice.ks`._

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

#### function `drop`

```kestrel
public func drop(last: Int64) -> ArraySlice[T]
```

Returns a slice with the last `count` elements skipped. O(1).

Complement of `suffix`.

##### Errors

Panics if `count > self.count`.

##### Examples

```
[1, 2, 3, 4, 5].drop(last: 2);  // ArraySlice[1, 2, 3]
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
mutating func ensureUnique()
```

_Defined in `lang/std/collections/slice.ks`._

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

#### function `first`

```kestrel
public func first(where: (T) -> Bool) -> T?
```

First element matching `predicate`, or `None`. O(n).

##### Examples

```
[1, 2, 3, 4, 5].first(where: { it > 3 });  // Some(4)
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

#### function `firstIndex`

```kestrel
public func firstIndex(of: T) -> Int64?
```

Index of the first element equal to `element`, or `None`. O(n).

##### Examples

```
[1, 2, 3, 2].firstIndex(of: 2);  // Some(1)
[1, 2, 3].firstIndex(of: 5);      // None
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

#### function `last`

```kestrel
public func last(where: (T) -> Bool) -> T?
```

Last element matching `predicate`, or `None`. O(n).

##### Examples

```
[1, 2, 3, 2, 1].last(where: { it > 1 });  // Some(2)
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

#### function `lastIndex`

```kestrel
public func lastIndex(of: T) -> Int64?
```

Index of the last element equal to `element`, or `None`. O(n).

##### Examples

```
[1, 2, 3, 2].lastIndex(of: 2);  // Some(3)
[1, 2, 3].lastIndex(of: 5);      // None
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

#### function `split`

```kestrel
public func split(T) -> ArraySplitView[T]
```

Multi-pass lazy view over the segments produced by splitting on
each occurrence of `separator`. Separators are dropped; empty
runs between adjacent separators are preserved.

Use `view.toArray()` to materialize all segments into an owned
`Array[ArraySlice[T]]`.

##### Examples

```
let v = [1, 0, 2, 0, 3].split(separator: 0);
for seg in v { ... }            // ArraySlice[1], ArraySlice[2], ArraySlice[3]
v.toArray();                     // eager: 3 segments

[1, 2, 3].split(separator: 0).toArray();
// [ArraySlice[1, 2, 3]] — separator not found, single segment
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

#### subscript `subscript`

```kestrel
public subscript[I](checked: I) -> I.SeqOutput? { get }
```

_Defined in `lang/std/collections/slice.ks`._

#### subscript `subscript`

```kestrel
public subscript[I](unchecked: I) -> I.SeqOutput { get set }
```

_Defined in `lang/std/collections/slice.ks`._

#### subscript `subscript`

```kestrel
public subscript[I](clamped: I) -> I.SeqClampedOutput { get set }
```

_Defined in `lang/std/collections/slice.ks`._

#### subscript `subscript`

```kestrel
public subscript[I](wrapped: I) -> I.SeqWrappedOutput { get set }
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
type Item
```

The element type that iteration yields.

_Defined in `lang/std/iter/iterator.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator
```

The concrete iterator type returned by `iter()`. Constrained so
`TargetIterator.Item` matches `Self.Item`.

_Defined in `lang/std/iter/iterator.ks`._

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

## struct `ValuesIterator`

```kestrel
public struct ValuesIterator[K, V] where K: Hashable { /* private fields */ }
```

Single-pass iterator yielding only the values of a dictionary.

Wraps a `DictionaryIterator[K, V]` and discards the key half of
each entry. Order matches the underlying entry iteration and is
unspecified.

### Examples

```
var it = ["a": 1, "b": 2].values.iter();
it.next();  // Some(1)  — order unspecified
it.next();  // Some(2)
it.next();  // None
```

### Representation

Wraps a `DictionaryIterator[K, V]`.

### Memory Model

Value type. Aliases dictionary storage; do not retain across
mutations.

_Defined in `lang/std/collections/dictionary.ks`._

### Members

#### initializer `From Dict`

```kestrel
public init(dictIter: DictionaryIterator[K, V])
```

Wraps a `DictionaryIterator` to yield only its values.

Prefer `ValuesView.iter()` over calling this directly.

_Defined in `lang/std/collections/dictionary.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = V
```

Element type yielded by `next()` — `V`.

_Defined in `lang/std/collections/dictionary.ks`._

#### function `next`

```kestrel
public mutating func next() -> V?
```

Returns the next value, or `None` when the underlying iterator
is exhausted.

##### Examples

```
var it = ["a": 1].values.iter();
it.next();  // Some(1)
it.next();  // None
```

_Defined in `lang/std/collections/dictionary.ks`._

## struct `ValuesView`

```kestrel
public struct ValuesView[K, V] where K: Hashable { /* private fields */ }
```

Lazy `Iterable` view over the values of a dictionary.

Returned by `Dictionary.values`. Constructing the view is O(1) —
it stores the bucket pointer and capacity. The view is invalidated
by any mutation that may reallocate.

### Examples

```
let dict = ["a": 1, "b": 2];
for v in dict.values { print(v) }
let sum = dict.values.iter().sum();
```

### Representation

`(buckets, capacity)` — a pointer into the source dictionary's
bucket array plus the total slot count.

### Memory Model

Value type that borrows the source dictionary's buffer.

_Defined in `lang/std/collections/dictionary.ks`._

### Members

#### initializer `From Buckets`

```kestrel
init(buckets: Pointer[Bucket[K, V]], capacity: Int64)
```

Internal — constructs a view from a bucket pointer and capacity.
Use `Dictionary.values` to obtain a view.

_Defined in `lang/std/collections/dictionary.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = V
```

`Iterable` element type — `V`.

_Defined in `lang/std/collections/dictionary.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = ValuesIterator[K, V]
```

Concrete iterator type returned by `iter()`.

_Defined in `lang/std/collections/dictionary.ks`._

#### function `iter`

```kestrel
public func iter() -> ValuesIterator[K, V]
```

Returns a fresh `ValuesIterator[K, V]` over the view.

Each call returns a new iterator starting at the beginning of
the bucket array.

_Defined in `lang/std/collections/dictionary.ks`._

## struct `WindowsIterator`

```kestrel
public struct WindowsIterator[T] { /* private fields */ }
```

_Defined in `lang/std/collections/views.ks`._

### Members

#### initializer `init`

```kestrel
public init(ptr: Pointer[T], totalCount: Int64, windowSize: Int64)
```

_Defined in `lang/std/collections/views.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = ArraySlice[T]
```

_Defined in `lang/std/collections/views.ks`._

#### function `next`

```kestrel
public mutating func next() -> Optional[ArraySlice[T]]
```

_Defined in `lang/std/collections/views.ks`._

## struct `WindowsView`

```kestrel
public struct WindowsView[T] { /* private fields */ }
```

Multi-pass lazy view over overlapping fixed-size sliding windows.

_Defined in `lang/std/collections/views.ks`._

### Members

#### field `count`

```kestrel
public var count: Int64 { get }
```

_Defined in `lang/std/collections/views.ks`._

#### field `first`

```kestrel
public var first: Optional[ArraySlice[T]] { get }
```

_Defined in `lang/std/collections/views.ks`._

#### initializer `init`

```kestrel
public init(slice: ArraySlice[T], windowSize: Int64)
```

_Defined in `lang/std/collections/views.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

_Defined in `lang/std/collections/views.ks`._

#### field `last`

```kestrel
public var last: Optional[ArraySlice[T]] { get }
```

_Defined in `lang/std/collections/views.ks`._

#### subscript `subscript`

```kestrel
public subscript(Int64) -> ArraySlice[T] { get }
```

_Defined in `lang/std/collections/views.ks`._

#### function `toArray`

```kestrel
public func toArray() -> Array[ArraySlice[T]]
```

_Defined in `lang/std/collections/views.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item = ArraySlice[T]
```

_Defined in `lang/std/collections/views.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = WindowsIterator[T]
```

_Defined in `lang/std/collections/views.ks`._

#### function `iter`

```kestrel
public func iter() -> WindowsIterator[T]
```

_Defined in `lang/std/collections/views.ks`._

