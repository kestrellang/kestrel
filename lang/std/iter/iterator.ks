// Core iterator protocols for lazy sequence processing

module std.iter

import std.result.(Optional, Result)
import std.core.(Bool, Equatable, Comparable, Addable, Multipliable, Copyable, Cloneable, Ordering)
import std.num.(Int64)
import std.collections.(Array)
import std.iter.(
    MapIterator,
    FilterIterator,
    FilterMapIterator,
    EnumerateIterator,
    FlatMapIterator,
    ScanIterator,
    TakeIterator,
    TakeWhileIterator,
    SkipIterator,
    SkipWhileIterator,
    ZipIterator,
    ChainIterator,
    PeekableIterator,
    FuseIterator,
    InspectIterator,
    StepByIterator,
    IntersperseIterator,
    IntersperseWithIterator,
    CycleIterator,
    FlattenIterator,
    RevIterator
)

// ============================================================================
// CORE PROTOCOLS
// ============================================================================

/// Pull-style sequence — produces one element at a time via `next()` until
/// it returns `None`.
///
/// Iterators are the foundation of lazy sequence processing in Kestrel.
/// They are *single-pass* by default: once consumed, an iterator must be
/// rebuilt from its source. The dozens of `extend Iterator` blocks in this
/// file layer adapter combinators (lazy: `map`, `filter`, `take`, `zip`,
/// …) and terminal operations (eager: `collect`, `fold`, `find`, `sum`,
/// …) on top of the single `next` requirement.
///
/// # Examples
///
/// ```
/// // Defining a custom iterator
/// struct CountDown: Iterator {
///     type Item = Int64;
///     var current: Int64;
///
///     mutating func next() -> Int64? {
///         if current <= 0 { return None };
///         let value = current;
///         current -= 1;
///         .Some(value)
///     }
/// }
///
/// var c = CountDown(current: 3);
/// c.next();   // Some(3)
/// c.next();   // Some(2)
/// c.next();   // Some(1)
/// c.next();   // None
/// ```
///
/// ```
/// // Using the adapter / terminal surface
/// let evens: [Int64] = [1, 2, 3, 4, 5].iter()
///     .filter({ it % 2 == 0 })
///     .map({ it * 10 })
///     .collect();   // [20, 40]
/// ```
@builtin(.IteratorProtocol)
public protocol Iterator {
    /// The element type produced by `next()`.
    type Item

    /// Yields the next element, or `None` once exhausted. The protocol
    /// does *not* require that subsequent calls keep returning `None` —
    /// wrap with `fuse()` if you need that guarantee.
    @builtin(.IteratorNextMethod)
    mutating func next() -> Item?
}

/// A type that can hand out an iterator over its contents — what `for-in`
/// loops desugar through, and what most collections conform to.
///
/// `Iterable` is one level above `Iterator`: a collection conforms to
/// `Iterable` and produces a fresh `Iter` each call to `iter()`, leaving
/// the source intact. (Compare with `Iterator`, which is consumed in
/// place.) Every `Iterator` is also `Iterable` via the blanket
/// conformance below — `iter()` on an iterator returns itself.
///
/// # Examples
///
/// ```
/// for item in myCollection {
///     // identical to:
///     // var it = myCollection.iter();
///     // while let .Some(item) = it.next() { ... }
/// }
/// ```
@builtin(.IterableProtocol)
public protocol Iterable {
    /// The element type that iteration yields.
    type Item

    /// The concrete iterator type returned by `iter()`. Constrained so
    /// `Iter.Item` matches `Self.Item`.
    type Iter: Iterator where Iter.Item = Item

    /// Builds a fresh iterator over the contents.
    @builtin(.IterableIterMethod)
    func iter() -> Iter
}

// ============================================================================
// ITERATOR IS ITERABLE
// ============================================================================

/// Blanket conformance: every `Iterator` is `Iterable` (returning itself).
/// This is what lets the result of an adapter chain — a `MapIterator`,
/// `FilterIterator`, etc. — drop into a `for-in` loop without an extra
/// `iter()` call.
extend Iterator: Iterable {
    type Iterable.Item = Self.Item
    type Iterable.Iter = Self

    /// Returns `self`. The blanket conformance pivot — iterators *are*
    /// iterables.
    func iter() -> Self { self }
}

/// An iterator that can also yield from the back. Powers `rev()` and
/// efficient "last N elements" patterns without first materialising the
/// whole sequence.
///
/// Front and back iteration share state — alternating `next()` and
/// `nextBack()` is well-defined and meets in the middle.
///
/// # Examples
///
/// ```
/// // Defining a double-ended range
/// struct Range: DoubleEndedIterator {
///     type Item = Int64;
///     var start: Int64;
///     var end: Int64;
///
///     mutating func next() -> Int64? {
///         if start >= end { return None };
///         let v = start;
///         start += 1;
///         .Some(v)
///     }
///
///     mutating func nextBack() -> Int64? {
///         if start >= end { return None };
///         end -= 1;
///         .Some(end)
///     }
/// }
///
/// var r = Range(start: 1, end: 4);
/// r.next();      // Some(1)
/// r.nextBack();  // Some(3)
/// r.next();      // Some(2)
/// r.nextBack();  // None  (start >= end)
/// ```
public protocol DoubleEndedIterator: Iterator {
    /// Yields the next element from the back, or `None` if the front and
    /// back have met. Can be interleaved freely with `next()`.
    mutating func nextBack() -> Item?
}

/// An iterator that knows its remaining length up front. Conform when you
/// can answer cheaply — consumers (notably `collect`) use it to
/// pre-allocate exact capacity.
public protocol ExactSizeIterator: Iterator {
    /// Number of elements still to come. Decreases by one each time
    /// `next()` returns `Some`; reaches zero when the iterator is
    /// exhausted.
    var remaining: Int64 { get };
}

// ============================================================================
// TRANSFORMATION ADAPTERS
// ============================================================================

/// Lazy element-by-element transforms. Each adapter returns a new
/// iterator that applies its operation as elements flow through, without
/// allocating intermediate collections.
extend Iterator {

    /// Applies `transform` to each element. Lazy — the function only
    /// fires when the downstream pulls a value.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].iter().map({ it * 2 }).collect();         // [2, 4, 6]
    /// ["hi", "yo"].iter().map({ it.count }).collect();    // [2, 2]
    /// ```
    public func map[U](transform: (Item) -> U) -> MapIterator[Self, U] {
        MapIterator(inner: self, transform: transform)
    }

    /// Yields only elements where `predicate` returns `true`. Lazy —
    /// elements are tested as they're pulled.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].iter().filter({ it % 2 == 0 }).collect();   // [2, 4]
    /// ```
    public func filter(predicate: (Item) -> Bool) -> FilterIterator[Self] {
        FilterIterator(inner: self, predicate: predicate)
    }

    /// Combined map + filter — `transform` returns `Optional[U]`; `None`
    /// values are skipped. Use over `map(...).filter(...)` when the
    /// transform itself decides whether the element belongs.
    ///
    /// # Examples
    ///
    /// ```
    /// ["1", "two", "3"].iter()
    ///     .filterMap({ Int64.parse(it) })
    ///     .collect();   // [1, 3]
    /// ```
    public func filterMap[U](transform: (Item) -> U?) -> FilterMapIterator[Self, U] {
        FilterMapIterator(inner: self, transform: transform)
    }

    /// Drops `None`s and unwraps `Some`s — the identity-transform special
    /// case of `filterMap`. Available when the iterator already yields
    /// optionals.
    ///
    /// # Examples
    ///
    /// ```
    /// let xs: [Int64?] = [.Some(1), .None, .Some(2), .None, .Some(3)];
    /// xs.iter().compactMap().collect();   // [1, 2, 3]
    /// ```
    public func compactMap[T]() -> FilterMapIterator[Self, T] where Item = Optional[T] {
        FilterMapIterator(inner: self, transform: { it })
    }

    /// Pairs each element with its zero-based position.
    ///
    /// # Examples
    ///
    /// ```
    /// for (i, item) in arr.iter().enumerate() {
    ///     print("Index \{i}: \{item}")
    /// };
    /// ```
    public func enumerate() -> EnumerateIterator[Self] {
        EnumerateIterator(inner: self)
    }

    /// Maps each element to an iterator and concatenates the results.
    /// The monadic bind for iterators.
    ///
    /// # Examples
    ///
    /// ```
    /// [[1, 2], [3, 4], [5]].iter()
    ///     .flatMap({ it.iter() })
    ///     .collect();   // [1, 2, 3, 4, 5]
    /// ```
    ///
    /// ```
    /// // Conditional expand — drop odd, double even
    /// [1, 2, 3].iter()
    ///     .flatMap({ if it % 2 == 0 { [it, it].iter() } else { [].iter() } })
    ///     .collect();   // [2, 2]
    /// ```
    public func flatMap[U](transform: (Item) -> U) -> FlatMapIterator[Self, U] where U: Iterator {
        FlatMapIterator(inner: self, transform: transform)
    }

    /// Like `fold`, but yields each intermediate accumulator value
    /// instead of just the final one. Useful for prefix sums, running
    /// products, and any "carry state along" pattern.
    ///
    /// # Examples
    ///
    /// ```
    /// // Running sum
    /// [1, 2, 3, 4].iter()
    ///     .scan(initial: 0, combine: |acc, x| acc + x)
    ///     .collect();   // [1, 3, 6, 10]
    /// ```
    public func scan[Acc](initial: Acc, combine: (Acc, Item) -> Acc) -> ScanIterator[Self, Acc] {
        ScanIterator(inner: self, initial: initial, combine: combine)
    }
}

// ============================================================================
// LIMITING ADAPTERS
// ============================================================================

/// Adapters that cut a sequence shorter or skip a prefix.
extend Iterator {

    /// Yields at most the first `count` elements; stops early even if
    /// more are available.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].iter().take(count: 3).collect();   // [1, 2, 3]
    /// [1, 2].iter().take(count: 10).collect();           // [1, 2]
    /// ```
    public func take(count: Int64) -> TakeIterator[Self] {
        TakeIterator(inner: self, count: count)
    }

    /// Yields elements until `predicate` first returns `false`, then
    /// stops. The "first failing" element is *not* yielded.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 1, 2].iter()
    ///     .takeWhile({ it < 4 })
    ///     .collect();   // [1, 2, 3]
    /// ```
    public func takeWhile(predicate: (Item) -> Bool) -> TakeWhileIterator[Self] {
        TakeWhileIterator(inner: self, predicate: predicate)
    }

    /// Drops the first `count` elements, then yields the rest.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].iter().skip(count: 2).collect();   // [3, 4, 5]
    /// [1, 2].iter().skip(count: 10).collect();           // []
    /// ```
    public func skip(count: Int64) -> SkipIterator[Self] {
        SkipIterator(inner: self, count: count)
    }

    /// Drops elements while `predicate` is `true`, then yields *every*
    /// remaining element (including ones that would also satisfy the
    /// predicate). Mirror of `takeWhile`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 1, 2].iter()
    ///     .skipWhile({ it < 3 })
    ///     .collect();   // [3, 4, 1, 2]
    /// ```
    public func skipWhile(predicate: (Item) -> Bool) -> SkipWhileIterator[Self] {
        SkipWhileIterator(inner: self, predicate: predicate)
    }
}

// ============================================================================
// COMBINING ADAPTERS
// ============================================================================

/// Adapters that fuse two iterators into one.
extend Iterator {

    /// Pairs elements from `self` and `other`. Stops as soon as either
    /// side runs out.
    ///
    /// # Examples
    ///
    /// ```
    /// let names = ["Alice", "Bob", "Charlie"];
    /// let ages  = [30, 25, 35];
    /// names.iter().zip(other: ages.iter()).collect();
    /// // [("Alice", 30), ("Bob", 25), ("Charlie", 35)]
    /// ```
    public func zip[Other](other: Other) -> ZipIterator[Self, Other] where Other: Iterator {
        ZipIterator(first: self, second: other)
    }

    /// Yields all of `self`, then all of `other`. Both must produce the
    /// same `Item` type.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2].iter().chain(other: [3, 4].iter()).collect();   // [1, 2, 3, 4]
    /// ```
    public func chain[Other](other: Other) -> ChainIterator[Self, Other] where Other: Iterator, Other.Item = Item {
        ChainIterator(first: self, second: other)
    }
}

// ============================================================================
// UTILITY ADAPTERS
// ============================================================================

/// Adapters that don't change *what* is yielded but change *how*.
extend Iterator {

    /// Wraps `self` so you can look at the next element without
    /// consuming it.
    ///
    /// # Examples
    ///
    /// ```
    /// var it = [1, 2, 3].iter().peekable();
    /// it.peek();   // Some(1) — no consumption
    /// it.peek();   // Some(1) — still
    /// it.next();   // Some(1) — now consumed
    /// it.peek();   // Some(2)
    /// ```
    public func peekable() -> PeekableIterator[Self] {
        PeekableIterator(inner: self)
    }

    /// Locks `None` once seen — protects against iterators that aren't
    /// fused (i.e. that may produce more elements after returning `None`
    /// once). After the first `None`, this adapter returns `None`
    /// forever.
    public func fuse() -> FuseIterator[Self] {
        FuseIterator(inner: self)
    }

    /// Calls `inspector` on each element as it flows through, leaving
    /// the value otherwise untouched. Useful for logging or
    /// instrumenting an adapter chain mid-pipeline.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].iter()
    ///     .inspect({ print("before filter: \{it}") })
    ///     .filter({ it > 1 })
    ///     .inspect({ print("after filter: \{it}") })
    ///     .collect();
    /// ```
    public func inspect(inspector: (Item) -> ()) -> InspectIterator[Self] {
        InspectIterator(inner: self, inspector: inspector)
    }

    /// Yields every `n`-th element, starting at the first. `n == 0` is
    /// undefined (the adapter will spin forever).
    ///
    /// # Examples
    ///
    /// ```
    /// [0, 1, 2, 3, 4, 5, 6].iter().stepBy(n: 2).collect();   // [0, 2, 4, 6]
    /// ```
    public func stepBy(n: Int64) -> StepByIterator[Self] {
        StepByIterator(inner: self, step: n)
    }
}

/// `intersperse` lives in its own block to keep its `T: Copyable`-style
/// requirements (the separator is copied per gap) close to the API.
extend Iterator {

    /// Inserts `separator` between consecutive elements. Empty inputs
    /// stay empty; single-element inputs get no separator.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].iter().intersperse(separator: 0).collect();
    /// // [1, 0, 2, 0, 3]
    /// ```
    public func intersperse(separator: Item) -> IntersperseIterator[Self] {
        IntersperseIterator(inner: self, separator: separator)
    }
}

/// Lazy-separator companion to `intersperse`.
extend Iterator {

    /// Like `intersperse`, but builds each separator on demand by calling
    /// `separator()`. Use when the separator is expensive or needs to
    /// vary by call.
    ///
    /// # Examples
    ///
    /// ```
    /// var counter = 0;
    /// [1, 2, 3].iter()
    ///     .intersperseWith(separator: || { counter += 1; counter * 10 })
    ///     .collect();   // [1, 10, 2, 20, 3]
    /// ```
    public func intersperseWith(separator: () -> Item) -> IntersperseWithIterator[Self] {
        IntersperseWithIterator(inner: self, separator: separator)
    }
}

/// `cycle` lives in its own block because it requires the inner iterator
/// to be re-runnable (the adapter copies it on each lap).
extend Iterator {

    /// Restarts iteration from the beginning whenever the inner iterator
    /// is exhausted, producing an infinite sequence. Always combine with
    /// `take` (or another short-circuiting consumer) — otherwise the
    /// result is unbounded.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].iter().cycle().take(count: 7).collect();
    /// // [1, 2, 3, 1, 2, 3, 1]
    /// ```
    public func cycle() -> CycleIterator[Self] {
        CycleIterator(iter: self)
    }
}

// ============================================================================
// TERMINAL OPERATIONS - COLLECTING
// ============================================================================

/// Terminal operations that consume the iterator and produce a value.
extend Iterator {

    /// Drains the iterator into an `Array[Item]`. Eager and `O(n)`. Use
    /// at the end of an adapter chain to materialise the result.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].iter().filter({ it > 1 }).collect();   // [2, 3]
    /// (1..5).iter().map({ it * it }).collect();        // [1, 4, 9, 16]
    /// ```
    public consuming func collect() -> Array[Item] {
        var result = Array[Item]();
        while let .Some(item) = self.next() {
            result.append(item);
        }
        result
    }

    /// Counts the elements by walking the whole iterator. `O(n)` — for
    /// types that already know their length, prefer
    /// `ExactSizeIterator.remaining`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].iter().filter({ it % 2 == 0 }).count();   // 2
    /// ```
    public consuming func count() -> Int64 {
        var count = 0;
        while let .Some(_) = self.next() {
            count = count + 1;
        }
        count
    }

    /// Splits an iterator of pairs into two parallel arrays. Inverse of
    /// `zip`.
    ///
    /// # Examples
    ///
    /// ```
    /// let pairs = [(1, "a"), (2, "b"), (3, "c")];
    /// let (nums, strs) = pairs.iter().unzip();
    /// // nums = [1, 2, 3], strs = ["a", "b", "c"]
    /// ```
    public consuming func unzip[A, B]() -> (Array[A], Array[B]) where Item = (A, B) {
        var left = Array[A]();
        var right = Array[B]();
        while let .Some(pair) = self.next() {
            left.append(pair.0);
            right.append(pair.1);
        }
        (left, right)
    }
}

// ============================================================================
// TERMINAL OPERATIONS - FOLDING
// ============================================================================

/// `fold`/`reduce` family — collapse an iterator to a single value.
extend Iterator {

    /// Left fold — start at `initial` and walk left to right, applying
    /// `combine(acc, element)`. Returns `initial` for an empty iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4].iter().fold(initial: 0, combine: |acc, x| acc + x);   // 10
    /// [1, 2, 3].iter().fold(initial: 1, combine: |acc, x| acc * x);      // 6
    /// [].iter().fold(initial: 42, combine: |acc, x| acc + x);            // 42
    /// ```
    public consuming func fold[Acc](initial initial: Acc, combine combine: (Acc, Item) -> Acc) -> Acc {
        var acc = initial;
        while let .Some(item) = self.next() {
            acc = combine(acc, item);
        }
        acc
    }

    /// Like `fold`, but seeds the accumulator with the first element
    /// instead of taking an explicit `initial`. Returns `None` for an
    /// empty iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4].iter().reduce(combine: |a, b| a + b);   // Some(10)
    /// [5].iter().reduce(combine: |a, b| a + b);            // Some(5)
    /// [].iter().reduce(combine: |a, b| a + b);             // None
    /// ```
    public consuming func reduce(combine combine: (Item, Item) -> Item) -> Item? {
        if let .Some(first) = self.next() {
            .Some(self.fold(initial: first, combine: combine))
        } else {
            .None
        }
    }

    /// Fold with early exit on `Err`. The combine returns `Result`; the
    /// first `Err` halts iteration and is returned. If everything
    /// succeeds, returns `Ok(final accumulator)`.
    ///
    /// # Examples
    ///
    /// ```
    /// // Stop the moment a parse fails
    /// ["1", "2", "3"].iter()
    ///     .tryFold(initial: 0, combine: |acc, s| {
    ///         match Int64.parse(s) {
    ///             .Some(n) => .Ok(acc + n),
    ///             .None    => .Err("parse error")
    ///         }
    ///     });   // Ok(6)
    ///
    /// ["1", "bad", "3"].iter()
    ///     .tryFold(initial: 0, combine: |acc, s| {
    ///         match Int64.parse(s) {
    ///             .Some(n) => .Ok(acc + n),
    ///             .None    => .Err("parse error")
    ///         }
    ///     });   // Err("parse error")
    /// ```
    public mutating func tryFold[Acc, E](initial initial: Acc, combine combine: (Acc, Item) -> Result[Acc, E]) -> Result[Acc, E] {
        var acc = initial;
        while let .Some(item) = self.next() {
            match combine(acc, item) {
                .Ok(newAcc) => acc = newAcc,
                .Err(err) => return .Err(err)
            }
        }
        .Ok(acc)
    }

    /// `forEach` with early exit on `Err`. Mirror of `tryFold` for the
    /// "do something with each element" shape.
    ///
    /// # Examples
    ///
    /// ```
    /// files.iter().tryForEach({ (path) in
    ///     File.delete(path)   // Result[(), IoError]
    /// });   // stops on first failure
    /// ```
    public mutating func tryForEach[E](action: (Item) -> Result[(), E]) -> Result[(), E] {
        self.tryFold(initial: (), combine: { (_, item) in action(item) })
    }
}

// ============================================================================
// TERMINAL OPERATIONS - ITERATION
// ============================================================================

/// Side-effect-only consumers.
extend Iterator {

    /// Calls `action` on every element, discarding return values. Use
    /// `tryForEach` if you need to short-circuit on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].iter().forEach({ print(it) });
    /// ```
    public consuming func forEach(action: (Item) -> ()) {
        while let .Some(item) = self.next() {
            action(item);
        }
    }
}

// ============================================================================
// TERMINAL OPERATIONS - PREDICATES
// ============================================================================

/// Boolean reductions that short-circuit.
extend Iterator {

    /// True if any element satisfies `predicate`. Stops at the first
    /// match. False for an empty iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4].iter().any({ it > 3 });    // true (stops at 4)
    /// [1, 2, 3].iter().any({ it > 10 });      // false
    /// [].iter().any({ true });                // false
    /// ```
    public mutating func any(predicate: (Item) -> Bool) -> Bool {
        while let .Some(item) = self.next() {
            if predicate(item) {
                return true
            }
        }
        false
    }

    /// True if every element satisfies `predicate`. Stops at the first
    /// failure. True for an empty iterator (vacuous truth).
    ///
    /// # Examples
    ///
    /// ```
    /// [2, 4, 6].iter().all({ it % 2 == 0 });   // true
    /// [2, 3, 4].iter().all({ it % 2 == 0 });   // false (stops at 3)
    /// [].iter().all({ false });                // true (empty)
    /// ```
    public mutating func all(predicate: (Item) -> Bool) -> Bool {
        while let .Some(item) = self.next() {
            if not predicate(item) {
                return false
            }
        }
        true
    }
}

// ============================================================================
// TERMINAL OPERATIONS - SEARCHING
// ============================================================================

/// Element-locating consumers — all short-circuit on hit.
extend Iterator {

    /// First element matching `predicate`, or `None`. Stops at the first
    /// match.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].iter().find({ it > 3 });   // Some(4)
    /// [1, 2, 3].iter().find({ it > 10 });        // None
    /// ```
    public mutating func find(predicate: (Item) -> Bool) -> Item? {
        while let .Some(item) = self.next() {
            if predicate(item) {
                return .Some(item)
            }
        }
        .None
    }

    /// Index of the first element matching `predicate`, or `None`.
    /// Mirror of `find` for positions.
    ///
    /// # Examples
    ///
    /// ```
    /// ["a", "b", "c"].iter().position({ it == "b" });   // Some(1)
    /// [1, 2, 3].iter().position({ it > 10 });           // None
    /// ```
    public mutating func position(predicate: (Item) -> Bool) -> Int64? {
        var index = 0;
        while let .Some(item) = self.next() {
            if predicate(item) {
                return .Some(index)
            }
            index = index + 1;
        }
        .None
    }

    /// Returns the element at index `n` (zero-based), consuming
    /// everything up to and including it. `None` if `n` is past the end.
    ///
    /// # Examples
    ///
    /// ```
    /// [10, 20, 30, 40].iter().nth(n: 2);   // Some(30)
    /// [10, 20].iter().nth(n: 5);           // None
    /// [10, 20, 30].iter().nth(n: 0);       // Some(10)
    /// ```
    public mutating func nth(n: Int64) -> Item? {
        var index = 0;
        while let .Some(item) = self.next() {
            if index == n {
                return .Some(item)
            }
            index = index + 1;
        }
        .None
    }

    /// Last element, or `None` if empty. Consumes the entire iterator —
    /// `O(n)` even for sequences whose last element is cheap to address
    /// directly.
    public consuming func last() -> Item? {
        var last: Item? = .None;
        while let .Some(item) = self.next() {
            last = .Some(item);
        }
        last
    }

    /// First element, or `None` if empty. Consumes only the first
    /// element. Equivalent to `next()`, but reads more naturally as a
    /// terminal.
    public mutating func first() -> Item? {
        self.next()
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - EQUATABLE
// ============================================================================

/// `contains` lives behind `Item: Equatable` so the dispatch can compare
/// elements with `==`.
extend Iterator where Item: Equatable {

    /// True if any element equals `element`. Short-circuits.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].iter().contains(element: 2);   // true
    /// [1, 2, 3].iter().contains(element: 5);   // false
    /// ```
    public mutating func contains(element: Item) -> Bool {
        self.any({ (item) in item.equals(element) })
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - COMPARABLE
// ============================================================================

/// `min`/`max`/sort-checks/sorted require an ordering on `Item`.
extend Iterator where Item: Comparable {

    /// Smallest element, or `None` for an empty iterator. Ties go to the
    /// first occurrence.
    ///
    /// # Examples
    ///
    /// ```
    /// [3, 1, 4, 1, 5].iter().min();   // Some(1)
    /// [].iter().min();                // None
    /// ```
    public consuming func min() -> Item? {
        self.reduce(combine: { (a, b) in if a.compare(b) == Ordering.Less { a } else { b } })
    }

    /// Largest element, or `None` for an empty iterator. Ties go to the
    /// first occurrence.
    public consuming func max() -> Item? {
        self.reduce(combine: { (a, b) in if a.compare(b) == Ordering.Greater { a } else { b } })
    }

    /// Collects into an `Array[Item]`, sorted ascending. Eager and
    /// `O(n log n)` — calls `Array.sort(by:)` after `collect()`.
    ///
    /// # Examples
    ///
    /// ```
    /// [3, 1, 4, 1, 5].iter().sorted();                       // [1, 1, 3, 4, 5]
    /// [3, 1, 2].iter().filter({ it > 1 }).sorted();          // [2, 3]
    /// ```
    public consuming func sorted() -> Array[Item] {
        var arr = self.collect();
        arr.sort(by: { (a, b) in a.compare(b) == Ordering.Less });
        arr
    }

    /// The element with the smallest `key(element)`. Ties go to the
    /// first occurrence.
    ///
    /// # Examples
    ///
    /// ```
    /// let people = [("Alice", 30), ("Bob", 25), ("Charlie", 35)];
    /// people.iter().minBy(key: { it.1 });   // Some(("Bob", 25))
    /// ```
    public consuming func minBy[K](key: (Item) -> K) -> Item? where K: Comparable {
        if let .Some(first) = self.next() {
            var minItem = first;
            var minKey = key(first);
            while let .Some(item) = self.next() {
                let itemKey = key(item);
                if itemKey.compare(minKey) == Ordering.Less {
                    minItem = item;
                    minKey = itemKey;
                }
            }
            .Some(minItem)
        } else {
            .None
        }
    }

    /// The element with the largest `key(element)`. Mirror of `minBy`.
    public consuming func maxBy[K](key: (Item) -> K) -> Item? where K: Comparable {
        if let .Some(first) = self.next() {
            var maxItem = first;
            var maxKey = key(first);
            while let .Some(item) = self.next() {
                let itemKey = key(item);
                if itemKey.compare(maxKey) == Ordering.Greater {
                    maxItem = item;
                    maxKey = itemKey;
                }
            }
            .Some(maxItem)
        } else {
            .None
        }
    }

    /// True if elements come out in ascending order. True for empty or
    /// single-element iterators (vacuous). Short-circuits on the first
    /// out-of-order pair.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].iter().isSorted();   // true
    /// [1, 3, 2, 4, 5].iter().isSorted();   // false
    /// [1, 1, 2, 2, 3].iter().isSorted();   // true (equal allowed)
    /// ```
    public consuming func isSorted() -> Bool {
        if let .Some(first) = self.next() {
            var prev = first;
            while let .Some(item) = self.next() {
                if item.compare(prev) == Ordering.Less {
                    return false
                }
                prev = item;
            }
        }
        true
    }

    /// True if elements come out in descending order. Mirror of
    /// `isSorted`.
    public consuming func isSortedDescending() -> Bool {
        if let .Some(first) = self.next() {
            var prev = first;
            while let .Some(item) = self.next() {
                if item.compare(prev) == Ordering.Greater {
                    return false
                }
                prev = item;
            }
        }
        true
    }
}

/// Sort-check variants that don't require `Item: Comparable`.
extend Iterator {

    /// True if every adjacent pair satisfies `comparator(prev, next)` —
    /// i.e. they are already in the order `comparator` defines.
    ///
    /// # Examples
    ///
    /// ```
    /// // Descending check
    /// [5, 4, 3, 2, 1].iter().isSorted(by: |a, b| a >= b);   // true
    /// // By absolute value
    /// [-1, 2, -3, 4].iter().isSorted(by: |a, b| a.abs() <= b.abs());   // true
    /// ```
    public consuming func isSorted(by comparator: (Item, Item) -> Bool) -> Bool {
        if let .Some(first) = self.next() {
            var prev = first;
            while let .Some(item) = self.next() {
                if not comparator(prev, item) {
                    return false
                }
                prev = item;
            }
        }
        true
    }

    /// True if elements are sorted ascending by `key(element)`. Sugar
    /// over `isSorted(by:)` for the common "by-key" shape.
    ///
    /// # Examples
    ///
    /// ```
    /// let words = ["a", "bb", "ccc"];
    /// words.iter().isSortedBy(key: { it.count });   // true
    /// ```
    public consuming func isSortedBy[K](key: (Item) -> K) -> Bool where K: Comparable {
        self.isSorted(by: { (a, b) in key(a).compare(key(b)) != Ordering.Greater })
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - NUMERIC
// ============================================================================

/// `sum` is gated on `Item: Addable` so we have a `zero` and a `+`.
extend Iterator where Item: Addable, Item.Output = Item {

    /// Sum of every element. Returns `Item.zero` for an empty iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].iter().sum();    // 15
    /// [1.5, 2.5, 3.0].iter().sum();    // 7.0
    /// [].iter().sum();                 // 0
    /// ```
    public consuming func sum() -> Item {
        self.fold(initial: Item.zero, combine: { (acc, x) in acc.add(x) })
    }
}

/// `product` is gated on `Item: Multipliable` so we have a `one` and a `*`.
extend Iterator where Item: Multipliable, Item.Output = Item {

    /// Product of every element. Returns `Item.one` for an empty
    /// iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].iter().product();   // 120
    /// (1..=5).iter().product();           // 120  (5!)
    /// [].iter().product();                // 1
    /// ```
    public consuming func product() -> Item {
        self.fold(initial: Item.one, combine: { (acc, x) in acc.multiply(x) })
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - NESTED ITERATORS
// ============================================================================

/// `flatten` for the case where `Item` is itself an iterator.
extend Iterator where Item: Iterator {

    /// Concatenates the inner iterators into one flat stream. Each inner
    /// iterator is fully drained before moving to the next. The
    /// already-have-iterators counterpart of `flatMap`.
    ///
    /// # Examples
    ///
    /// ```
    /// let nested = [[1, 2], [3, 4], [5]].iter().map({ it.iter() });
    /// nested.flatten().collect();   // [1, 2, 3, 4, 5]
    /// ```
    public func flatten() -> FlattenIterator[Self] {
        FlattenIterator(inner: self)
    }
}

// ============================================================================
// DOUBLE-ENDED ITERATOR EXTENSIONS
// ============================================================================

/// Reverse-iteration support for double-ended iterators.
extend DoubleEndedIterator {

    /// Yields elements back-to-front by pulling `nextBack()` instead of
    /// `next()`. `O(1)` to construct — no buffering.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].iter().rev().collect();                        // [5, 4, 3, 2, 1]
    /// [1, 2, 3, 4, 5].iter().rev().take(count: 3).collect();         // [5, 4, 3]
    /// [1, 2, 3, 4, 5].iter().rev().find({ it % 2 == 0 });            // Some(4)
    /// ```
    public func rev() -> RevIterator[Self] {
        RevIterator(inner: self)
    }
}

/// `RevIterator` is itself double-ended when the wrapped iterator is, so
/// `rev().rev()` gives back the original direction in `O(1)`.
extend RevIterator[I]: DoubleEndedIterator where I: DoubleEndedIterator {

    /// Pulls from the back of the inner iterator (which is the front of
    /// the reversed view).
    public mutating func next() -> I.Item? {
        self.inner.nextBack()
    }

    /// Pulls from the front of the inner iterator (which is the back of
    /// the reversed view).
    public mutating func nextBack() -> I.Item? {
        self.inner.next()
    }
}

/// Convenience helper available on every `ExactSizeIterator`.
extend ExactSizeIterator {

    /// True when no elements remain. Equivalent to `remaining == 0`.
    public func isEmpty() -> Bool {
        self.remaining == 0
    }
}
