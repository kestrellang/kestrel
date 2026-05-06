# std.collections

## struct `Array`

```kestrel
public struct Array[T] { /* private fields */ }
```

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

#### subscript `Checked Index`

```kestrel
public subscript[I](checked: I) -> I.ArrayYield? { get }
```

Reads at `index`, returning `None` on out-of-bounds.

The non-panicking counterpart to `arr(i)`. Read-only; for fallible
writes pattern-match the result and assign through the default
subscript. Single-element indexes return `T?`; range indexes
return `Slice[T]?`. Prefer this when `index` may come from
untrusted input.

##### Examples

```
let arr = [10, 20, 30];
arr(checked: 0);       // Some(10)
arr(checked: 5);       // None
arr(checked: 0..<2);   // Some(Slice[10, 20])
arr(checked: 0..<10);  // None

if let .Some(v) = arr(checked: i) {
    // ...
}
```

_Defined in `lang/std/collections/array.ks`._

#### subscript `Clamping`

```kestrel
public subscript[I](clamped: I) -> I.ArrayClampedYield { get set }
```

Reads or writes at `index` with bounds saturated to `[0, count)`.

Never panics on out-of-bounds. For `Int64`, indices below `0`
clamp up and indices `>= count` clamp down; an empty array yields
`None`. For `Range[Int64]`, both endpoints clamp into `[0, count]`
and the result is a (possibly empty) `Slice[T]`. Compare
`arr(wrapped: i)`, which wraps instead of saturating.

##### Examples

```
let arr = [10, 20, 30];
arr(clamped: -5);          // Some(10) — clamped to first
arr(clamped: 100);         // Some(30) — clamped to last
arr(clamped: 1);           // Some(20) — in range
[](clamped: 0);            // None     — empty array

arr(clamped: -5..<100);    // Slice over the whole array
arr(clamped: -5..<1);      // Slice[10]
arr(clamped: 10..<20);     // empty Slice (both clamp to 3)
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
by following with `shrinkToFit()`. See also `appendFrom(iterable:)`
to add elements to an existing array.

##### Examples

```
let fromRange = Array(from: 1..<5);         // [1, 2, 3, 4]
let fromSet   = Array(from: mySet);         // arbitrary order
let collected = Array(from: lines.iter());  // exhausts the iterator
```

_Defined in `lang/std/collections/array.ks`._

#### subscript `Indexed`

```kestrel
public subscript[I](I) -> I.ArrayYield { get set }
```

Reads or writes at `index`, panicking on out-of-bounds.

The default subscript: trades safety for ergonomics. Dispatches via
the `ArrayIndex[T]` protocol — `Int64` reads/writes a single
element, `Range[Int64]` and `ClosedRange[Int64]` read or replace a
`Slice[T]`. Range writes require the source slice's length to
match the range's length and panic otherwise. Use
`arr(checked: i)` for an `Optional` instead of a panic, or
`arr(unchecked: i)` to skip the bounds check entirely. Setters
trigger COW; if storage is shared the buffer is cloned before the
write lands.

##### Errors

Panics with `"Array index out of bounds"` (Int64) or
`"Array range out of bounds"` (Range / ClosedRange) if the access
falls outside `[0, count)`. Range writes also panic if the source
slice's length doesn't match the range's length.

##### Examples

```
var arr = [10, 20, 30, 40, 50];
arr(0);                        // 10
arr(1) = 25;                   // [10, 25, 30, 40, 50]
arr(1..<4);                    // Slice[25, 30, 40]
arr(1..<4) = otherSlice3;      // splice in three elements
arr(5);                        // PANIC: index out of bounds
```

_Defined in `lang/std/collections/array.ks`._

#### initializer `Literal Bridge`

```kestrel
public init(_arrayLiteralPointer: lang.ptr[T], _arrayLiteralCount: lang.i64)
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

#### subscript `Unchecked Index`

```kestrel
public subscript[I](unchecked: I) -> I.ArrayYield { get set }
```

Reads or writes at `index` without a bounds check.

The fastest accessor; intended for hot loops where the index has
already been validated (e.g. inside `0..<count`). Setters trigger
COW. Range writes still panic on length mismatch — that's a
definitional check, not a bounds check.

##### Safety

Undefined behavior if the access is out of range. Always validate
before calling.

##### Examples

```
let arr = [10, 20, 30];
for i in arr.indices {
    let v = arr(unchecked: i);   // safe — i is in range
}
let s = arr(unchecked: 0..<2);   // Slice[10, 20]
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

#### subscript `Wrapping`

```kestrel
public subscript[I](wrapped: I) -> I.ArrayWrappedYield { get set }
```

Reads or writes at `index` using modulo-wrapping indexing.

Negative indices count from the end (`-1` is the last element);
positive indices `>= count` wrap to the start. The only failure
mode is an empty array, which yields `None` (and no-ops on the
setter). Compare `arr(clamped: i)`, which saturates instead of
wrapping.

##### Examples

```
let arr = [10, 20, 30];
arr(wrapped: -1);  // Some(30) — last element
arr(wrapped: -2);  // Some(20) — second to last
arr(wrapped:  3);  // Some(10) — wraps to index 0
arr(wrapped:  4);  // Some(20) — wraps to index 1
[](wrapped: 0);    // None     — empty array
```

_Defined in `lang/std/collections/array.ks`._

#### function `all`

```kestrel
public func all(where: (T) -> Bool) -> Bool
```

`true` when every element satisfies `predicate` (vacuously true
for an empty array).

Short-circuits on the first failure. The dual is
`any(where:)`.

##### Examples

```
[2, 4, 6].all(where: { (x) in x % 2 == 0 });  // true
[2, 3, 6].all(where: { (x) in x % 2 == 0 });  // false
[].all(where: { (x) in false });              // true (vacuous)
```

_Defined in `lang/std/collections/array.ks`._

#### function `any`

```kestrel
public func any(where: (T) -> Bool) -> Bool
```

`true` when at least one element satisfies `predicate` (always
`false` for an empty array).

Short-circuits on the first match. The dual is `all(where:)`.

##### Examples

```
[1, 2, 3].any(where: { (x) in x > 2 });  // true
[1, 2, 3].any(where: { (x) in x > 5 });  // false
[].any(where: { (x) in true });          // false (empty)
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
`appendFrom(iterable:)`.

##### Examples

```
var arr = [1, 2];
arr.append(3);  // [1, 2, 3]
```

_Defined in `lang/std/collections/array.ks`._

#### function `append`

```kestrel
public mutating func append(contentsOf: Array[T])
```

Appends every element of `other` to the end of this array.

Reserves the exact required capacity in one growth step then
copies the elements over, so it's faster than calling `append`
in a loop. Sharing semantics: `other` is read-only here, but if
`self` shares storage with anything else, COW fires once at the
start. See also `appendFrom(iterable:)` for arbitrary iterable
sources.

##### Examples

```
var arr = [1, 2];
arr.append(contentsOf: [3, 4]);  // [1, 2, 3, 4]
arr.append(contentsOf: []);      // [1, 2, 3, 4]  — no-op
```

_Defined in `lang/std/collections/array.ks`._

#### function `appendFrom`

```kestrel
public mutating func appendFrom[I](I) where I: Iterable, I.Item == T
```

Appends every element produced by an arbitrary iterable.

Drains the iterable via `append`, so capacity grows geometrically
rather than to an exact target — for sized sources like another
`Array`, prefer `append(contentsOf:)`.

##### Examples

```
var arr = [1, 2];
arr.appendFrom(iterable: 3..<6);  // [1, 2, 3, 4, 5]
```

_Defined in `lang/std/collections/array.ks`._

#### function `asPointer`

```kestrel
public func asPointer() -> Pointer[T]
```

Returns a raw pointer to the contiguous element buffer.

Intended for FFI or low-level memory work. Any operation that may
reallocate (`append`, `insert`, `reserveCapacity`, `shrinkToFit`,
or any mutation through a shared `Array` that triggers COW)
invalidates the pointer. For a higher-level borrowed view, use
`asSlice()`.

##### Safety

The pointer outlives the array no further than the next mutation.
Reading past `count` is undefined behavior; writing through the
pointer skips COW and may silently mutate other `Array` copies
that share the same storage.

##### Examples

```
let p = arr.asPointer();
c_sum(p, arr.count);   // pass to a C function
```

_Defined in `lang/std/collections/array.ks`._

#### function `asSlice`

```kestrel
public func asSlice() -> Slice[T]
```

Returns a `Slice[T]` over the entire array.

The slice borrows the array's buffer; reallocation invalidates
it. For a sub-range, use a range subscript such as `arr(0..<n)`.

##### Examples

```
let arr = [1, 2, 3];
let slice = arr.asSlice();  // Slice over [1, 2, 3]
```

_Defined in `lang/std/collections/array.ks`._

#### function `binarySearch`

```kestrel
public func binarySearch(T) -> Int64?
```

Returns the index of `element` via binary search, or `None`.

O(log n). When the array contains duplicates, *which* matching
index is returned is unspecified. For unsorted data use
`firstIndex(of:)` instead.

##### Safety

The array must be sorted in ascending order (per `isSorted()`).
Calling this on an unsorted array does not crash, but the result
is meaningless (false negatives become possible).

##### Examples

```
let arr = [1, 2, 3, 4, 5];
arr.binarySearch(element: 3);  // Some(2)
arr.binarySearch(element: 6);  // None
```

_Defined in `lang/std/collections/array.ks`._

#### field `capacity`

```kestrel
public var capacity: Int64 { get }
```

The number of elements the buffer can hold without reallocating.

Always `>= count`. When `append` would push `count` past
`capacity` the buffer doubles (or jumps from 0 to 4). Use
`reserveCapacity(...)` to pre-grow and `shrinkToFit()` to release
excess. The exact value after `init(capacity:)` may exceed the
requested amount because allocation rounds up.

##### Examples

```
let arr = Array[Int64](capacity: 10);
arr.capacity;  // >= 10
arr.count;     // 0
```

_Defined in `lang/std/collections/array.ks`._

#### function `chunks`

```kestrel
public func chunks(of: Int64) -> ChunksIterator[T]
```

Returns a `ChunksIterator[T]` over non-overlapping `size`-sized
`Slice[T]`s.

The final chunk may be shorter when `count` is not divisible by
`size`. For overlapping fixed-size views, use `windows(of:)`. The
produced iterator borrows the array's buffer.

##### Errors

Panics with `"Array.chunks: size must be positive"` if `size <= 0`.

##### Examples

```
let arr = [1, 2, 3, 4, 5];
for chunk in arr.chunks(of: 2) {
    // yields Slice[1,2], Slice[3,4], Slice[5]
}
arr.chunks(of: 0);  // PANIC
```

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

#### function `contains`

```kestrel
public func contains(T) -> Bool
```

`true` if the array contains an element equal to `element`.

Linear scan; short-circuits on the first match. For predicate-
based searching see `any(where:)` or `firstIndex(where:)`.

##### Examples

```
[1, 2, 3].contains(element: 2);  // true
[1, 2, 3].contains(element: 5);  // false
```

_Defined in `lang/std/collections/array.ks`._

#### field `count`

```kestrel
public var count: Int64 { get }
```

The number of elements currently in the array. Read-only; O(1).

Reflects only initialized elements, not capacity. To check
emptiness without comparing to zero, prefer `isEmpty`.

##### Examples

```
[1, 2, 3].count;  // 3
[].count;         // 0
```

_Defined in `lang/std/collections/array.ks`._

#### function `countItems`

```kestrel
public func countItems(where: (T) -> Bool) -> Int64
```

Returns the number of elements for which `predicate` is true.

Linear scan, no short-circuit. For just a presence check use
`any(where:)`; for a yes/no on every element,
`all(where:)`.

##### Examples

```
[1, 2, 3, 4, 5].countItems(where: { (x) in x % 2 == 0 });  // 2
[].countItems(where: { (x) in true });                     // 0
```

_Defined in `lang/std/collections/array.ks`._

#### function `dedup`

```kestrel
public mutating func dedup()
```

Removes runs of consecutive equal elements, in place.

Only adjacent duplicates collapse — non-adjacent equal values are
kept. To deduplicate globally, `sort()` first or, for `Hash`
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

#### function `drop`

```kestrel
public func drop(first: Int64) -> Slice[T]
```

Returns a `Slice[T]` with the first `count` elements skipped.

Complement of `prefix(count:)`. Borrows the array's buffer.

##### Errors

Panics with `"Array.drop(first:): count exceeds array length"` if
`count > self.count`.

##### Examples

```
[1, 2, 3, 4, 5].drop(first: 2);  // Slice[3, 4, 5]
[1, 2].drop(first: 2);           // empty Slice
```

_Defined in `lang/std/collections/array.ks`._

#### function `drop`

```kestrel
public func drop(last: Int64) -> Slice[T]
```

Returns a `Slice[T]` with the last `count` elements skipped.

Complement of `suffix(count:)`. Borrows the array's buffer.

##### Errors

Panics with `"Array.drop(last:): count exceeds array length"` if
`count > self.count`.

##### Examples

```
[1, 2, 3, 4, 5].drop(last: 2);  // Slice[1, 2, 3]
[1, 2].drop(last: 2);           // empty Slice
```

_Defined in `lang/std/collections/array.ks`._

#### function `ends`

```kestrel
public func ends(with: Array[T]) -> Bool
```

`true` if the array's trailing elements match `suffix` exactly.

An empty suffix always matches; a suffix longer than the array
never matches. Mirror of `starts(with:)`.

##### Examples

```
[1, 2, 3].ends(with: [2, 3]);  // true
[1, 2, 3].ends(with: [1, 2]);  // false
[1, 2, 3].ends(with: []);      // true (vacuous)
```

_Defined in `lang/std/collections/array.ks`._

#### function `first`

```kestrel
public func first() -> T?
```

Returns the first element, or `None` if the array is empty.

O(1). Read-only — to remove the first element use `popFirst()`.
To find the first element matching a predicate, see
`first(where:)`.

##### Examples

```
[1, 2, 3].first();  // Some(1)
[].first();         // None
```

_Defined in `lang/std/collections/array.ks`._

#### function `first`

```kestrel
public func first(where: (T) -> Bool) -> T?
```

Returns the first element matching `predicate`, or `None`.

Wraps `firstIndex(where:)` and reads the element at the
returned index. For just the index, use `firstIndex(where:)`.

##### Examples

```
let arr = [1, 2, 3, 4, 5];
arr.first(where: { (x) in x > 3 });   // Some(4)
arr.first(where: { (x) in x > 99 });  // None
```

_Defined in `lang/std/collections/array.ks`._

#### function `firstIndex`

```kestrel
public func firstIndex(where: (T) -> Bool) -> Int64?
```

Returns the index of the first element matching `predicate`, or
`None`.

Linear scan from the front; short-circuits on the first match.
To get the element instead of the index, use `first(where:)`.
For value-based search on `Equatable` arrays, use
`firstIndex(of:)`.

##### Examples

```
let arr = [1, 2, 3, 4, 5];
arr.firstIndex(where: { (x) in x > 3 });   // Some(3)
arr.firstIndex(where: { (x) in x > 10 });  // None
```

_Defined in `lang/std/collections/array.ks`._

#### function `firstIndex`

```kestrel
public func firstIndex(of: T) -> Int64?
```

Returns the index of the first element equal to `element`, or
`None`.

Wraps `firstIndex(where:)` with `equals(element)`. The mirror
is `lastIndex(of:)`.

##### Examples

```
[1, 2, 3, 2].firstIndex(of: 2);  // Some(1)
[1, 2, 3].firstIndex(of: 5);     // None
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

#### field `indices`

```kestrel
public var indices: Range[Int64] { get }
```

The valid index range `0..<count` as a `Range[Int64]`.

Convenient for index-based iteration or for passing to
`arr(range:)`. The range is empty for an empty array.

##### Examples

```
let arr = [10, 20, 30];
arr.indices;  // 0..<3

for i in arr.indices {
        print(arr(i));
}
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
`replaceSubrange(range: i..<i, with: ...)`.

##### Errors

Panics with `"Array.insert: index out of bounds"` if `index < 0`
or `index > count`.

##### Examples

```
var arr = [1, 3];
arr.insert(element: 2, at: 1);  // [1, 2, 3]
arr.insert(element: 0, at: 0);  // [0, 1, 2, 3]
arr.insert(element: 4, at: 4);  // [0, 1, 2, 3, 4]  — append-equivalent
arr.insert(element: 9, at: 99); // PANIC
```

_Defined in `lang/std/collections/array.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

`true` when the array has no elements; equivalent to `count == 0`.

Reads more naturally than the comparison and is preferred in
guards and predicates.

##### Examples

```
[].isEmpty;                // true
[1].isEmpty;               // false
Array[Int64]().isEmpty;    // true
```

_Defined in `lang/std/collections/array.ks`._

#### function `isSorted`

```kestrel
public func isSorted() -> Bool
```

`true` if the array is sorted in non-decreasing (ascending) order.

Equal adjacent elements are allowed. Empty and single-element
arrays are vacuously sorted. Useful as a precondition for
`binarySearch(element:)`.

##### Examples

```
[1, 2, 3].isSorted();  // true
[1, 3, 2].isSorted();  // false
[1, 1, 1].isSorted();  // true (equal adjacents allowed)
[].isSorted();         // true (vacuous)
```

_Defined in `lang/std/collections/array.ks`._

#### function `isValidIndex`

```kestrel
public func isValidIndex(Int64) -> Bool
```

`true` if `index` is in `[0, count)`.

Equivalent to `index >= 0 and index < count`. Pair with
`arr(unchecked: i)` to skip a redundant bounds check after you've
already validated the index.

##### Examples

```
let arr = [1, 2, 3];
arr.isValidIndex(index: 0);   // true
arr.isValidIndex(index: 2);   // true
arr.isValidIndex(index: 3);   // false
arr.isValidIndex(index: -1);  // false
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
[1, 2, 3].joined(separator: ", ");  // "1, 2, 3"
[1, 2, 3].joined();                 // "123"
["a", "b"].joined(separator: "-");  // "a-b"
[].joined(separator: ", ");         // ""
```

_Defined in `lang/std/collections/array.ks`._

#### function `last`

```kestrel
public func last() -> T?
```

Returns the last element, or `None` if the array is empty.

O(1). Read-only — to remove the last element use `pop()`. To find
the last element matching a predicate, see `last(where:)`.

##### Examples

```
[1, 2, 3].last();  // Some(3)
[].last();         // None
```

_Defined in `lang/std/collections/array.ks`._

#### function `last`

```kestrel
public func last(where: (T) -> Bool) -> T?
```

Returns the last element matching `predicate`, or `None`.

Wraps `lastIndex(where:)`. For just the index, use
`lastIndex(where:)`.

##### Examples

```
let arr = [1, 2, 3, 2, 1];
arr.last(where: { (x) in x > 1 });  // Some(2) — the second 2
```

_Defined in `lang/std/collections/array.ks`._

#### function `lastIndex`

```kestrel
public func lastIndex(where: (T) -> Bool) -> Int64?
```

Returns the index of the last element matching `predicate`, or
`None`.

Linear scan from the back; short-circuits on the first match. The
mirror of `firstIndex(where:)`. For value-based search on
`Equatable` arrays, use `lastIndex(of:)`.

##### Examples

```
let arr = [1, 2, 3, 2, 1];
arr.lastIndex(where: { (x) in x == 2 });   // Some(3)
arr.lastIndex(where: { (x) in x == 99 });  // None
```

_Defined in `lang/std/collections/array.ks`._

#### function `lastIndex`

```kestrel
public func lastIndex(of: T) -> Int64?
```

Returns the index of the last element equal to `element`, or
`None`.

Wraps `lastIndex(where:)` with `equals(element)`. The mirror
is `firstIndex(of:)`.

##### Examples

```
[1, 2, 3, 2].lastIndex(of: 2);  // Some(3)
[1, 2, 3].lastIndex(of: 5);     // None
```

_Defined in `lang/std/collections/array.ks`._

#### function `max`

```kestrel
public func max() -> T?
```

Returns the largest element, or `None` if the array is empty.

Single linear pass; ties go to the first occurrence. Pair with
`min()` for the lower bound.

##### Examples

```
[3, 1, 4].max();  // Some(4)
[].max();         // None
```

_Defined in `lang/std/collections/array.ks`._

#### function `min`

```kestrel
public func min() -> T?
```

Returns the smallest element, or `None` if the array is empty.

Single linear pass; ties go to the first occurrence. Pair with
`max()` for the upper bound.

##### Examples

```
[3, 1, 4].min();  // Some(1)
[].min();         // None
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

#### function `prefix`

```kestrel
public func prefix(Int64) -> Slice[T]
```

Returns a `Slice[T]` over the first `count` elements.

Borrows the array's buffer; reallocation invalidates it. Pair
with `drop(first:)` to get the complementary suffix. For the
trailing elements, see `suffix(count:)`.

##### Errors

Panics with `"Array.prefix: count exceeds array length"` if
`count > self.count`.

##### Examples

```
[1, 2, 3, 4, 5].prefix(count: 3);  // Slice[1, 2, 3]
[1, 2].prefix(count: 0);           // empty Slice
[1, 2].prefix(count: 9);           // PANIC
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
arr.remove(element: 2);  // true; arr is [1, 3, 2]
arr.remove(element: 5);  // false; arr unchanged
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
arr.removeAll(element: 2);  // [1, 3, 4]
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
public mutating func removeSubrange(Range[Int64])
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
arr.removeSubrange(range: 1..<4);  // arr is [1, 5]
arr.removeSubrange(range: 0..<0);  // no-op
```

_Defined in `lang/std/collections/array.ks`._

#### function `replaceSubrange`

```kestrel
public mutating func replaceSubrange(Range[Int64], with: Array[T])
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
arr.replaceSubrange(range: 1..<4, with: [20, 30]);    // [1, 20, 30, 5]
arr.replaceSubrange(range: 1..<1, with: [9, 9]);      // insert: [1, 9, 9, 20, 30, 5]
arr.replaceSubrange(range: 0..<2, with: Array[Int64]());  // remove: [9, 20, 30, 5]
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
arr.reserveCapacity(minimumCapacity: 1000);
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

#### function `reversed`

```kestrel
public func reversed() -> Array[T]
```

Returns a new array with the elements in reverse order.

Non-mutating. Internally clones via COW (cheap until the next
mutation) then `reverse()`s the copy. Use `reverse()` if you
don't need to keep the original ordering.

##### Examples

```
let arr = [1, 2, 3];
let rev = arr.reversed();  // [3, 2, 1]
// arr is still [1, 2, 3]
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
public mutating func shuffle[R](using: R) where R: RandomNumberGenerator
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
public func shuffled[R](using: R) -> Array[T] where R: RandomNumberGenerator
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

Stable insertion sort under the hood (see the custom-comparator
`sort(by:)` for the algorithm). For descending or custom orderings
pass a comparator to `sort(by:)`. Non-mutating variant: `sorted()`.

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
before the second. Uses insertion sort — O(n²) worst-case but
stable and excellent for small or nearly-sorted inputs. Pass a
reversed comparator for descending order.

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

Equivalent to `sort(by: { (a, b) in key(a) < key(b) })`. The key
closure runs O(n²) times in the worst case (insertion sort), so
keep it cheap. For descending order, pass a comparator to
`sort(by:)` instead.

##### Examples

```
var people = [Person("Alice", 30), Person("Bob", 25)];
people.sort(byKey: { (p) in p.age });  // sorted by age ascending
```

_Defined in `lang/std/collections/array.ks`._

#### function `sorted`

```kestrel
public func sorted() -> Array[T]
```

Returns a new array sorted in ascending order; original unchanged.

Non-mutating mirror of `sort()`. Internally clones via COW then
sorts the copy.

##### Examples

```
let arr = [3, 1, 4, 1, 5];
let sorted = arr.sorted();  // [1, 1, 3, 4, 5]
// arr is still [3, 1, 4, 1, 5]
```

_Defined in `lang/std/collections/array.ks`._

#### function `sorted`

```kestrel
public func sorted(by: (T, T) -> Bool) -> Array[T]
```

Returns a new array sorted by a custom comparator. Original
unchanged.

Non-mutating mirror of `sort(by:)`. Useful for one-shot orderings
such as case-insensitive string sorts.

##### Examples

```
let arr = ["apple", "Banana", "cherry"];
let sorted = arr.sorted(by: { (a, b) in a.lowercase() < b.lowercase() });
```

_Defined in `lang/std/collections/array.ks`._

#### function `sorted`

```kestrel
public func sorted[K](byKey: (T) -> K) -> Array[T] where K: Comparable
```

Returns a new array sorted by an extracted `Comparable` key;
original unchanged.

Non-mutating mirror of `sort(byKey:)`.

##### Examples

```
let words = ["hi", "hello", "hey"];
let byLength = words.sorted(byKey: { (w) in w.count });  // ["hi", "hey", "hello"]
```

_Defined in `lang/std/collections/array.ks`._

#### function `split`

```kestrel
public func split(T) -> Array[Slice[T]]
```

Splits the array on each element equal to `separator`, returning
the in-between runs as `Slice[T]`s.

Separators themselves are dropped, but empty runs (between
adjacent separators, or before the first / after the last) are
preserved as empty slices. The result therefore always has length
`(separatorCount + 1)`. The slices alias the source buffer.

##### Examples

```
[1, 0, 2, 0, 3].split(separator: 0);
// [Slice[1], Slice[2], Slice[3]]

[0, 1, 0, 0, 2, 0].split(separator: 0);
// [Slice[], Slice[1], Slice[], Slice[2], Slice[]]

[1, 2, 3].split(separator: 0);
// [Slice[1, 2, 3]] — separator not found

[].split(separator: 0);
// [Slice[]] — empty array yields one empty slice
```

_Defined in `lang/std/collections/array.ks`._

#### function `starts`

```kestrel
public func starts(with: Array[T]) -> Bool
```

`true` if the array's leading elements match `prefix` exactly.

An empty prefix always matches; a prefix longer than the array
never matches. Mirror of `ends(with:)`.

##### Examples

```
[1, 2, 3].starts(with: [1, 2]);     // true
[1, 2, 3].starts(with: [1, 2, 3]);  // true (full match)
[1, 2, 3].starts(with: [2, 3]);     // false
[1, 2].starts(with: [1, 2, 3]);     // false (prefix longer)
[1, 2, 3].starts(with: []);         // true (vacuous)
```

_Defined in `lang/std/collections/array.ks`._

#### function `suffix`

```kestrel
public func suffix(Int64) -> Slice[T]
```

Returns a `Slice[T]` over the last `count` elements.

Mirror of `prefix(count:)`. Borrows the array's buffer.

##### Errors

Panics with `"Array.suffix: count exceeds array length"` if
`count > self.count`.

##### Examples

```
[1, 2, 3, 4, 5].suffix(count: 2);  // Slice[4, 5]
[1, 2].suffix(count: 0);           // empty Slice
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

#### function `unique`

```kestrel
public func unique() -> Array[T]
```

Returns a new array containing each distinct element once, in the
order of first occurrence.

Currently O(n²) (linear scan per insert). For an O(n) build, push
the elements through a `Set` first. The in-place mirror is
`removeDuplicates()`. Compare with `dedup()`, which only collapses
adjacent duplicates and does not require `Hash`.

##### Examples

```
[1, 2, 1, 3, 2, 4].unique();  // [1, 2, 3, 4]
["a", "a", "b"].unique();      // ["a", "b"]
```

_Defined in `lang/std/collections/array.ks`._

#### function `windows`

```kestrel
public func windows(of: Int64) -> WindowsIterator[T]
```

Returns a `WindowsIterator[T]` over overlapping `size`-sized
`Slice[T]`s.

Adjacent windows overlap by `size - 1` elements. For
non-overlapping fixed-size groups, use `chunks(of:)`. The
produced iterator borrows the array's buffer.

##### Errors

Panics with `"Array.windows: size must be positive"` if
`size <= 0`, or `"Array.windows: size exceeds array length"` if
`size > count`.

##### Examples

```
let arr = [1, 2, 3, 4];
for window in arr.windows(of: 2) {
    // yields Slice[1,2], Slice[2,3], Slice[3,4]
}
[1, 2].windows(of: 5);  // PANIC: size exceeds array length
```

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
type TargetIterator = ArrayIterator[T]
```

`Iterable` iterator type — the concrete iterator returned by `iter()`.

_Defined in `lang/std/collections/array.ks`._

#### function `iter`

```kestrel
public func iter() -> ArrayIterator[T]
```

Returns a forward iterator over the array's elements.

The returned `ArrayIterator[T]` aliases the array's buffer; do
not mutate the array while iterating. For grouped views see
`chunks(of:)` and `windows(of:)`.

##### Examples

```
for item in arr.iter() { ... }
let doubled = arr.iter().map({ (x) in x * 2 }).collect();
```

_Defined in `lang/std/collections/array.ks`._

### Implements `ExpressibleByArrayLiteral`

#### initializer `Array Literal`

```kestrel
init(LiteralSlice[Element])
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
init(_arrayLiteralPointer: lang.ptr[Element], _arrayLiteralCount: lang.i64)
```

Compiler-emitted init taking a raw pointer and count.

_Defined in `lang/std/core/literals.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> Array[T]
```

Returns an `Array[T]` sharing the same storage; the deep copy is
deferred until either side mutates.

O(1) — just bumps the storage `RcBox`'s refcount. The first
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

#### function `equals`

```kestrel
public func equals(Array[T]) -> Bool
```

Element-wise equality: arrays are equal iff they have the same
`count` and every corresponding pair of elements is equal.

Short-circuits on the first mismatch. Order matters —
`[1, 2, 3]` is not equal to `[3, 2, 1]`.

##### Examples

```
[1, 2, 3].equals(other: [1, 2, 3]);  // true
[1, 2, 3].equals(other: [1, 2]);     // false
[1, 2, 3].equals(other: [3, 2, 1]);  // false
```

_Defined in `lang/std/collections/array.ks`._

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
public func matchSlice(Int64, Int64) -> Slice[T]
```

Pattern-matcher hook returning the half-open `[from, to)` slice.

Used to bind `..rest` segments. The matcher guarantees the
indices are in range.

_Defined in `lang/std/collections/array.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(FormatOptions) -> String
```

Renders the array as `"[" + elements.joined(", ") + "]"`, passing
`options` through to each element's `format`.

Empty arrays render as `"[]"`.

##### Examples

```
[1, 2, 3].format();         // "[1, 2, 3]"
Array[Int64]().format();    // "[]"
"\{[1, 2, 3]}";             // "[1, 2, 3]" (via interpolation)
```

_Defined in `lang/std/collections/array.ks`._

## struct `ArrayIterator`

```kestrel
public struct ArrayIterator[T] { /* private fields */ }
```

Single-pass forward iterator over the elements of an `Array[T]`.

Produced by `Array.iter()`, walks the underlying storage one element at a
time and yields owned copies of each element. The iterator holds a raw
pointer into the array's buffer, so any mutation of the source array
(which may reallocate) invalidates iteration. Use `chunks(of:)` or
`windows(of:)` if you need grouped views instead.

### Examples

```
let arr = [1, 2, 3];
var it = arr.iter();
it.next();  // Some(1)
it.next();  // Some(2)
it.next();  // Some(3)
it.next();  // None
```

### Representation

A `(ptr, remaining)` pair: a `Pointer[T]` advanced on each call and an
`Int64` count of remaining elements.

### Memory Model

Value type. The pointer aliases array storage; do not retain an iterator
across mutations of the source array.

_Defined in `lang/std/collections/array.ks`._

### Members

#### initializer `From Pointer`

```kestrel
public init(ptr: Pointer[T], remaining: Int64)
```

Constructs an iterator from a raw pointer and a remaining-count.

Normally you should not call this directly — use `Array.iter()` instead.
The pointer must be valid for `remaining` reads of `T`.

##### Safety

The caller must guarantee `ptr` points to at least `remaining`
initialized elements of `T` and remains valid for the iterator's
lifetime.

##### Examples

```
let it = ArrayIterator(ptr: arr.asPointer(), remaining: arr.count);
```

_Defined in `lang/std/collections/array.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = T
```

The element type yielded by `next()` — always `T`.

_Defined in `lang/std/collections/array.ks`._

#### function `next`

```kestrel
public mutating func next() -> T?
```

Advances the iterator and returns the next element, or `None` when the
iterator is exhausted.

Each call reads one element, advances the internal pointer by one,
and decrements the remaining count. Once `None` is returned the
iterator stays exhausted.

##### Examples

```
var it = [10, 20].iter();
it.next();  // Some(10)
it.next();  // Some(20)
it.next();  // None
```

_Defined in `lang/std/collections/array.ks`._

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

Iterator over non-overlapping `Slice[T]` chunks of an `Array[T]`.

Produced by `Array.chunks(of:)`, walks the source buffer in fixed-size
strides and yields each chunk as a borrowed `Slice[T]`. The last chunk
may be shorter than `chunkSize` when the array length is not evenly
divisible. For overlapping windows of a fixed size instead, use
`WindowsIterator` / `Array.windows(of:)`.

### Examples

```
let arr = [1, 2, 3, 4, 5];
for chunk in arr.chunks(of: 2) {
    // yields: Slice[1, 2], Slice[3, 4], Slice[5]
}
```

### Representation

A `(ptr, remaining, chunkSize)` triple: a pointer advanced by one chunk
per `next()` call, plus the count of unread elements and the requested
stride.

### Memory Model

Value type. Yielded slices alias the source array's buffer; do not
retain them across mutations of the array.

_Defined in `lang/std/collections/array.ks`._

### Members

#### initializer `From Pointer`

```kestrel
public init(ptr: Pointer[T], remaining: Int64, chunkSize: Int64)
```

Constructs a chunks iterator from a pointer, total element count, and
chunk stride.

Prefer `Array.chunks(of:)` over calling this directly.

##### Safety

`ptr` must point to at least `remaining` initialized elements of
`T`, and `chunkSize` should be positive.

##### Examples

```
let it = ChunksIterator(ptr: arr.asPointer(), remaining: arr.count, chunkSize: 2);
```

_Defined in `lang/std/collections/array.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = Slice[T]
```

The element type yielded by `next()` — a borrowed `Slice[T]` over
one chunk.

_Defined in `lang/std/collections/array.ks`._

#### function `next`

```kestrel
public mutating func next() -> Slice[T]?
```

Returns the next chunk, or `None` when the source is exhausted.

The returned `Slice[T]` has length `chunkSize`, except for the final
chunk which may be shorter if the total count was not evenly
divisible.

##### Examples

```
var it = [1, 2, 3, 4, 5].chunks(of: 2);
it.next();  // Some(Slice[1, 2])
it.next();  // Some(Slice[3, 4])
it.next();  // Some(Slice[5])     // shorter trailing chunk
it.next();  // None
```

_Defined in `lang/std/collections/array.ks`._

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
public mutating func write(Slice[UInt8])
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

## struct `Dictionary`

```kestrel
public struct Dictionary[K, V, H = DefaultHasher] where K: Hash, H: Hasher, H: Defaultable { /* private fields */ }
```

An unordered hash map keyed by any `K: Hash`, parameterized over the
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

- Every key satisfies `K: Hash`. The cached hash is computed once
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
let grouped = Dictionary(grouping: words, by: { (w) in w.chars.first().unwrap() });
// ["a": ["apple", "apricot"], "b": ["banana", "blueberry"]]

let nums = [1, 2, 3, 4, 5];
let parity = Dictionary(grouping: nums, by: { (n) in n % 2 });
// [0: [2, 4], 1: [1, 3, 5]]
```

_Defined in `lang/std/collections/dictionary.ks`._

#### initializer `Literal Bridge`

```kestrel
public init(lang.ptr[(K, V)], lang.i64)
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
["a": 2, "b": 4].all(where: { (k, v) in v % 2 == 0 });  // true
["a": 1, "b": 2].all(where: { (k, v) in v % 2 == 0 });  // false
[:].all(where: { (k, v) in false });                    // true (vacuous)
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
["a": 1, "b": 5].any(where: { (k, v) in v > 3 });  // true
[:].any(where: { (k, v) in true });                // false (empty)
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
let parsed = dict.compactMapValues(transform: { (s) in Int64.parse(s) });
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
["a": 1, "b": 2].contains(key: "a");  // true
["a": 1, "b": 2].contains(key: "z");  // false
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
["a": 1, "b": 5].contains(where: { (k, v) in v > 3 });  // true
["a": 1, "b": 2].contains(where: { (k, v) in v > 3 });  // false
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
["a": 1, "b": 2].containsValue(value: 2);  // true
["a": 1, "b": 2].containsValue(value: 5);  // false
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
["a": 1, "b": 2, "c": 3].countItems(where: { (k, v) in v > 1 });  // 2
[:].countItems(where: { (k, v) in true });                        // 0
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
let big = dict.filter(where: { (k, v) in v > 1 });  // ["b": 2, "c": 3]
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
dict.first(where: { (k, v) in v > 2 });  // Some entry with v > 2
dict.first(where: { (k, v) in v > 99 }); // None
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
dict.insert(key: "b", value: 2);  // None;     dict = ["a": 1, "b": 2]
dict.insert(key: "a", value: 9);  // Some(1);  dict = ["a": 9, "b": 2]
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
let doubled = dict.mapValues(transform: { (v) in v * 2 });
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
a.merge(b, uniquingKeysWith: { (old, new) in old + new });
// a == ["x": 1, "y": 22, "z": 30]
```

_Defined in `lang/std/collections/dictionary.ks`._

#### function `mergeFrom`

```kestrel
public mutating func mergeFrom[I](I, uniquingKeysWith: (V, V) -> V) where I: Iterable, I.Item == (K, V)
```

Merges every `(key, value)` pair from an arbitrary iterable into
`self`, calling `combine` on collisions.

Same semantics as `merge(...)` but accepts any iterable of
pairs — useful for arrays of tuples, generator output, or
streamed sources.

##### Examples

```
var dict = ["a": 1];
dict.mergeFrom(pairs: [("b", 2), ("c", 3)], uniquingKeysWith: { (_, new) in new });
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
let merged = a.merging(other: b, uniquingKeysWith: { (_, new) in new });
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
dict.remove(key: "a");  // Some(1); dict = ["b": 2]
dict.remove(key: "z");  // None;    dict unchanged
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
dict.removeAll(where: { (k, v) in v < 2 });  // ["b": 2, "c": 3]
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
dict.reserveCapacity(minimumCapacity: 1000);
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
dict.retain(where: { (k, v) in v > 1 });  // ["b": 2, "c": 3]
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
dict.update(key: "a", with: { (v) in v * 10 });  // true;  dict("a") == Some(10)
dict.update(key: "z", with: { (v) in v * 10 });  // false; dict unchanged
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
counts.upsert(key: "apple", default: 0, with: { (n) in n + 1 });
counts.upsert(key: "apple", default: 0, with: { (n) in n + 1 });
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

#### function `equals`

```kestrel
public func equals(Dictionary[K, V, H]) -> Bool
```

Order-independent equality: dictionaries are equal iff they have
the same `count` and every key in `self` is present in `other`
with an equal value.

Short-circuits on the first mismatch. Insertion order does not
matter — only the multiset of `(key, value)` pairs does.

##### Examples

```
["a": 1, "b": 2].equals(other: ["b": 2, "a": 1]);  // true
["a": 1].equals(other: ["a": 2]);                  // false
["a": 1].equals(other: [:]);                       // false
```

_Defined in `lang/std/collections/dictionary.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(FormatOptions) -> String
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

Key type for the literal protocol — equals `K`.

_Defined in `lang/std/collections/dictionary.ks`._

#### initializer `Literal Bridge`

```kestrel
init(lang.ptr[(Key, Value)], lang.i64)
```

Compiler-emitted init taking a raw `(Key, Value)` pointer and count.

_Defined in `lang/std/core/literals.ks`._

#### typealias `Value`

```kestrel
type Value = V
```

Value type for the literal protocol — equals `V`.

_Defined in `lang/std/collections/dictionary.ks`._

### Implements `ExpressibleByDictionaryLiteral`

#### initializer `Dictionary Literal`

```kestrel
init(LiteralSlice[(Key, Value)])
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

## struct `KeysIterator`

```kestrel
public struct KeysIterator[K, V] where K: Hash { /* private fields */ }
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
public struct KeysView[K, V] where K: Hash { /* private fields */ }
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

## struct `Set`

```kestrel
public struct Set[T, H = DefaultHasher] where T: Hash, H: Hasher, H: Defaultable { /* private fields */ }
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
a.union(other: b);          // {1, 2, 3, 4, 5}
a.intersection(other: b);   // {3}
a.isSubset(of: b);          // false
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

Each element's hash is computed via `T: Hash` and stored in the
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

- Elements are unique by `Hash`/`Equatable` equality.
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
public init(_arrayLiteralPointer: lang.ptr[T], _arrayLiteralCount: lang.i64)
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
`any(where:)`.

##### Examples

```
Set([2, 4, 6]).all(where: { (x) in x % 2 == 0 });  // true
Set([1, 2, 4]).all(where: { (x) in x % 2 == 0 });  // false
Set[Int64]().all(where: { (x) in false });         // true (vacuous)
```

_Defined in `lang/std/collections/set.ks`._

#### function `any`

```kestrel
public func any(where: (T) -> Bool) -> Bool
```

`true` when at least one element satisfies `predicate`.

Alias for `contains(where:)` — both names exist so
predicate-style code reads naturally regardless of context.
Short-circuits.

##### Examples

```
Set([1, 2, 3]).any(where: { (x) in x > 2 });  // true
Set[Int64]().any(where: { (x) in true });     // false (empty)
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
public func compactMap[U]((T) -> U?) -> Set[U, H] where U: Hash
```

Returns a new set with each element run through `transform`,
dropping any `None` results.

Useful for parse-or-skip patterns. Same uniqueness caveat as
`map(transform:)` — collisions in the transformed values
collapse.

##### Examples

```
let set: Set = ["1", "two", "3"];
let nums = set.compactMap(transform: { (s) in Int64.parse(s) });
// {1, 3}  — "two" failed to parse
```

_Defined in `lang/std/collections/set.ks`._

#### function `contains`

```kestrel
public func contains(T) -> Bool
```

`true` if `element` is a member of the set; O(1) average.

Forwards to the dictionary's key lookup. For predicate-based
search use `contains(where:)`.

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
empty sets. The aliased shape `any(where:)` exists for
symmetry with `Array`.

##### Examples

```
Set([1, 2, 3]).contains(where: { (x) in x > 2 });  // true
Set([1, 2, 3]).contains(where: { (x) in x > 5 });  // false
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
`any(where:)`; for a yes/no on every element,
`all(where:)`.

##### Examples

```
Set([1, 2, 3, 4, 5]).countItems(where: { (x) in x % 2 == 0 });  // 2
Set[Int64]().countItems(where: { (x) in true });                // 0
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

Non-mutating mirror of `formDifference(other:)`. Order of
arguments matters: `a.difference(b)` is generally not equal
to `b.difference(a)`.

##### Examples

```
let a: Set = [1, 2, 3];
let b: Set = [2, 3, 4];
a.difference(other: b);  // {1}
b.difference(other: a);  // {4}
```

_Defined in `lang/std/collections/set.ks`._

#### function `filter`

```kestrel
public func filter(where: (T) -> Bool) -> Set[T, H]
```

Returns a new set containing only elements for which
`predicate` is true.

Non-mutating mirror of `retain(where:)`. Allocates a fresh
set; for in-place filtering use `retain` or
`removeAll(where:)`.

##### Examples

```
let set: Set = [1, 2, 3, 4, 5];
let evens = set.filter(where: { (x) in x % 2 == 0 });  // {2, 4}
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
set.first(where: { (x) in x > 3 });   // Some(4) or Some(5)
set.first(where: { (x) in x > 99 });  // None
```

_Defined in `lang/std/collections/set.ks`._

#### function `flatMap`

```kestrel
public func flatMap[U]((T) -> Set[U, H]) -> Set[U, H] where U: Hash
```

Returns a new set formed by unioning every set produced by
`transform`.

Each element maps to a `Set[U, H]`; those sets are merged
together. The result holds the unique union — duplicates
across sub-sets collapse, as with all set operations.

##### Examples

```
let set: Set = [1, 2];
let expanded = set.flatMap(transform: { (x) in Set([x, x * 10]) });
// {1, 10, 2, 20}
```

_Defined in `lang/std/collections/set.ks`._

#### function `formDifference`

```kestrel
public mutating func formDifference(Set[T, H])
```

In-place difference: removes every element of `self` that **is**
in `other`.

Mutating mirror of `difference(other:)`. The result is "self
minus other".

##### Examples

```
var a: Set = [1, 2, 3];
let b: Set = [2, 3, 4];
a.formDifference(other: b);  // a == {1}
```

_Defined in `lang/std/collections/set.ks`._

#### function `formIntersection`

```kestrel
public mutating func formIntersection(Set[T, H])
```

In-place intersection: removes every element of `self` that
is **not** in `other`.

Mutating mirror of `intersection(other:)`. Iterates over
`self`, so the cost scales with `self.count`, not
`other.count`.

##### Examples

```
var a: Set = [1, 2, 3];
let b: Set = [2, 3, 4];
a.formIntersection(other: b);  // a == {2, 3}
```

_Defined in `lang/std/collections/set.ks`._

#### function `formSymmetricDifference`

```kestrel
public mutating func formSymmetricDifference(Set[T, H])
```

In-place symmetric difference: keeps elements in exactly one
of `self` or `other`.

Mutating mirror of `symmetricDifference(other:)`. Two passes:
removes shared elements, then inserts elements unique to
`other`.

##### Examples

```
var a: Set = [1, 2, 3];
let b: Set = [2, 3, 4];
a.formSymmetricDifference(other: b);  // a == {1, 4}
```

_Defined in `lang/std/collections/set.ks`._

#### function `formUnion`

```kestrel
public mutating func formUnion(Set[T, H])
```

In-place union: adds every element of `other` to `self`.

Mutating mirror of `union(other:)`. For multi-source unions,
chain calls or use `insert(contentsOf:)` over the elements.

##### Examples

```
var a: Set = [1, 2];
let b: Set = [2, 3];
a.formUnion(other: b);  // a == {1, 2, 3}
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
prefer `formUnion(other:)` — it's the same semantically but
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

Non-mutating mirror of `formIntersection(other:)`. For
efficiency, iterates over `self`; pass the smaller set as the
receiver if it matters.

##### Examples

```
let a: Set = [1, 2, 3];
let b: Set = [2, 3, 4];
a.intersection(other: b);  // {2, 3}
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
public func map[U]((T) -> U) -> Set[U, H] where U: Hash
```

Returns a new set with each element run through `transform`.

**Cardinality may shrink**: if `transform` maps two distinct
elements to the same output, the result holds only one copy
(sets are unique). For an `Optional`-aware variant that drops
`None`, use `compactMap(transform:)`.

##### Examples

```
let set: Set = [1, 2, 3];
let doubled = set.map(transform: { (x) in x * 2 });
// {2, 4, 6}

let words: Set = ["Hello", "WORLD"];
let lower = words.map(transform: { (s) in s.lowercase() });
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

Inverse of `retain(where:)`. Same two-pass structure.

##### Examples

```
var set: Set = [1, 2, 3, 4, 5];
set.removeAll(where: { (x) in x % 2 == 0 });  // {1, 3, 5}
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
anyway). Mirror is `removeAll(where:)`.

##### Examples

```
var set: Set = [1, 2, 3, 4, 5];
set.retain(where: { (x) in x % 2 == 0 });  // {2, 4}
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

Non-mutating mirror of `formSymmetricDifference(other:)`.
Equivalent to `union(other:) - intersection(other:)`. The
operation is commutative — order of arguments doesn't change
the result.

##### Examples

```
let a: Set = [1, 2, 3];
let b: Set = [2, 3, 4];
a.symmetricDifference(other: b);  // {1, 4}
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

Non-mutating mirror of `formUnion(other:)`. Internally clones
`self` (cheap COW) and adds `other` into the copy.

##### Examples

```
let a: Set = [1, 2, 3];
let b: Set = [3, 4, 5];
a.union(other: b);  // {1, 2, 3, 4, 5}
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

#### function `equals`

```kestrel
public func equals(Set[T, H]) -> Bool
```

`true` when `self` and `other` contain exactly the same
elements.

Order-independent (sets are unordered). Implemented as
"equal counts and `self.isSubset(of: other)`" — short-circuits
at the count check.

##### Examples

```
Set([1, 2, 3]).equals(other: Set([3, 2, 1]));  // true
Set([1, 2]).equals(other: Set([1, 2, 3]));     // false
```

_Defined in `lang/std/collections/set.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(FormatOptions) -> String
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
init(LiteralSlice[Element])
```

Builds an instance from a literal slice of elements.

_Defined in `lang/std/core/literals.ks`._

## struct `SetIterator`

```kestrel
public struct SetIterator[T, H = DefaultHasher] where T: Hash, H: Hasher, H: Defaultable { /* private fields */ }
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

## struct `ValuesIterator`

```kestrel
public struct ValuesIterator[K, V] where K: Hash { /* private fields */ }
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
public struct ValuesView[K, V] where K: Hash { /* private fields */ }
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

Iterator over overlapping fixed-size sliding windows of an `Array[T]`.

Produced by `Array.windows(of:)`. Every yielded window has exactly
`windowSize` elements; the pointer advances by one element per step, so
adjacent windows overlap by `windowSize - 1` elements. If the array is
shorter than the window size, no windows are yielded. For
non-overlapping fixed-size groups, use `ChunksIterator` instead.

### Examples

```
let arr = [1, 2, 3, 4];
for window in arr.windows(of: 2) {
    // yields: Slice[1, 2], Slice[2, 3], Slice[3, 4]
}
```

### Representation

A `(ptr, remaining, windowSize)` triple. `remaining` is precomputed at
construction as `max(totalCount - windowSize + 1, 0)`.

### Memory Model

Value type. Yielded slices alias the source array's buffer; do not
retain them across mutations of the array.

_Defined in `lang/std/collections/array.ks`._

### Members

#### initializer `From Pointer`

```kestrel
public init(ptr: Pointer[T], totalCount: Int64, windowSize: Int64)
```

Constructs a windows iterator from a pointer, total element count,
and window size.

Prefer `Array.windows(of:)` over calling this directly. The window
count is derived as `max(totalCount - windowSize + 1, 0)`, so a
`windowSize` larger than `totalCount` yields nothing.

##### Safety

`ptr` must point to at least `totalCount` initialized elements of
`T` and remain valid for the iterator's lifetime.

##### Examples

```
let it = WindowsIterator(ptr: arr.asPointer(), totalCount: arr.count, windowSize: 2);
```

_Defined in `lang/std/collections/array.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = Slice[T]
```

The element type yielded by `next()` — a borrowed `Slice[T]` over
one window.

_Defined in `lang/std/collections/array.ks`._

#### function `next`

```kestrel
public mutating func next() -> Slice[T]?
```

Returns the next window, or `None` when no more full windows fit.

Each call slides the pointer forward by one element, so consecutive
windows share `windowSize - 1` elements.

##### Examples

```
var it = [1, 2, 3].windows(of: 2);
it.next();  // Some(Slice[1, 2])
it.next();  // Some(Slice[2, 3])
it.next();  // None
```

_Defined in `lang/std/collections/array.ks`._

