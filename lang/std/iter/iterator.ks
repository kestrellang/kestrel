// Core iterator protocols for lazy sequence processing

module std.iter

import std.result.(Optional, Result)
import std.core.(Bool, Equatable, Comparable, Addable, Multipliable, Copyable, Cloneable, Ordering)
import std.num.(Int64)
import std.collections.(Array)

// ============================================================================
// CORE PROTOCOLS
// ============================================================================

/// Protocol for types that produce a sequence of values.
///
/// Iterators are the foundation of lazy sequence processing in Kestrel.
/// They produce elements one at a time via the `next()` method until exhausted.
///
/// Example:
///     struct CountDown: Iterator {
///         type Item = Int64
///         var current: Int64
///
///         mutating func next() -> Int64? {
///             if current <= 0 { return None }
///             let value = current
///             current -= 1
///             return Some(value)
///         }
///     }
@builtin(.IteratorProtocol)
public protocol Iterator {
    /// The type of elements yielded by this iterator.
    type Item

    /// Returns the next element, or None if the sequence is exhausted.
    ///
    /// Once None is returned, subsequent calls should continue to return None.
    @builtin(.IteratorNextMethod)
    mutating func next() -> Item?
}

/// Protocol for types that can produce an iterator.
///
/// Enables for-in loops and other iteration constructs.
/// Collections typically implement Iterable to allow iteration over their elements.
///
/// Example:
///     for item in myCollection {
///         // item is each element from myCollection.iter()
///     }
@builtin(.IterableProtocol)
public protocol Iterable {
    /// The type of elements produced by iteration.
    type Item

    /// The type of iterator that will be produced.
    type Iter: Iterator where Iter.Item = Item

    /// Creates an iterator over this collection's elements.
    @builtin(.IterableIterMethod)
    func iter() -> Iter
}

// ============================================================================
// ITERATOR IS ITERABLE
// ============================================================================

/// Extension making all Iterators also Iterable.
///
/// An iterator can serve as its own iterable, returning itself.
/// This allows iterators to be used directly in for-in loops.
extend Iterator: Iterable {
    type Iterable.Item = Self.Item
    type Iterable.Iter = Self

    /// Returns self, allowing an iterator to be used where an iterable is expected.
    func iter() -> Self { self }
}

/// Protocol for iterators that can traverse from both ends.
///
/// Double-ended iterators can yield elements from the back as well as the front,
/// enabling efficient reverse iteration without collecting into an array first.
///
/// Example:
///     struct Range: DoubleEndedIterator {
///         type Item = Int64
///         var start: Int64
///         var end: Int64
///
///         mutating func next() -> Int64? {
///             if start >= end { return None }
///             let value = start
///             start += 1
///             return Some(value)
///         }
///
///         mutating func nextBack() -> Int64? {
///             if start >= end { return None }
///             end -= 1
///             return Some(end)
///         }
///     }
///
/// Usage:
///     var range = Range(start: 1, end: 4)
///     range.next()      // Some(1)
///     range.nextBack()  // Some(3)
///     range.next()      // Some(2)
///     range.nextBack()  // None (start >= end)
public protocol DoubleEndedIterator: Iterator {
    /// Returns the next element from the back, or None if exhausted.
    ///
    /// Repeated calls will return elements in reverse order until the
    /// iterator is exhausted. Can be interleaved with `next()` calls.
    mutating func nextBack() -> Item?
}

/// Protocol for iterators that know their exact remaining length.
///
/// This enables optimizations like pre-allocating exact capacity when
/// collecting into an array.
///
/// Example:
///     struct ArrayIterator[T]: ExactSizeIterator {
///         type Item = T
///         var ptr: Pointer[T]
///         var remaining: Int64
///
///         var remaining: Int64 { get { remaining } }
///
///         mutating func next() -> T? {
///             if remaining == 0 { return None }
///             let value = ptr.pointee
///             ptr = ptr.advanced(by: 1)
///             remaining -= 1
///             return Some(value)
///         }
///     }
public protocol ExactSizeIterator: Iterator {
    /// Returns the exact number of remaining elements.
    ///
    /// This value decreases by 1 for each `next()` call that returns Some.
    /// When this returns 0, `next()` will return None.
    var remaining: Int64 { get };
}

// ============================================================================
// TRANSFORMATION ADAPTERS
// ============================================================================

/// Extension providing lazy transformation methods.
///
/// These methods create new iterators that transform elements on-the-fly
/// without allocating intermediate collections.
extend Iterator {

    /// Transforms each element using the given function.
    ///
    /// Lazy - transformation happens as elements are consumed.
    ///
    /// Example:
    ///     [1, 2, 3].iter().map(|x| x * 2).collect()  // [2, 4, 6]
    ///     ["hello", "world"].iter().map(|s| s.count).collect()  // [5, 5]
    public func map[U](transform: (Item) -> U) -> MapIterator[Self, U] {
        MapIterator(inner: self, transform: transform)
    }

    /// Yields only elements satisfying the predicate.
    ///
    /// Lazy - elements are tested as they are consumed.
    ///
    /// Example:
    ///     [1, 2, 3, 4, 5].iter().filter(|x| x % 2 == 0).collect()  // [2, 4]
    ///     ["", "a", "", "b"].iter().filter(|s| !s.isEmpty).collect()  // ["a", "b"]
    public func filter(predicate: (Item) -> Bool) -> FilterIterator[Self] {
        FilterIterator(inner: self, predicate: predicate)
    }

    /// Transforms elements and filters out None results in one step.
    ///
    /// Combines map and filter - useful when transformation might fail.
    ///
    /// Example:
    ///     ["1", "two", "3"].iter()
    ///         .filterMap(|s| Int64.parse(s))
    ///         .collect()  // [1, 3]
    public func filterMap[U](transform: (Item) -> U?) -> FilterMapIterator[Self, U] {
        FilterMapIterator(inner: self, transform: transform)
    }



    /// Pairs each element with its zero-based index.
    ///
    /// Example:
    ///     ["a", "b", "c"].iter().enumerate().collect()
    ///     // [(0, "a"), (1, "b"), (2, "c")]
    ///
    ///     for (i, item) in arr.iter().enumerate() {
    ///         print("Index \(i): \(item)")
    ///     }
    public func enumerate() -> EnumerateIterator[Self] {
        EnumerateIterator(inner: self)
    }

    /// Transforms each element into an iterator and flattens the results.
    ///
    /// Each element is passed to transform, which returns an iterator.
    /// All elements from all resulting iterators are yielded sequentially.
    ///
    /// Example:
    ///     [[1, 2], [3, 4], [5]].iter()
    ///         .flatMap(|arr| arr.iter())
    ///         .collect()  // [1, 2, 3, 4, 5]
    ///
    ///     // Get all characters from all words
    ///     ["hello", "world"].iter()
    ///         .flatMap(|s| s.chars())
    ///         .collect()  // ['h', 'e', 'l', 'l', 'o', 'w', 'o', 'r', 'l', 'd']
    ///
    ///     // Filter and expand in one step
    ///     [1, 2, 3].iter()
    ///         .flatMap(|x| if x % 2 == 0 { [x, x].iter() } else { [].iter() })
    ///         .collect()  // [2, 2]
    public func flatMap[U](transform: (Item) -> U) -> FlatMapIterator[Self, U] where U: Iterator {
        FlatMapIterator(inner: self, transform: transform)
    }

    /// Yields running accumulator values, like fold but with intermediate results.
    ///
    /// Starts with initial value, applies combine for each element, and yields
    /// each intermediate accumulator value. Useful for running totals, prefixes, etc.
    ///
    /// Example:
    ///     // Running sum
    ///     [1, 2, 3, 4].iter()
    ///         .scan(initial: 0, combine: |acc, x| acc + x)
    ///         .collect()  // [1, 3, 6, 10]
    ///
    ///     // Running product
    ///     [1, 2, 3, 4].iter()
    ///         .scan(initial: 1, combine: |acc, x| acc * x)
    ///         .collect()  // [1, 2, 6, 24]
    ///
    ///     // Track state while iterating
    ///     "aabbc".chars().iter()
    ///         .scan(initial: 0, combine: |count, c| if c == 'b' { count + 1 } else { count })
    ///         .collect()  // [0, 0, 1, 2, 2]
    public func scan[Acc](initial: Acc, combine: (Acc, Item) -> Acc) -> ScanIterator[Self, Acc] {
        ScanIterator(inner: self, initial: initial, combine: combine)
    }
}

// TODO: Ref[T] type not yet implemented
// /// Extension for iterators over copyable references.
// extend Iterator where Item = Ref[T], T: Copyable {
//     /// Copies each referenced element, yielding owned values.
//     public func copied() -> CopiedIterator[Self, T] {
//         CopiedIterator(inner: self)
//     }
// }

// /// Extension for iterators over cloneable references.
// extend Iterator where Item = Ref[T], T: Cloneable {
//     /// Clones each referenced element, yielding owned values.
//     public func cloned() -> ClonedIterator[Self, T] {
//         ClonedIterator(inner: self)
//     }
// }

// ============================================================================
// LIMITING ADAPTERS
// ============================================================================

/// Extension providing methods to limit iteration.
extend Iterator {

    /// Takes only the first count elements.
    ///
    /// Stops iteration after yielding count elements, even if more are available.
    ///
    /// Example:
    ///     [1, 2, 3, 4, 5].iter().take(count: 3).collect()  // [1, 2, 3]
    ///     [1, 2].iter().take(count: 10).collect()  // [1, 2] - fewer available
    public func take(count: Int64) -> TakeIterator[Self] {
        TakeIterator(inner: self, count: count)
    }

    /// Takes elements while predicate returns true, then stops.
    ///
    /// Stops at the first element that doesn't satisfy the predicate.
    /// Does not include that element.
    ///
    /// Example:
    ///     [1, 2, 3, 4, 1, 2].iter()
    ///         .takeWhile(|x| x < 4)
    ///         .collect()  // [1, 2, 3] - stops at 4
    public func takeWhile(predicate: (Item) -> Bool) -> TakeWhileIterator[Self] {
        TakeWhileIterator(inner: self, predicate: predicate)
    }

    /// Skips the first count elements.
    ///
    /// Example:
    ///     [1, 2, 3, 4, 5].iter().skip(count: 2).collect()  // [3, 4, 5]
    ///     [1, 2].iter().skip(count: 10).collect()  // [] - all skipped
    public func skip(count: Int64) -> SkipIterator[Self] {
        SkipIterator(inner: self, count: count)
    }

    /// Skips elements while predicate returns true, then yields all remaining.
    ///
    /// Once the predicate returns false, yields that element and all subsequent
    /// elements without further testing.
    ///
    /// Example:
    ///     [1, 2, 3, 4, 1, 2].iter()
    ///         .skipWhile(|x| x < 3)
    ///         .collect()  // [3, 4, 1, 2] - note 1, 2 at end included
    public func skipWhile(predicate: (Item) -> Bool) -> SkipWhileIterator[Self] {
        SkipWhileIterator(inner: self, predicate: predicate)
    }
}

// ============================================================================
// COMBINING ADAPTERS
// ============================================================================

/// Extension providing methods to combine iterators.
extend Iterator {

    /// Pairs elements from this iterator with another.
    ///
    /// Stops when either iterator is exhausted.
    ///
    /// Example:
    ///     let names = ["Alice", "Bob", "Charlie"]
    ///     let ages = [30, 25, 35]
    ///     names.iter().zip(other: ages.iter()).collect()
    ///     // [("Alice", 30), ("Bob", 25), ("Charlie", 35)]
    ///
    ///     // Different lengths - stops at shorter
    ///     [1, 2, 3].iter().zip(other: ["a", "b"].iter()).collect()
    ///     // [(1, "a"), (2, "b")]
    public func zip[Other](other: Other) -> ZipIterator[Self, Other] where Other: Iterator {
        ZipIterator(first: self, second: other)
    }

    /// Chains another iterator after this one.
    ///
    /// Yields all elements from self, then all from other.
    ///
    /// Example:
    ///     [1, 2].iter().chain(other: [3, 4].iter()).collect()  // [1, 2, 3, 4]
    ///
    ///     // Concatenate multiple iterators
    ///     a.iter().chain(other: b.iter()).chain(other: c.iter())
    public func chain[Other](other: Other) -> ChainIterator[Self, Other] where Other: Iterator, Other.Item = Item {
        ChainIterator(first: self, second: other)
    }
}

// ============================================================================
// UTILITY ADAPTERS
// ============================================================================

/// Extension providing utility adapter methods.
extend Iterator {

    /// Wraps this iterator to allow peeking at the next element without consuming it.
    ///
    /// Example:
    ///     var iter = [1, 2, 3].iter().peekable()
    ///     iter.peek()  // Some(1) - doesn't consume
    ///     iter.peek()  // Some(1) - still 1
    ///     iter.next()  // Some(1) - now consumed
    ///     iter.peek()  // Some(2)
    public func peekable() -> PeekableIterator[Self] {
        PeekableIterator(inner: self)
    }

    /// Stops permanently after yielding None once.
    ///
    /// Some iterators might return Some again after returning None (non-fused).
    /// This adapter guarantees once None is returned, it's always None.
    ///
    /// Example:
    ///     var iter = possiblyNonFusedIterator.fuse()
    ///     // After iter.next() returns None, it will always return None
    public func fuse() -> FuseIterator[Self] {
        FuseIterator(inner: self)
    }

    /// Calls a function on each element as it passes through.
    ///
    /// The iterator chain continues unchanged; this is purely for side effects.
    /// Useful for debugging, logging, or observing values mid-chain.
    ///
    /// Example:
    ///     [1, 2, 3].iter()
    ///         .inspect(|x| print("before filter: \(x)"))
    ///         .filter(|x| x > 1)
    ///         .inspect(|x| print("after filter: \(x)"))
    ///         .collect()
    ///     // Prints: before filter: 1
    ///     //         before filter: 2
    ///     //         after filter: 2
    ///     //         before filter: 3
    ///     //         after filter: 3
    ///     // Returns: [2, 3]
    public func inspect(inspector: (Item) -> ()) -> InspectIterator[Self] {
        InspectIterator(inner: self, inspector: inspector)
    }

    /// Yields every nth element, starting with the first.
    ///
    /// Panics if step is 0.
    ///
    /// Example:
    ///     [0, 1, 2, 3, 4, 5, 6].iter().stepBy(n: 2).collect()
    ///     // [0, 2, 4, 6]
    ///
    ///     [0, 1, 2, 3, 4].iter().stepBy(n: 3).collect()
    ///     // [0, 3]
    ///
    ///     // Useful for sampling
    ///     largeDataset.iter().stepBy(n: 100).collect()  // every 100th element
    public func stepBy(n: Int64) -> StepByIterator[Self] {
        StepByIterator(inner: self, step: n)
    }
}

// TODO: IntersperseIterator commented out due to I.Item: Cloneable constraint issues
// /// Intersperse adapter requires Cloneable for the separator.
// extend Iterator where Item: Cloneable {
//     public func intersperse(separator: Item) -> IntersperseIterator[Self] {
//         IntersperseIterator(inner: self, separator: separator)
//     }
// }

/// Intersperse with lazy separator generation.
extend Iterator {

    /// Inserts separators generated by a function between each pair of elements.
    ///
    /// Useful when the separator is expensive to create or needs to vary.
    ///
    /// Example:
    ///     var counter = 0
    ///     [1, 2, 3].iter()
    ///         .intersperseWith(separator: || { counter += 1; counter * 10 })
    ///         .collect()
    ///     // [1, 10, 2, 20, 3]
    public func intersperseWith(separator: () -> Item) -> IntersperseWithIterator[Self] {
        IntersperseWithIterator(inner: self, separator: separator)
    }
}

/// Cycle adapter requires Cloneable to restart iteration.
extend Iterator where Self: Cloneable {

    /// Repeats this iterator forever by cloning when exhausted.
    ///
    /// WARNING: Creates an infinite iterator. Use with `take()` to limit.
    ///
    /// Example:
    ///     [1, 2, 3].iter().cycle().take(count: 7).collect()
    ///     // [1, 2, 3, 1, 2, 3, 1]
    public func cycle() -> CycleIterator[Self] {
        CycleIterator(iter: self)
    }
}

// ============================================================================
// TERMINAL OPERATIONS - COLLECTING
// ============================================================================

/// Extension providing terminal operations that consume the iterator.
extend Iterator {

    /// Collects all elements into an array.
    ///
    /// Consumes the entire iterator.
    ///
    /// Example:
    ///     [1, 2, 3].iter().filter(|x| x > 1).collect()  // [2, 3]
    ///     (1..5).iter().map(|x| x * x).collect()  // [1, 4, 9, 16]
    public func collect() -> Array[Item] {
        var result = Array[Item]();
        while let .Some(item) = self.next() {
            result.append(item);
        }
        result
    }

    /// Returns the number of elements.
    ///
    /// Consumes the entire iterator.
    ///
    /// Example:
    ///     [1, 2, 3, 4, 5].iter().filter(|x| x % 2 == 0).count()  // 2
    public func count() -> Int64 {
        var count = Int64(intLiteral: 0);
        while let .Some(_) = self.next() {
            count = count + Int64(intLiteral: 1);
        }
        count
    }

    // TODO: Restore when method-level tuple constraints are supported.
    // /// Splits an iterator of pairs into two arrays.
    // ///
    // /// Takes an iterator over pairs `(A, B)` and returns two arrays:
    // /// one containing all the first elements and one containing all the second elements.
    // ///
    // /// Example:
    // ///     let pairs = [(1, "a"), (2, "b"), (3, "c")];
    // ///     let (nums, strs) = pairs.iter().unzip();
    // ///     // nums = [1, 2, 3], strs = ["a", "b", "c"]
    // public func unzip[A, B]() -> (Array[A], Array[B]) where Item = (A, B) {
    //     var left = Array[A]();
    //     var right = Array[B]();
    //     while let .Some(pair) = self.next() {
    //         let tuple: (A, B) = pair;
    //         left.append(tuple.0);
    //         right.append(tuple.1);
    //     }
    //     (left, right)
    // }
}

// ============================================================================
// TERMINAL OPERATIONS - FOLDING
// ============================================================================

/// Extension providing fold/reduce operations.
extend Iterator {

    /// Reduces elements to a single value using an accumulator.
    ///
    /// Starts with initial value and applies combine(accumulator, element)
    /// for each element. Returns initial if the iterator is empty.
    ///
    /// Example:
    ///     [1, 2, 3, 4].iter().fold(initial: 0, combine: |acc, x| acc + x)  // 10
    ///     [1, 2, 3].iter().fold(initial: 1, combine: |acc, x| acc * x)  // 6
    ///     [].iter().fold(initial: 42, combine: |acc, x| acc + x)  // 42
    public func fold[Acc](initial initial: Acc, combine combine: (Acc, Item) -> Acc) -> Acc {
        var acc = initial;
        while let .Some(item) = self.next() {
            acc = combine(acc, item);
        }
        acc
    }

    /// Reduces elements using the first element as the initial accumulator.
    ///
    /// Returns None if the iterator is empty.
    ///
    /// Example:
    ///     [1, 2, 3, 4].iter().reduce(combine: |a, b| a + b)  // Some(10)
    ///     [5].iter().reduce(combine: |a, b| a + b)  // Some(5)
    ///     [].iter().reduce(combine: |a, b| a + b)  // None
    public func reduce(combine combine: (Item, Item) -> Item) -> Item? {
        if let .Some(first) = self.next() {
            .Some(self.fold(initial: first, combine: combine))
        } else {
            .None
        }
    }

    /// Folds with early exit on error.
    ///
    /// Like `fold`, but the combine function returns a Result. If any call
    /// returns Err, iteration stops immediately and that error is returned.
    /// If all calls succeed, returns Ok with the final accumulator.
    ///
    /// Example:
    ///     // Parse and sum, stopping on first parse error
    ///     ["1", "2", "3"].iter()
    ///         .tryFold(initial: 0, combine: |acc, s| {
    ///             match Int64.parse(s) {
    ///                 Some(n) => Ok(acc + n),
    ///                 None => Err("parse error")
    ///             }
    ///         })  // Ok(6)
    ///
    ///     ["1", "bad", "3"].iter()
    ///         .tryFold(initial: 0, combine: |acc, s| {
    ///             match Int64.parse(s) {
    ///                 Some(n) => Ok(acc + n),
    ///                 None => Err("parse error")
    ///             }
    ///         })  // Err("parse error") - stops at "bad"
    ///
    ///     // Early exit for performance
    ///     (1..1000000).iter()
    ///         .tryFold(initial: 0, combine: |acc, x| {
    ///             if acc > 100 { Err(acc) }  // stop early
    ///             else { Ok(acc + x) }
    ///         })
    public func tryFold[Acc, E](initial initial: Acc, combine combine: (Acc, Item) -> Result[Acc, E]) -> Result[Acc, E] {
        var acc = initial;
        while let .Some(item) = self.next() {
            match combine(acc, item) {
                .Ok(newAcc) => acc = newAcc,
                .Err(err) => return .Err(err)
            }
        }
        .Ok(acc)
    }

    /// Calls action on each element with early exit on error.
    ///
    /// Like `forEach`, but the action returns a Result. Stops on first Err.
    ///
    /// Example:
    ///     files.iter().tryForEach(|path| {
    ///         File.delete(path)  // Returns Result[Void, IoError]
    ///     })  // Stops on first deletion failure
    public func tryForEach[E](action: (Item) -> Result[(), E]) -> Result[(), E] {
        self.tryFold(initial: (), combine: { (_, item) in action(item) })
    }
}

// ============================================================================
// TERMINAL OPERATIONS - ITERATION
// ============================================================================

/// Extension providing iteration operations.
extend Iterator {

    /// Calls action on each element.
    ///
    /// Consumes the entire iterator. Use when you need side effects.
    ///
    /// Example:
    ///     [1, 2, 3].iter().forEach(|x| print(x))
    public func forEach(action: (Item) -> ()) {
        while let .Some(item) = self.next() {
            action(item);
        }
    }
}

// ============================================================================
// TERMINAL OPERATIONS - PREDICATES
// ============================================================================

/// Extension providing predicate operations.
extend Iterator {

    /// Returns true if any element satisfies the predicate.
    ///
    /// Short-circuits on first match (doesn't consume remaining elements).
    /// Returns false for an empty iterator.
    ///
    /// Example:
    ///     [1, 2, 3, 4].iter().any(|x| x > 3)  // true (stops at 4)
    ///     [1, 2, 3].iter().any(|x| x > 10)    // false
    ///     [].iter().any(|x| true)             // false
    public func any(predicate: (Item) -> Bool) -> Bool {
        while let .Some(item) = self.next() {
            if predicate(item) {
                return true
            }
        }
        false
    }

    /// Returns true if all elements satisfy the predicate.
    ///
    /// Short-circuits on first non-match.
    /// Returns true for an empty iterator (vacuous truth).
    ///
    /// Example:
    ///     [2, 4, 6].iter().all(|x| x % 2 == 0)  // true
    ///     [2, 3, 4].iter().all(|x| x % 2 == 0)  // false (stops at 3)
    ///     [].iter().all(|x| false)              // true (empty)
    public func all(predicate: (Item) -> Bool) -> Bool {
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

/// Extension providing search operations.
extend Iterator {

    /// Returns the first element satisfying the predicate, or None.
    ///
    /// Short-circuits on first match.
    ///
    /// Example:
    ///     [1, 2, 3, 4, 5].iter().find(|x| x > 3)   // Some(4)
    ///     [1, 2, 3].iter().find(|x| x > 10)        // None
    public func find(predicate: (Item) -> Bool) -> Item? {
        while let .Some(item) = self.next() {
            if predicate(item) {
                return .Some(item)
            }
        }
        .None
    }

    /// Returns the position of the first element satisfying the predicate, or None.
    ///
    /// Short-circuits on first match.
    ///
    /// Example:
    ///     ["a", "b", "c"].iter().position(|s| s == "b")  // Some(1)
    ///     [1, 2, 3].iter().position(|x| x > 10)         // None
    public func position(predicate: (Item) -> Bool) -> Int64? {
        var index = Int64(intLiteral: 0);
        while let .Some(item) = self.next() {
            if predicate(item) {
                return .Some(index)
            }
            index = index + Int64(intLiteral: 1);
        }
        .None
    }

    /// Returns the nth element (zero-indexed), or None if out of bounds.
    ///
    /// Consumes elements up to and including n.
    ///
    /// Example:
    ///     [10, 20, 30, 40].iter().nth(n: 2)  // Some(30)
    ///     [10, 20].iter().nth(n: 5)          // None
    ///     [10, 20, 30].iter().nth(n: 0)      // Some(10)
    public func nth(n: Int64) -> Item? {
        var index = Int64(intLiteral: 0);
        while let .Some(item) = self.next() {
            if index == n {
                return .Some(item)
            }
            index = index + Int64(intLiteral: 1);
        }
        .None
    }

    /// Returns the last element, or None if empty.
    ///
    /// Consumes the entire iterator.
    ///
    /// Example:
    ///     [1, 2, 3].iter().last()  // Some(3)
    ///     [].iter().last()         // None
    public func last() -> Item? {
        var last: Item? = .None;
        while let .Some(item) = self.next() {
            last = .Some(item);
        }
        last
    }

    /// Returns the first element, or None if empty.
    ///
    /// Consumes only the first element.
    ///
    /// Example:
    ///     [1, 2, 3].iter().first()  // Some(1)
    ///     [].iter().first()         // None
    public func first() -> Item? {
        self.next()
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - EQUATABLE
// ============================================================================

/// Extension for iterators with equatable elements.
extend Iterator where Item: Equatable {

    /// Returns true if any element equals the given value.
    ///
    /// Short-circuits on first match.
    ///
    /// Example:
    ///     [1, 2, 3].iter().contains(element: 2)  // true
    ///     [1, 2, 3].iter().contains(element: 5)  // false
    public func contains(element: Item) -> Bool {
        self.any({ (item) in item.equals(element) })
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - COMPARABLE
// ============================================================================

/// Extension for iterators with comparable elements.
extend Iterator where Item: Comparable {

    /// Returns the minimum element, or None if empty.
    ///
    /// Consumes the entire iterator.
    ///
    /// Example:
    ///     [3, 1, 4, 1, 5].iter().min()  // Some(1)
    ///     [].iter().min()               // None
    public func min() -> Item? {
        self.reduce(combine: { (a, b) in if a.compare(b) == Ordering.Less { a } else { b } })
    }

    /// Returns the maximum element, or None if empty.
    ///
    /// Consumes the entire iterator.
    ///
    /// Example:
    ///     [3, 1, 4, 1, 5].iter().max()  // Some(5)
    ///     [].iter().max()               // None
    public func max() -> Item? {
        self.reduce(combine: { (a, b) in if a.compare(b) == Ordering.Greater { a } else { b } })
    }

    /// Collects elements into a sorted array.
    ///
    /// Consumes the entire iterator.
    ///
    /// Example:
    ///     [3, 1, 4, 1, 5].iter().sorted()  // [1, 1, 3, 4, 5]
    ///     [3, 1, 2].iter().filter(|x| x > 1).sorted()  // [2, 3]
    public func sorted() -> Array[Item] {
        var arr = self.collect();
        arr.sort(by: { (a, b) in a.compare(b) == Ordering.Less });
        arr
    }

    /// Returns the element with the minimum value of the key function.
    ///
    /// Consumes the entire iterator. Returns None if empty.
    /// If multiple elements have the minimum key, returns the first.
    ///
    /// Example:
    ///     let people = [("Alice", 30), ("Bob", 25), ("Charlie", 35)]
    ///     people.iter().minBy(key: |p| p.1)  // Some(("Bob", 25))
    ///
    ///     let words = ["hello", "hi", "hey"]
    ///     words.iter().minBy(key: |w| w.count)  // Some("hi")
    ///
    ///     [].iter().minBy(key: |x| x)  // None
    public func minBy[K](key: (Item) -> K) -> Item? where K: Comparable {
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

    /// Returns the element with the maximum value of the key function.
    ///
    /// Consumes the entire iterator. Returns None if empty.
    /// If multiple elements have the maximum key, returns the first.
    ///
    /// Example:
    ///     let people = [("Alice", 30), ("Bob", 25), ("Charlie", 35)]
    ///     people.iter().maxBy(key: |p| p.1)  // Some(("Charlie", 35))
    ///
    ///     let words = ["hello", "hi", "hey"]
    ///     words.iter().maxBy(key: |w| w.count)  // Some("hello")
    ///
    ///     [].iter().maxBy(key: |x| x)  // None
    public func maxBy[K](key: (Item) -> K) -> Item? where K: Comparable {
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

    /// Returns true if the iterator yields elements in sorted (ascending) order.
    ///
    /// Consumes the entire iterator. Returns true for empty or single-element
    /// iterators.
    ///
    /// Example:
    ///     [1, 2, 3, 4, 5].iter().isSorted()  // true
    ///     [1, 3, 2, 4, 5].iter().isSorted()  // false
    ///     [1, 1, 2, 2, 3].iter().isSorted()  // true (equal elements OK)
    ///     [].iter().isSorted()               // true
    ///     [42].iter().isSorted()             // true
    ///
    ///     // Short-circuits on first out-of-order pair
    ///     [1, 0, 2, 3, 4, 5, ...].iter().isSorted()  // false (stops at 1, 0)
    public func isSorted() -> Bool {
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

    /// Returns true if elements are sorted in descending order.
    ///
    /// Example:
    ///     [5, 4, 3, 2, 1].iter().isSortedDescending()  // true
    ///     [5, 3, 4, 2, 1].iter().isSortedDescending()  // false
    public func isSortedDescending() -> Bool {
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

/// Extension for checking sorted order with custom comparators.
extend Iterator {

    /// Returns true if elements are sorted according to the comparator.
    ///
    /// The comparator should return true if the first argument should come
    /// before the second (i.e., they are in the correct order).
    ///
    /// Example:
    ///     // Check descending order
    ///     [5, 4, 3, 2, 1].iter().isSorted(by: |a, b| a >= b)  // true
    ///
    ///     // Check sorted by absolute value
    ///     [-1, 2, -3, 4].iter().isSorted(by: |a, b| a.abs() <= b.abs())  // true
    ///
    ///     // Case-insensitive string sorting
    ///     ["Apple", "banana", "Cherry"].iter()
    ///         .isSorted(by: |a, b| a.lowercase() <= b.lowercase())  // true
    public func isSorted(by comparator: (Item, Item) -> Bool) -> Bool {
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

    /// Returns true if elements are sorted by the given key in ascending order.
    ///
    /// Example:
    ///     let people = [("Alice", 25), ("Bob", 30), ("Charlie", 35)]
    ///     people.iter().isSortedBy(key: |p| p.1)  // true (sorted by age)
    ///
    ///     let words = ["a", "bb", "ccc"]
    ///     words.iter().isSortedBy(key: |w| w.count)  // true (sorted by length)
    public func isSortedBy[K](key: (Item) -> K) -> Bool where K: Comparable {
        self.isSorted(by: { (a, b) in key(a).compare(key(b)) != Ordering.Greater })
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - NUMERIC
// ============================================================================

// TODO: sum() and product() require Addable.zero() and Multipliable.one() which don't exist
// /// Extension for iterators with addable elements.
// extend Iterator where Item: Addable {
//     /// Returns the sum of all elements.
//     public func sum() -> Item {
//         self.fold(initial: Item.zero(), combine: { (acc, x) in acc.add(x) })
//     }
// }

// /// Extension for iterators with multipliable elements.
// extend Iterator where Item: Multipliable {
//     /// Returns the product of all elements.
//     public func product() -> Item {
//         self.fold(initial: Item.one(), combine: { (acc, x) in acc.multiply(x) })
//     }
// }

// ============================================================================
// CONDITIONAL EXTENSIONS - NESTED ITERATORS
// ============================================================================

// TODO: FlattenIterator commented out due to I.Item: Iterator constraint issues
// /// Extension for iterators of iterators.
// extend Iterator where Item: Iterator {
//     public func flatten() -> FlattenIterator[Self] {
//         FlattenIterator(inner: self)
//     }
// }

// ============================================================================
// DOUBLE-ENDED ITERATOR EXTENSIONS
// ============================================================================

/// Extension providing reverse iteration for double-ended iterators.
extend DoubleEndedIterator {

    /// Returns an iterator that yields elements in reverse order.
    ///
    /// Unlike collecting and reversing, this is lazy and O(1) to create.
    ///
    /// Example:
    ///     [1, 2, 3, 4, 5].iter().rev().collect()  // [5, 4, 3, 2, 1]
    ///
    ///     // Take last 3 elements efficiently
    ///     [1, 2, 3, 4, 5].iter().rev().take(count: 3).collect()  // [5, 4, 3]
    ///
    ///     // Find last matching element
    ///     [1, 2, 3, 4, 5].iter().rev().find(|x| x % 2 == 0)  // Some(4)
    public func rev() -> RevIterator[Self] {
        RevIterator(inner: self)
    }
}

// TODO: Cannot access private 'inner' field from extension
// /// Extension making RevIterator also DoubleEndedIterator so rev().rev() works.
// extend RevIterator[I]: DoubleEndedIterator where I: DoubleEndedIterator {
//     public mutating func nextBack() -> I.Item? {
//         self.inner.next()
//     }
// }

/// Extension providing optimized collect for exact-size iterators.
extend ExactSizeIterator {

    /// Returns true if the iterator has no more elements.
    ///
    /// Equivalent to `remaining == 0`.
    public func isEmpty() -> Bool {
        self.remaining == Int64(intLiteral: 0)
    }
}
