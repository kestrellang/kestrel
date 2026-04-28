# std.iter

## struct `ChainIterator`

```kestrel
public struct ChainIterator[A, B] where A: Iterator, B: Iterator, B.Item == A.Item { /* private fields */ }
```

Lazy `chain` — yields all of `first`, then all of `second`. Returned
by `Iterator.chain(other:)`.

### Representation

Both source iterators + a one-bit `firstDone` flag that latches when
the first iterator runs out.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Sources`

```kestrel
public init(first: A, second: B)
```

Builds a `ChainIterator`. Prefer `first.chain(other: second)`.

_Defined in `lang/std/iter/adapters.ks`._

#### field `first`

```kestrel
internal var first: A
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `firstDone`

```kestrel
internal var firstDone: Bool
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `second`

```kestrel
internal var second: B
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = A.Item
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> A.Item?
```

Pulls from `first` until it's empty, then forwards to `second`.

_Defined in `lang/std/iter/adapters.ks`._

## struct `CycleIterator`

```kestrel
public struct CycleIterator[I] where I: Iterator { /* private fields */ }
```

Repeats a finite iterator forever by copying it on each lap. Returned
by `Iterator.cycle()`.

### Representation

Two copies of the source: `original` (immutable template) and
`current` (the working iterator). When `current` exhausts, it is
reset from `original`.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(iter: I)
```

Builds a `CycleIterator` that will replay `iter` forever.

_Defined in `lang/std/iter/adapters.ks`._

#### field `current`

```kestrel
internal var current: I
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `original`

```kestrel
internal var original: I
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = I.Item
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> I.Item?
```

Pulls the current lap; on exhaustion, restarts and pulls again.

_Defined in `lang/std/iter/adapters.ks`._

## protocol `DoubleEndedIterator`

```kestrel
public protocol DoubleEndedIterator
```

An iterator that can also yield from the back. Powers `rev()` and
efficient "last N elements" patterns without first materialising the
whole sequence.

Front and back iteration share state — alternating `next()` and
`nextBack()` is well-defined and meets in the middle.

### Examples

```
// Defining a double-ended range
struct Range: DoubleEndedIterator {
    type Item = Int64;
    var start: Int64;
    var end: Int64;

    mutating func next() -> Int64? {
        if start >= end { return None };
        let v = start;
        start += 1;
        .Some(v)
    }

    mutating func nextBack() -> Int64? {
        if start >= end { return None };
        end -= 1;
        .Some(end)
    }
}

var r = Range(start: 1, end: 4);
r.next();      // Some(1)
r.nextBack();  // Some(3)
r.next();      // Some(2)
r.nextBack();  // None  (start >= end)
```

_Defined in `lang/std/iter/iterator.ks`._

### Members

#### function `nextBack`

```kestrel
mutating func nextBack() -> Item?
```

Yields the next element from the back, or `None` if the front and
back have met. Can be interleaved freely with `next()`.

_Defined in `lang/std/iter/iterator.ks`._

#### function `rev`

```kestrel
public func rev() -> ReversedIterator[Self]
```

Yields elements back-to-front by pulling `nextBack()` instead of
`next()`. `O(1)` to construct — no buffering.

##### Examples

```
[1, 2, 3, 4, 5].iter().rev().collect();                        // [5, 4, 3, 2, 1]
[1, 2, 3, 4, 5].iter().rev().take(count: 3).collect();         // [5, 4, 3]
[1, 2, 3, 4, 5].iter().rev().first(matching: { it % 2 == 0 });            // Some(4)
```

_Defined in `lang/std/iter/iterator.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item
```

The element type produced by `next()`.

_Defined in `lang/std/iter/iterator.ks`._

#### function `next`

```kestrel
mutating func next() -> Item?
```

Yields the next element, or `None` once exhausted. The protocol
does *not* require that subsequent calls keep returning `None` —
wrap with `fuse()` if you need that guarantee.

_Defined in `lang/std/iter/iterator.ks`._

## struct `EmptyIterator`

```kestrel
public struct EmptyIterator[T] { /* private fields */ }
```

Iterator that yields no elements. Returned by `empty()`.

### Representation

Zero-sized — no fields.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Builds an `EmptyIterator`. Prefer the free `empty()` function.

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> T?
```

Always `None`.

_Defined in `lang/std/iter/adapters.ks`._

## struct `EnumerateIterator`

```kestrel
public struct EnumerateIterator[I] where I: Iterator { /* private fields */ }
```

Lazy `enumerate` — pairs each element with its zero-based position.
Returned by `Iterator.enumerate()`.

### Representation

Source iterator + a running `Int64` index that ticks per element.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(inner: I)
```

Builds an `EnumerateIterator` with the index starting at 0.
Prefer `inner.enumerate()`.

_Defined in `lang/std/iter/adapters.ks`._

#### field `index`

```kestrel
internal var index: Int64
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `inner`

```kestrel
internal var inner: I
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = (Int64, I.Item)
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> (Int64, I.Item)?
```

Pulls the next element and pairs it with the current index, then
increments the index.

_Defined in `lang/std/iter/adapters.ks`._

## protocol `ExactSizeIterator`

```kestrel
public protocol ExactSizeIterator
```

An iterator that knows its remaining length up front. Conform when you
can answer cheaply — consumers (notably `collect`) use it to
pre-allocate exact capacity.

_Defined in `lang/std/iter/iterator.ks`._

### Members

#### function `isEmpty`

```kestrel
public func isEmpty() -> Bool
```

`true` when no elements remain. Equivalent to `remaining == 0`.

_Defined in `lang/std/iter/iterator.ks`._

#### field `remaining`

```kestrel
var remaining: Int64 { get }
```

Number of elements still to come. Decreases by one each time
`next()` returns `Some`; reaches zero when the iterator is
exhausted.

_Defined in `lang/std/iter/iterator.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item
```

The element type produced by `next()`.

_Defined in `lang/std/iter/iterator.ks`._

#### function `next`

```kestrel
mutating func next() -> Item?
```

Yields the next element, or `None` once exhausted. The protocol
does *not* require that subsequent calls keep returning `None` —
wrap with `fuse()` if you need that guarantee.

_Defined in `lang/std/iter/iterator.ks`._

## struct `FilterIterator`

```kestrel
public struct FilterIterator[I] where I: Iterator { /* private fields */ }
```

Lazy `filter` — yields only elements where the predicate returns
`true`. Returned by `Iterator.filter(_:)`.

### Representation

Source iterator + predicate closure. `next()` skips ahead until the
predicate accepts.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(inner: I, matching: (I.Item) -> Bool)
```

Builds a `FilterIterator`. Prefer `inner.filter(predicate)`.

_Defined in `lang/std/iter/adapters.ks`._

#### field `inner`

```kestrel
internal var inner: I
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `predicate`

```kestrel
internal var predicate: (I.Item) -> Bool
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = I.Item
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> I.Item?
```

Pulls until an element satisfies `predicate`, returning it. `None`
when the source is exhausted with no further match.

_Defined in `lang/std/iter/adapters.ks`._

## struct `FilterMapIterator`

```kestrel
public struct FilterMapIterator[I, U] where I: Iterator { /* private fields */ }
```

Lazy `filterMap` / `compactMap` — runs a transform that returns
`Optional[U]` and drops `None`s. Returned by both
`Iterator.filterMap(_:)` and `Iterator.compactMap()`.

### Representation

Source iterator + transform closure. `next()` skips ahead until the
transform yields `Some`.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(inner: I, mapping: (I.Item) -> U?)
```

Builds a `FilterMapIterator`. Prefer `inner.filterMap(...)` /
`inner.compactMap()`.

_Defined in `lang/std/iter/adapters.ks`._

#### field `inner`

```kestrel
internal var inner: I
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `transform`

```kestrel
internal var transform: (I.Item) -> U?
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = U
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> U?
```

Pulls until `transform` returns `Some`, then yields it.

_Defined in `lang/std/iter/adapters.ks`._

## struct `FlatMapIterator`

```kestrel
public struct FlatMapIterator[I, U] where I: Iterator, U: Iterator { /* private fields */ }
```

Lazy `flatMap` — turns each element of the source into an iterator
and concatenates the results. Returned by `Iterator.flatMap(_:)`.

### Representation

Source iterator + transform closure + a one-slot buffer (`current`)
holding the inner iterator currently being drained.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(inner: I, mapping: (I.Item) -> U)
```

Builds a `FlatMapIterator` with no inner iterator buffered.

_Defined in `lang/std/iter/adapters.ks`._

#### field `current`

```kestrel
internal var current: U?
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `inner`

```kestrel
internal var inner: I
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `transform`

```kestrel
internal var transform: (I.Item) -> U
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = U.Item
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> U.Item?
```

Drains the buffered inner iterator; when it runs out, pulls the
next source element, transforms it into a fresh inner iterator,
and continues.

_Defined in `lang/std/iter/adapters.ks`._

## struct `FlattenIterator`

```kestrel
public struct FlattenIterator[I] where I: Iterator, I.Item: Iterator { /* private fields */ }
```

Lazy `flatten` — concatenates the inner iterators of an
iterator-of-iterators. Returned by `Iterator.flatten()`.

### Representation

Source iterator + a one-slot buffer holding the inner iterator
currently being drained.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(inner: I)
```

Builds a `FlattenIterator` with no inner iterator buffered.

_Defined in `lang/std/iter/adapters.ks`._

#### field `current`

```kestrel
internal var current: I.Item?
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `inner`

```kestrel
internal var inner: I
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = I.Item.Item
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> I.Item.Item?
```

Drains the buffered inner iterator, then pulls the next inner
iterator from the source.

_Defined in `lang/std/iter/adapters.ks`._

## struct `FusedIterator`

```kestrel
public struct FusedIterator[I] where I: Iterator { /* private fields */ }
```

Wraps a source so that once `None` is returned, future calls also
return `None`. Returned by `Iterator.fuse()`.

### Representation

Source iterator + a one-bit latch.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(inner: I)
```

Builds a `FusedIterator` in the "still active" state.

_Defined in `lang/std/iter/adapters.ks`._

#### field `done`

```kestrel
internal var done: Bool
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `inner`

```kestrel
internal var inner: I
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = I.Item
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> I.Item?
```

Forwards `next()`; latches `done = true` on the first `None` and
returns `None` forever afterwards.

_Defined in `lang/std/iter/adapters.ks`._

## struct `InspectIterator`

```kestrel
public struct InspectIterator[I] where I: Iterator { /* private fields */ }
```

Side-effecting passthrough. Calls `inspector` on each element and
then yields it unchanged. Returned by `Iterator.inspect(_:)`.

### Representation

Source iterator + inspector closure. No buffering.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(inner: I, inspecting: (I.Item) -> ())
```

Builds an `InspectIterator`. Prefer `inner.inspect(inspector)`.

_Defined in `lang/std/iter/adapters.ks`._

#### field `inner`

```kestrel
internal var inner: I
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `inspector`

```kestrel
internal var inspector: (I.Item) -> ()
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = I.Item
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> I.Item?
```

Pulls from the source, calls `inspector` on the value, and yields
it.

_Defined in `lang/std/iter/adapters.ks`._

## struct `IntersperseIterator`

```kestrel
public struct IntersperseIterator[I] where I: Iterator { /* private fields */ }
```

Lazy `intersperse` — inserts a copy of `separator` between
consecutive elements. Returned by `Iterator.intersperse(separator:)`.

### Representation

Source iterator + separator value + a `needsSeparator` flag + a
one-slot pending-element buffer (used to remember an element while a
separator is being yielded).

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(inner: I, with: I.Item)
```

Builds an `IntersperseIterator`.

_Defined in `lang/std/iter/adapters.ks`._

#### field `inner`

```kestrel
internal var inner: I
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `needsSeparator`

```kestrel
internal var needsSeparator: Bool
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `pendingItem`

```kestrel
internal var pendingItem: I.Item?
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `separator`

```kestrel
internal var separator: I.Item
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = I.Item
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> I.Item?
```

Returns the buffered element if one is pending, otherwise pulls
the next source element — yielding a separator instead the second
time around.

_Defined in `lang/std/iter/adapters.ks`._

## struct `IntersperseWithIterator`

```kestrel
public struct IntersperseWithIterator[I] where I: Iterator { /* private fields */ }
```

Lazy `intersperseWith` — like `IntersperseIterator`, but builds each
separator on demand by calling a closure. Returned by
`Iterator.intersperseWith(separator:)`.

### Representation

Same as `IntersperseIterator`, except the stored value is a
zero-arg closure producing fresh separators.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(inner: I, with: () -> I.Item)
```

Builds an `IntersperseWithIterator`.

_Defined in `lang/std/iter/adapters.ks`._

#### field `inner`

```kestrel
internal var inner: I
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `needsSeparator`

```kestrel
internal var needsSeparator: Bool
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `pendingItem`

```kestrel
internal var pendingItem: I.Item?
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `separator`

```kestrel
internal var separator: () -> I.Item
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = I.Item
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> I.Item?
```

Same logic as `IntersperseIterator.next`, but each separator is
produced by calling `separator()`.

_Defined in `lang/std/iter/adapters.ks`._

## protocol `Iterable`

```kestrel
public protocol Iterable
```

A type that can hand out an iterator over its contents — what `for-in`
loops desugar through, and what most collections conform to.

`Iterable` is one level above `Iterator`: a collection conforms to
`Iterable` and produces a fresh `Iter` each call to `iter()`, leaving
the source intact. (Compare with `Iterator`, which is consumed in
place.) Every `Iterator` is also `Iterable` via the blanket
conformance below — `iter()` on an iterator returns itself.

### Examples

```
for item in myCollection {
    // identical to:
    // var it = myCollection.iter();
    // while let .Some(item) = it.next() { ... }
}
```

_Defined in `lang/std/iter/iterator.ks`._

### Members

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
func iter() -> TargetIterator
```

Builds a fresh iterator over the contents.

_Defined in `lang/std/iter/iterator.ks`._

## protocol `Iterator`

```kestrel
public protocol Iterator
```

_Defined in `lang/std/iter/iterator.ks`._

### Members

#### function `all`

```kestrel
public mutating func all(matching: (Item) -> Bool) -> Bool
```

True if every element satisfies `predicate`. Stops at the first
failure. True for an empty iterator (vacuous truth).

##### Examples

```
[2, 4, 6].iter().all({ it % 2 == 0 });   // true
[2, 3, 4].iter().all({ it % 2 == 0 });   // false (stops at 3)
[].iter().all({ false });                // true (empty)
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `any`

```kestrel
public mutating func any(matching: (Item) -> Bool) -> Bool
```

True if any element satisfies `predicate`. Stops at the first
match. False for an empty iterator.

##### Examples

```
[1, 2, 3, 4].iter().any({ it > 3 });    // true (stops at 4)
[1, 2, 3].iter().any({ it > 10 });      // false
[].iter().any({ true });                // false
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
[1, 2].iter().chain(other: [3, 4].iter()).collect();   // [1, 2, 3, 4]
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
[1, 2, 3].iter().filter({ it > 1 }).collect();   // [2, 3]
(1..5).iter().map({ it * it }).collect();        // [1, 4, 9, 16]
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
[1, 2, 3].iter().contains(element: 2);   // true
[1, 2, 3].iter().contains(element: 5);   // false
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
[1, 2, 3, 4, 5].iter().filter({ it % 2 == 0 }).count();   // 2
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
[1, 2, 3].iter().cycle().take(count: 7).collect();
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
public func filter(matching: (Item) -> Bool) -> FilterIterator[Self]
```

Yields only elements where `predicate` returns `true`. Lazy —
elements are tested as they're pulled.

##### Examples

```
[1, 2, 3, 4, 5].iter().filter({ it % 2 == 0 }).collect();   // [2, 4]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `filterMap`

```kestrel
public func filterMap[U]((Item) -> U?) -> FilterMapIterator[Self, U]
```

Combined map + filter — `transform` returns `Optional[U]`; `None`
values are skipped. Use over `map(...).filter(...)` when the
transform itself decides whether the element belongs.

##### Examples

```
["1", "two", "3"].iter()
    .filterMap({ Int64.parse(it) })
    .collect();   // [1, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `first`

```kestrel
public mutating func first(matching: (Item) -> Bool) -> Item?
```

First element matching `predicate`, or `None`. Stops at the first
match.

##### Examples

```
[1, 2, 3, 4, 5].iter().first(matching: { it > 3 });   // Some(4)
[1, 2, 3].iter().first(matching: { it > 10 });        // None
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `first`

```kestrel
public mutating func first() -> Item?
```

First element, or `None` if empty. Consumes only the first
element. Equivalent to `next()`, but reads more naturally as a
terminal.

_Defined in `lang/std/iter/iterator.ks`._

#### function `flatMap`

```kestrel
public func flatMap[U]((Item) -> U) -> FlatMapIterator[Self, U] where U: Iterator
```

Maps each element to an iterator and concatenates the results.
The monadic bind for iterators.

##### Examples

```
[[1, 2], [3, 4], [5]].iter()
    .flatMap({ it.iter() })
    .collect();   // [1, 2, 3, 4, 5]
```

```
// Conditional expand — drop odd, double even
[1, 2, 3].iter()
    .flatMap({ if it % 2 == 0 { [it, it].iter() } else { [].iter() } })
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
let nested = [[1, 2], [3, 4], [5]].iter().map({ it.iter() });
nested.flatten().collect();   // [1, 2, 3, 4, 5]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `fold`

```kestrel
public consuming func fold[Acc](from: Acc, combining: (Acc, Item) -> Acc) -> Acc
```

Left fold — start at `initial` and walk left to right, applying
`combine(acc, element)`. Returns `initial` for an empty iterator.

##### Examples

```
[1, 2, 3, 4].iter().fold(from: 0,  combining: |acc, x| acc + x);   // 10
[1, 2, 3].iter().fold(from: 1,  combining: |acc, x| acc * x);      // 6
[].iter().fold(from: 42,  combining: |acc, x| acc + x);            // 42
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
[1, 2, 3].iter().forEach({ print(it) });
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
public func inspect(inspecting: (Item) -> ()) -> InspectIterator[Self]
```

Calls `inspector` on each element as it flows through, leaving
the value otherwise untouched. Useful for logging or
instrumenting an adapter chain mid-pipeline.

##### Examples

```
[1, 2, 3].iter()
    .inspect({ print("before filter: \{it}") })
    .filter({ it > 1 })
    .inspect({ print("after filter: \{it}") })
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
[1, 2, 3].iter().intersperse(separator: 0).collect();
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
    .intersperseWith(separator: || { counter += 1; counter * 10 })
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

#### function `isSorted`

```kestrel
public consuming func isSorted(by: (Item, Item) -> Bool) -> Bool
```

True if every adjacent pair satisfies `comparator(prev, next)` —
i.e. they are already in the order `comparator` defines.

##### Examples

```
// Descending check
[5, 4, 3, 2, 1].iter().isSorted(by: |a, b| a >= b);   // true
// By absolute value
[-1, 2, -3, 4].iter().isSorted(by: |a, b| a.abs() <= b.abs());   // true
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `isSorted`

```kestrel
public consuming func isSorted[K](byKey: (Item) -> K) -> Bool where K: Comparable
```

True if elements are sorted ascending by `key(element)`. Sugar
over `isSorted(by:)` for the common "by-key" shape.

##### Examples

```
let words = ["a", "bb", "ccc"];
words.iter().isSorted(byKey: { it.count });   // true
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `isSortedDescending`

```kestrel
public consuming func isSortedDescending() -> Bool
```

True if elements come out in descending order. Mirror of
`isSorted`.

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
public func map[U]((Item) -> U) -> MapIterator[Self, U]
```

Applies `transform` to each element. Lazy — the function only
fires when the downstream pulls a value.

##### Examples

```
[1, 2, 3].iter().map({ it * 2 }).collect();         // [2, 4, 6]
["hi", "yo"].iter().map({ it.count }).collect();    // [2, 2]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `max`

```kestrel
public consuming func max() -> Item?
```

Largest element, or `None` for an empty iterator. Ties go to the
first occurrence.

_Defined in `lang/std/iter/iterator.ks`._

#### function `max`

```kestrel
public consuming func max[K](byKey: (Item) -> K) -> Item? where K: Comparable
```

The element with the largest `key(element)`. Mirror of `min(byKey:)`.

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

#### function `min`

```kestrel
public consuming func min[K](byKey: (Item) -> K) -> Item? where K: Comparable
```

The element with the smallest `key(element)`. Ties go to the
first occurrence.

##### Examples

```
let people = [("Alice", 30), ("Bob", 25), ("Charlie", 35)];
people.iter().min(byKey: { it.1 });   // Some(("Bob", 25))
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `next`

```kestrel
mutating func next() -> Item?
```

Yields the next element, or `None` once exhausted. The protocol
does *not* require that subsequent calls keep returning `None` —
wrap with `fuse()` if you need that guarantee.

_Defined in `lang/std/iter/iterator.ks`._

#### function `nth`

```kestrel
public mutating func nth(Int64) -> Item?
```

Returns the element at index `n` (zero-based), consuming
everything up to and including it. `None` if `n` is past the end.

##### Examples

```
[10, 20, 30, 40].iter().nth(n: 2);   // Some(30)
[10, 20].iter().nth(n: 5);           // None
[10, 20, 30].iter().nth(n: 0);       // Some(10)
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

#### function `position`

```kestrel
public mutating func position(matching: (Item) -> Bool) -> Int64?
```

Index of the first element matching `predicate`, or `None`.
Mirror of `find` for positions.

##### Examples

```
["a", "b", "c"].iter().position({ it == "b" });   // Some(1)
[1, 2, 3].iter().position({ it > 10 });           // None
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
public consuming func reduce(combining: (Item, Item) -> Item) -> Item?
```

Like `fold`, but seeds the accumulator with the first element
instead of taking an explicit `initial`. Returns `None` for an
empty iterator.

##### Examples

```
[1, 2, 3, 4].iter().reduce(combining: |a, b| a + b);   // Some(10)
[5].iter().reduce(combining: |a, b| a + b);            // Some(5)
[].iter().reduce(combining: |a, b| a + b);             // None
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `scan`

```kestrel
public func scan[Acc](from: Acc, combining: (Acc, Item) -> Acc) -> ScanIterator[Self, Acc]
```

Like `fold`, but yields each intermediate accumulator value
instead of just the final one. Useful for prefix sums, running
products, and any "carry state along" pattern.

##### Examples

```
// Running sum
[1, 2, 3, 4].iter()
    .scan(from: 0, combining: |acc, x| acc + x)
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
[1, 2, 3, 4, 5].iter().skip(count: 2).collect();   // [3, 4, 5]
[1, 2].iter().skip(count: 10).collect();           // []
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `skipWhile`

```kestrel
public func skipWhile(matching: (Item) -> Bool) -> SkipWhileIterator[Self]
```

Drops elements while `predicate` is `true`, then yields *every*
remaining element (including ones that would also satisfy the
predicate). Mirror of `takeWhile`.

##### Examples

```
[1, 2, 3, 4, 1, 2].iter()
    .skipWhile({ it < 3 })
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
[3, 1, 2].iter().filter({ it > 1 }).sorted();          // [2, 3]
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
[0, 1, 2, 3, 4, 5, 6].iter().stepBy(n: 2).collect();   // [0, 2, 4, 6]
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
[1, 2, 3, 4, 5].iter().take(count: 3).collect();   // [1, 2, 3]
[1, 2].iter().take(count: 10).collect();           // [1, 2]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `takeWhile`

```kestrel
public func takeWhile(matching: (Item) -> Bool) -> TakeWhileIterator[Self]
```

Yields elements until `predicate` first returns `false`, then
stops. The "first failing" element is *not* yielded.

##### Examples

```
[1, 2, 3, 4, 1, 2].iter()
    .takeWhile({ it < 4 })
    .collect();   // [1, 2, 3]
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `tryFold`

```kestrel
public mutating func tryFold[Acc, E](from: Acc, combining: (Acc, Item) -> Result[Acc, E]) -> Result[Acc, E]
```

Fold with early exit on `Err`. The combine returns `Result`; the
first `Err` halts iteration and is returned. If everything
succeeds, returns `Ok(final accumulator)`.

##### Examples

```
// Stop the moment a parse fails
["1", "2", "3"].iter()
    .tryFold(from: 0,  combining: |acc, s| {
        match Int64.parse(s) {
            .Some(n) => .Ok(acc + n),
            .None    => .Err("parse error")
        }
    });   // Ok(6)

["1", "bad", "3"].iter()
    .tryFold(from: 0,  combining: |acc, s| {
        match Int64.parse(s) {
            .Some(n) => .Ok(acc + n),
            .None    => .Err("parse error")
        }
    });   // Err("parse error")
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
files.iter().tryForEach({ (path) in
    File.delete(path)   // Result[(), IoError]
});   // stops on first failure
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
names.iter().zip(other: ages.iter()).collect();
// [("Alice", 30), ("Bob", 25), ("Charlie", 35)]
```

_Defined in `lang/std/iter/iterator.ks`._

### Implements `Iterable`

#### typealias `Item`

```kestrel
type Item
```

The element type produced by `next()`.

_Defined in `lang/std/iter/iterator.ks`._

#### typealias `Item`

```kestrel
type Item = Self.Item
```

_Defined in `lang/std/iter/iterator.ks`._

#### typealias `TargetIterator`

```kestrel
type TargetIterator = Self
```

_Defined in `lang/std/iter/iterator.ks`._

#### function `iter`

```kestrel
func iter() -> Self
```

Returns `self`. The blanket conformance pivot — iterators *are*
iterables.

_Defined in `lang/std/iter/iterator.ks`._

## struct `MapIterator`

```kestrel
public struct MapIterator[I, U] where I: Iterator { /* private fields */ }
```

Lazy `map` — applies a transform to each element of `inner` as values
are pulled. Returned by `Iterator.map(_:)`.

### Representation

Wraps the source iterator and the transform closure. No buffering —
elements pass through one at a time.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(inner: I, mapping: (I.Item) -> U)
```

Builds a `MapIterator` from `inner` and `transform`. Prefer
`inner.map(transform)`.

_Defined in `lang/std/iter/adapters.ks`._

#### field `inner`

```kestrel
internal var inner: I
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `transform`

```kestrel
internal var transform: (I.Item) -> U
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = U
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> U?
```

Pulls the next element from `inner` and runs `transform` on it.

_Defined in `lang/std/iter/adapters.ks`._

## struct `OnceIterator`

```kestrel
public struct OnceIterator[T] { /* private fields */ }
```

Iterator that yields a single value, then nothing. Returned by
`once(value:)`.

### Representation

One `Optional[T]` field. `next()` empties it on first call.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Value`

```kestrel
public init(value: T)
```

Builds a `OnceIterator` carrying `value`.

_Defined in `lang/std/iter/adapters.ks`._

#### field `value`

```kestrel
internal var value: T?
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> T?
```

Returns the value once, then `None` forever after.

_Defined in `lang/std/iter/adapters.ks`._

## struct `PeekableIterator`

```kestrel
public struct PeekableIterator[I] where I: Iterator { /* private fields */ }
```

Iterator wrapper that lets you peek at the next element without
consuming it. Returned by `Iterator.peekable()`.

### Representation

Source iterator + a one-slot lookahead buffer (`peeked`). `peek()`
fills the buffer; `next()` drains it before pulling the source.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(inner: I)
```

Builds a `PeekableIterator` with no value buffered.

_Defined in `lang/std/iter/adapters.ks`._

#### field `inner`

```kestrel
internal var inner: I
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `peek`

```kestrel
public mutating func peek() -> I.Item?
```

Returns the next element without consuming it. Subsequent
`peek()` calls keep returning the same value until `next()` is
called.

_Defined in `lang/std/iter/adapters.ks`._

#### field `peeked`

```kestrel
internal var peeked: Optional[I.Item]?
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = I.Item
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> I.Item?
```

Returns the buffered value if present, otherwise pulls the source.

_Defined in `lang/std/iter/adapters.ks`._

## struct `RepeatIterator`

```kestrel
public struct RepeatIterator[T] { /* private fields */ }
```

Iterator that yields the same value indefinitely. Returned by
`repeatValue(value:)`.

### Representation

One `T` field that is copied on every `next()` call.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Value`

```kestrel
public init(value: T)
```

Builds a `RepeatIterator` over `value`.

_Defined in `lang/std/iter/adapters.ks`._

#### field `value`

```kestrel
internal var value: T
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> T?
```

Returns a fresh copy of the stored value every call.

_Defined in `lang/std/iter/adapters.ks`._

## struct `RepeatNIterator`

```kestrel
public struct RepeatNIterator[T] { /* private fields */ }
```

Iterator that yields the same value `count` times, then stops.
Returned by `repeatN(value:count:)`.

### Representation

`T` payload + an `Int64` countdown.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Value`

```kestrel
public init(value: T, count: Int64)
```

Builds a `RepeatNIterator` that will yield `value` exactly
`count` times.

_Defined in `lang/std/iter/adapters.ks`._

#### field `remaining`

```kestrel
internal var remaining: Int64
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `value`

```kestrel
internal var value: T
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = T
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> T?
```

Decrements `remaining` and returns a fresh copy of the value;
returns `None` once the counter hits zero.

_Defined in `lang/std/iter/adapters.ks`._

## struct `ReversedIterator`

```kestrel
public struct ReversedIterator[I] where I: DoubleEndedIterator, I: Iterator { /* private fields */ }
```

Wraps a `DoubleEndedIterator` to walk it back to front. The
`Iterator` conformance is added by the `extend ReversedIterator[I]:
DoubleEndedIterator` block in `iterator.ks`. Returned by
`DoubleEndedIterator.rev()`.

### Representation

Just the inner iterator — no buffering.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(inner: I)
```

Builds a `ReversedIterator`. Prefer `inner.rev()`.

_Defined in `lang/std/iter/adapters.ks`._

#### field `inner`

```kestrel
internal var inner: I
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = I.Item
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> I.Item?
```

Pulls from the back of the inner iterator (which is the front of
the reversed view).

_Defined in `lang/std/iter/iterator.ks`._

### Implements `DoubleEndedIterator`

#### function `nextBack`

```kestrel
public mutating func nextBack() -> I.Item?
```

Pulls from the front of the inner iterator (which is the back of
the reversed view).

_Defined in `lang/std/iter/iterator.ks`._

## struct `ScanIterator`

```kestrel
public struct ScanIterator[I, Acc] where I: Iterator { /* private fields */ }
```

Lazy `scan` — yields the running fold accumulator after each step.
Returned by `Iterator.scan(initial:combine:)`.

### Representation

Source iterator + the running accumulator state + the combine
closure.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(inner: I, from: Acc, combining: (Acc, I.Item) -> Acc)
```

Builds a `ScanIterator` seeded with `initial`.

_Defined in `lang/std/iter/adapters.ks`._

#### field `combine`

```kestrel
internal var combine: (Acc, I.Item) -> Acc
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `inner`

```kestrel
internal var inner: I
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `state`

```kestrel
internal var state: Acc
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = Acc
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> Acc?
```

Pulls the next element, updates `state`, and yields the new
state.

_Defined in `lang/std/iter/adapters.ks`._

## struct `SkipIterator`

```kestrel
public struct SkipIterator[I] where I: Iterator { /* private fields */ }
```

Lazy `skip` — drops the first `count` elements, then yields the rest.
Returned by `Iterator.skip(count:)`.

### Representation

Source iterator + a counter; the first `next()` call drains the
budget by pulling the source.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(inner: I, count: Int64)
```

Builds a `SkipIterator` that will drop `count` elements before
yielding.

_Defined in `lang/std/iter/adapters.ks`._

#### field `inner`

```kestrel
internal var inner: I
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `remaining`

```kestrel
internal var remaining: Int64
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = I.Item
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> I.Item?
```

On first call, walks past `remaining` source elements; subsequent
calls forward `next()` directly.

_Defined in `lang/std/iter/adapters.ks`._

## struct `SkipWhileIterator`

```kestrel
public struct SkipWhileIterator[I] where I: Iterator { /* private fields */ }
```

Lazy `skipWhile` — drops a leading run of elements satisfying the
predicate, then yields *every* remaining element. Returned by
`Iterator.skipWhile(_:)`.

### Representation

Source iterator + predicate + a one-bit `doneSkipping` flag that
latches once the skipping phase ends.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(inner: I, matching: (I.Item) -> Bool)
```

Builds a `SkipWhileIterator`. Prefer `inner.skipWhile(predicate)`.

_Defined in `lang/std/iter/adapters.ks`._

#### field `doneSkipping`

```kestrel
internal var doneSkipping: Bool
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `inner`

```kestrel
internal var inner: I
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `predicate`

```kestrel
internal var predicate: (I.Item) -> Bool
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = I.Item
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> I.Item?
```

On first call, drains source elements that match `predicate` and
returns the first one that doesn't. After that, forwards `next()`
directly.

_Defined in `lang/std/iter/adapters.ks`._

## struct `StepByIterator`

```kestrel
public struct StepByIterator[I] where I: Iterator { /* private fields */ }
```

Lazy `stepBy` — yields every `step`-th element, starting with the
first. Returned by `Iterator.stepBy(n:)`.

### Representation

Source iterator + step size + a one-bit `first` flag (the first
element is always emitted; subsequent ones consume `step - 1` extra
pulls).

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(inner: I, step: Int64)
```

Builds a `StepByIterator`. Caller guarantees `step >= 1`; `step
== 0` produces undefined behaviour.

_Defined in `lang/std/iter/adapters.ks`._

#### field `first`

```kestrel
internal var first: Bool
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `inner`

```kestrel
internal var inner: I
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `step`

```kestrel
internal var step: Int64
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = I.Item
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> I.Item?
```

Yields the first element on the first call; subsequently drains
`step - 1` elements and yields the next.

_Defined in `lang/std/iter/adapters.ks`._

## struct `TakeIterator`

```kestrel
public struct TakeIterator[I] where I: Iterator { /* private fields */ }
```

Lazy `take` — yields at most `count` elements from the source.
Returned by `Iterator.take(count:)`.

### Representation

Source iterator + a counter that ticks down to zero.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(inner: I, count: Int64)
```

Builds a `TakeIterator` with `count` capacity.

_Defined in `lang/std/iter/adapters.ks`._

#### field `inner`

```kestrel
internal var inner: I
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `remaining`

```kestrel
internal var remaining: Int64
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = I.Item
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> I.Item?
```

Decrements `remaining` and forwards `next()`; returns `None` once
the budget hits zero.

_Defined in `lang/std/iter/adapters.ks`._

## struct `TakeWhileIterator`

```kestrel
public struct TakeWhileIterator[I] where I: Iterator { /* private fields */ }
```

Lazy `takeWhile` — yields elements until the predicate first returns
`false`, then permanently stops. Returned by `Iterator.takeWhile(_:)`.

### Representation

Source iterator + predicate + a one-bit `done` flag that latches once
the predicate fails.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Source`

```kestrel
public init(inner: I, matching: (I.Item) -> Bool)
```

Builds a `TakeWhileIterator`. Prefer `inner.takeWhile(predicate)`.

_Defined in `lang/std/iter/adapters.ks`._

#### field `done`

```kestrel
internal var done: Bool
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `inner`

```kestrel
internal var inner: I
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `predicate`

```kestrel
internal var predicate: (I.Item) -> Bool
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = I.Item
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> I.Item?
```

Returns the next element if `predicate` still accepts; latches
`done = true` and returns `None` on the first rejection or
underlying exhaustion.

_Defined in `lang/std/iter/adapters.ks`._

## struct `ZipIterator`

```kestrel
public struct ZipIterator[A, B] where A: Iterator, B: Iterator { /* private fields */ }
```

Lazy `zip` — pairs elements from two iterators. Stops at the shorter
one. Returned by `Iterator.zip(other:)`.

### Representation

Holds both source iterators. No buffering.

_Defined in `lang/std/iter/adapters.ks`._

### Members

#### initializer `From Sources`

```kestrel
public init(first: A, second: B)
```

Builds a `ZipIterator`. Prefer `first.zip(other: second)`.

_Defined in `lang/std/iter/adapters.ks`._

#### field `first`

```kestrel
internal var first: A
```

_Defined in `lang/std/iter/adapters.ks`._

#### field `second`

```kestrel
internal var second: B
```

_Defined in `lang/std/iter/adapters.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = (A.Item, B.Item)
```

_Defined in `lang/std/iter/adapters.ks`._

#### function `next`

```kestrel
public mutating func next() -> (A.Item, B.Item)?
```

Pulls one element from each side and pairs them. `None` if either
runs out.

_Defined in `lang/std/iter/adapters.ks`._

## function `empty`

```kestrel
public func empty[T]() -> EmptyIterator[T]
```

Returns an `EmptyIterator[T]`. Useful as a "neutral element" in
iterator algebra (`a.chain(other: empty())`).

_Defined in `lang/std/iter/adapters.ks`._

## function `once`

```kestrel
public func once[T](T) -> OnceIterator[T]
```

Returns a `OnceIterator` that yields `value` and then nothing.
Equivalent to `[value].iter()` without the array allocation.

_Defined in `lang/std/iter/adapters.ks`._

## function `repeatN`

```kestrel
public func repeatN[T](T, Int64) -> RepeatNIterator[T]
```

Returns a `RepeatNIterator` that yields `count` copies of `value`,
then stops.

_Defined in `lang/std/iter/adapters.ks`._

## function `repeatValue`

```kestrel
public func repeatValue[T](T) -> RepeatIterator[T]
```

Returns a `RepeatIterator` that yields copies of `value` forever.
Combine with `take` to cap it.

_Defined in `lang/std/iter/adapters.ks`._

