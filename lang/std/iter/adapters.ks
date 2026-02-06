// Iterator adapter types
// These types provide lazy transformation and filtering of sequences.

module std.iter

import std.result.(Optional)
import std.core.(Bool, Copyable, Cloneable)
import std.num.(Int64)

// ============================================================================
// TRANSFORMATION ADAPTERS
// ============================================================================

/// Transforms each element using a function.
public struct MapIterator[I, U]: Iterator where I: Iterator {
    type Item = U

    internal var inner: I
    internal var transform: (I.Item) -> U

    /// Creates a map iterator that applies transform to each element of inner.
    public init(inner inner: I, transform transform: (I.Item) -> U) {
        self.inner = inner;
        self.transform = transform;
    }

    /// Returns the next transformed element, or None if exhausted.
    public mutating func next() -> U? {
        if let .Some(item) = self.inner.next() {
            .Some(self.transform(item))
        } else {
            .None
        }
    }
}

// ============================================================================
// FILTERING ADAPTERS
// ============================================================================

/// Yields only elements matching a predicate.
public struct FilterIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var predicate: (I.Item) -> Bool

    /// Creates a filter iterator that yields only elements where predicate returns true.
    public init(inner inner: I, predicate predicate: (I.Item) -> Bool) {
        self.inner = inner;
        self.predicate = predicate;
    }

    /// Returns the next matching element, or None if exhausted.
    public mutating func next() -> I.Item? {
        while let .Some(value) = self.inner.next() {
            if self.predicate(value) {
                return .Some(value)
            }
        }
        .None
    }
}

/// Filters and transforms in one step.
/// Elements where transform returns None are skipped.
public struct FilterMapIterator[I, U]: Iterator where I: Iterator {
    type Item = U

    internal var inner: I
    internal var transform: (I.Item) -> U?

    /// Creates an iterator that applies transform and yields only Some results.
    public init(inner inner: I, transform transform: (I.Item) -> U?) {
        self.inner = inner;
        self.transform = transform;
    }

    /// Returns the next transformed element, or None if exhausted.
    public mutating func next() -> U? {
        while let .Some(item) = self.inner.next() {
            let transformed = self.transform(item);
            if let .Some(value) = transformed {
                return .Some(value)
            }
        }
        .None
    }
}

// ============================================================================
// CONDITIONAL ADAPTERS
// ============================================================================

/// Takes elements while predicate is true, then stops.
public struct TakeWhileIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var predicate: (I.Item) -> Bool
    internal var done: Bool

    /// Creates an iterator that yields elements until predicate returns false.
    public init(inner inner: I, predicate predicate: (I.Item) -> Bool) {
        self.inner = inner;
        self.predicate = predicate;
        self.done = false;
    }

    /// Returns the next element if predicate is still true, or None.
    public mutating func next() -> I.Item? {
        if self.done {
            return .None
        }

        if let .Some(value) = self.inner.next() {
            if self.predicate(value) {
                .Some(value)
            } else {
                self.done = true;
                .None
            }
        } else {
            self.done = true;
            .None
        }
    }
}

/// Skips elements while predicate is true, then yields all remaining.
public struct SkipWhileIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var predicate: (I.Item) -> Bool
    internal var doneSkipping: Bool

    /// Creates an iterator that skips elements until predicate returns false.
    public init(inner inner: I, predicate predicate: (I.Item) -> Bool) {
        self.inner = inner;
        self.predicate = predicate;
        self.doneSkipping = false;
    }

    /// Returns the next element after skipping is complete, or None.
    public mutating func next() -> I.Item? {
        if self.doneSkipping {
            return self.inner.next()
        }

        // Skip while predicate is true
        while let .Some(value) = self.inner.next() {
            if self.predicate(value) == false {
                self.doneSkipping = true;
                return .Some(value)
            }
        }
        self.doneSkipping = true;
        .None
    }
}

// ============================================================================
// COMBINING ADAPTERS
// ============================================================================

/// Pairs elements from two iterators.
/// Stops when either iterator is exhausted.
public struct ZipIterator[A, B]: Iterator where A: Iterator, B: Iterator {
    type Item = (A.Item, B.Item)

    internal var first: A
    internal var second: B

    /// Creates an iterator that pairs elements from first and second.
    public init(first first: A, second second: B) {
        self.first = first;
        self.second = second;
    }

    /// Returns the next pair, or None if either iterator is exhausted.
    public mutating func next() -> (A.Item, B.Item)? {
        if let .Some(a) = self.first.next() {
            if let .Some(b) = self.second.next() {
                .Some((a, b))
            } else {
                .None
            }
        } else {
            .None
        }
    }
}

/// Yields (index, item) pairs.
public struct EnumerateIterator[I]: Iterator where I: Iterator {
    type Item = (Int64, I.Item)

    internal var inner: I
    internal var index: Int64

    /// Creates an iterator that pairs each element with its zero-based index.
    public init(inner inner: I) {
        self.inner = inner;
        self.index = Int64(intLiteral: 0);
    }

    /// Returns the next (index, element) pair, or None if exhausted.
    public mutating func next() -> (Int64, I.Item)? {
        if let .Some(item) = self.inner.next() {
            let i = self.index;
            self.index = self.index + Int64(intLiteral: 1);
            .Some((i, item))
        } else {
            .None
        }
    }
}

// ============================================================================
// SLICING ADAPTERS
// ============================================================================

/// Takes only the first n elements.
public struct TakeIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var remaining: Int64

    /// Creates an iterator that yields at most count elements.
    public init(inner inner: I, count count: Int64) {
        self.inner = inner;
        self.remaining = count;
    }

    /// Returns the next element if count not reached, or None.
    public mutating func next() -> I.Item? {
        if self.remaining > Int64(intLiteral: 0) {
            self.remaining = self.remaining - Int64(intLiteral: 1);
            self.inner.next()
        } else {
            .None
        }
    }
}

/// Skips the first n elements.
public struct SkipIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var remaining: Int64

    /// Creates an iterator that skips the first count elements.
    public init(inner inner: I, count count: Int64) {
        self.inner = inner;
        self.remaining = count;
    }

    /// Returns the next element after skipping, or None if exhausted.
    public mutating func next() -> I.Item? {
        // Skip remaining elements first
        while self.remaining > Int64(intLiteral: 0) {
            if let .Some(_) = self.inner.next() {
                self.remaining = self.remaining - Int64(intLiteral: 1)
            } else {
                return .None
            }
        }
        self.inner.next()
    }
}

/// Chains two iterators together.
/// First yields all elements from first, then all from second.
public struct ChainIterator[A, B]: Iterator where A: Iterator, B: Iterator, B.Item = A.Item {
    type Item = A.Item

    internal var first: A
    internal var second: B
    internal var firstDone: Bool

    /// Creates an iterator that chains first and second together.
    public init(first first: A, second second: B) {
        self.first = first;
        self.second = second;
        self.firstDone = false;
    }

    /// Returns the next element from first, or from second if first is exhausted.
    public mutating func next() -> A.Item? {
        if not self.firstDone {
            if let .Some(item) = self.first.next() {
                return .Some(item)
            }
            self.firstDone = true
        }
        self.second.next()
    }
}

// ============================================================================
// UTILITY ADAPTERS
// ============================================================================

/// Allows peeking at the next element without consuming it.
public struct PeekableIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var peeked: Optional[I.Item]?

    /// Creates a peekable iterator wrapping inner.
    public init(inner inner: I) {
        self.inner = inner;
        self.peeked = .None;
    }

    /// Returns the next element without consuming it.
    public mutating func peek() -> I.Item? {
        if let .None = self.peeked {
            self.peeked = .Some(self.inner.next())
        }
        if let .Some(value) = self.peeked {
            value
        } else {
            .None
        }
    }

    /// Returns and consumes the next element.
    public mutating func next() -> I.Item? {
        if let .Some(peeked) = self.peeked {
            if let .Some(value) = peeked {
                self.peeked = .None;
                return .Some(value)
            }
            self.peeked = .None;
            return .None
        }
        self.inner.next()
    }
}

/// Repeats an iterator forever by cloning it when exhausted.
public struct CycleIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var original: I
    internal var current: I

    /// Creates an iterator that repeats iter infinitely.
    public init(iter iter: I) {
        self.original = iter;
        self.current = iter;
    }

    /// Returns the next element, restarting from the beginning if needed.
    public mutating func next() -> I.Item? {
        if let .Some(item) = self.current.next() {
            return .Some(item)
        }
        self.current = self.original;
        self.current.next()
    }
}

/// Stops permanently after yielding None once.
/// Useful for iterators that might resume after None.
public struct FuseIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var done: Bool

    /// Creates a fused iterator that stops permanently after the first None.
    public init(inner inner: I) {
        self.inner = inner;
        self.done = false;
    }

    /// Returns the next element, or None permanently after first exhaustion.
    public mutating func next() -> I.Item? {
        if self.done {
            return .None
        }

        if let .Some(item) = self.inner.next() {
            .Some(item)
        } else {
            self.done = true;
            .None
        }
    }
}

// ============================================================================
// SOURCE ITERATORS
// ============================================================================

/// An iterator that yields nothing.
public struct EmptyIterator[T]: Iterator {
    type Item = T

    /// Creates an empty iterator.
    public init() {}

    /// Always returns None.
    public mutating func next() -> T? {
        .None
    }
}

/// An iterator that yields a single value.
public struct OnceIterator[T]: Iterator {
    type Item = T

    internal var value: T?

    /// Creates an iterator that yields value exactly once.
    public init(value value: T) {
        self.value = .Some(value);
    }

    /// Returns the value on first call, None thereafter.
    public mutating func next() -> T? {
        let result = self.value;
        self.value = .None;
        result
    }
}

/// An iterator that yields the same value forever.
public struct RepeatIterator[T]: Iterator {
    type Item = T

    internal var value: T

    /// Creates an iterator that yields value forever.
    public init(value value: T) {
        self.value = value;
    }

    /// Returns a copy of the value.
    public mutating func next() -> T? {
        .Some(self.value)
    }
}

/// An iterator that yields the same value n times.
public struct RepeatNIterator[T]: Iterator {
    type Item = T

    internal var value: T
    internal var remaining: Int64

    /// Creates an iterator that yields value exactly count times.
    public init(value value: T, count count: Int64) {
        self.value = value;
        self.remaining = count;
    }

    /// Returns a copy of the value, or None after count iterations.
    public mutating func next() -> T? {
        if self.remaining > Int64(intLiteral: 0) {
            self.remaining = self.remaining - Int64(intLiteral: 1);
            .Some(self.value)
        } else {
            .None
        }
    }
}

// ============================================================================
// FLATMAPPING ADAPTERS
// ============================================================================

/// Transforms each element into an iterator and flattens the results.
///
/// Each element is transformed into an iterator, and all resulting
/// elements are yielded sequentially.
public struct FlatMapIterator[I, U]: Iterator where I: Iterator, U: Iterator {
    type Item = U.Item

    internal var inner: I
    internal var transform: (I.Item) -> U
    internal var current: U?

    /// Creates an iterator that applies transform to each element and flattens.
    public init(inner inner: I, transform transform: (I.Item) -> U) {
        self.inner = inner;
        self.transform = transform;
        self.current = .None;
    }

    /// Returns the next element from the flattened sequence, or None if exhausted.
    public mutating func next() -> U.Item? {
        while true {
            if let .Some(existing) = self.current {
                var currentIter = existing;
                if let .Some(item) = currentIter.next() {
                    self.current = .Some(currentIter);
                    return .Some(item)
                }
                self.current = .None;
            }

            if let .Some(item) = self.inner.next() {
                self.current = .Some(self.transform(item));
            } else {
                return .None
            }
        }
        // Unreachable - loop always returns
        .None
    }
}

// ============================================================================
// FLATTENING ADAPTERS
// ============================================================================

/// Flattens nested iterators into a single iterator.
///
/// Takes an iterator of iterators and yields all elements from each
/// inner iterator sequentially.
public struct FlattenIterator[I]: Iterator where I: Iterator, I.Item: Iterator {
    type Item = I.Item.Item

    internal var inner: I
    internal var current: I.Item?

    /// Creates an iterator that flattens nested iterators.
    public init(inner inner: I) {
        self.inner = inner;
        self.current = .None;
    }

    /// Returns the next element from the flattened sequence.
    public mutating func next() -> I.Item.Item? {
        while true {
            if let .Some(existing) = self.current {
                var currentIter = existing;
                if let .Some(item) = currentIter.next() {
                    self.current = .Some(currentIter);
                    return .Some(item)
                }
                self.current = .None;
            }

            if let .Some(nextIter) = self.inner.next() {
                self.current = .Some(nextIter);
            } else {
                return .None
            }
        }
        // Unreachable - loop always returns
        .None
    }
}

// ============================================================================
// INSPECTING ADAPTERS
// ============================================================================

/// Calls a function on each element as it passes through.
///
/// Useful for debugging or logging without affecting the iterator chain.
/// The inspector function receives a reference to each element.
public struct InspectIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var inspector: (I.Item) -> ()

    /// Creates an iterator that calls inspector on each element.
    public init(inner inner: I, inspector inspector: (I.Item) -> ()) {
        self.inner = inner;
        self.inspector = inspector;
    }

    /// Returns the next element after calling the inspector on it.
    public mutating func next() -> I.Item? {
        if let .Some(item) = self.inner.next() {
            self.inspector(item);
            .Some(item)
        } else {
            .None
        }
    }
}

// ============================================================================
// STEPPING ADAPTERS
// ============================================================================

/// Yields every nth element, starting with the first.
public struct StepByIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var step: Int64
    internal var first: Bool

    /// Creates an iterator that yields every `step` elements.
    /// Panics if step is 0.
    public init(inner inner: I, step step: Int64) {
        self.inner = inner;
        self.step = step;
        self.first = true;
    }

    /// Returns every nth element.
    public mutating func next() -> I.Item? {
        if self.first {
            self.first = false;
            return self.inner.next()
        }

        var i = Int64(intLiteral: 0);
        while i < self.step - Int64(intLiteral: 1) {
            let _ = self.inner.next();
            i = i + Int64(intLiteral: 1);
        }
        self.inner.next()
    }
}

// ============================================================================
// REVERSAL ADAPTERS
// ============================================================================

/// Reverses a double-ended iterator.
///
/// Yields elements by calling `nextBack()` on the inner iterator.
public struct RevIterator[I] where I: DoubleEndedIterator {
    type Item = I.Item

    internal var inner: I

    /// Creates an iterator that yields elements in reverse order.
    public init(inner inner: I) {
        self.inner = inner;
    }
}



// ============================================================================
// SCANNING ADAPTERS
// ============================================================================

/// Yields running accumulator values during a fold.
///
/// Like fold, but yields each intermediate accumulator value.
public struct ScanIterator[I, Acc]: Iterator where I: Iterator {
    type Item = Acc

    internal var inner: I
    internal var state: Acc
    internal var combine: (Acc, I.Item) -> Acc

    /// Creates an iterator that yields running fold values.
    public init(inner inner: I, initial initial: Acc, combine combine: (Acc, I.Item) -> Acc) {
        self.inner = inner;
        self.state = initial;
        self.combine = combine;
    }

    /// Returns the next accumulated value.
    public mutating func next() -> Acc? {
        if let .Some(item) = self.inner.next() {
            self.state = self.combine(self.state, item);
            .Some(self.state)
        } else {
            .None
        }
    }
}

// ============================================================================
// INTERSPERSING ADAPTERS
// ============================================================================

/// Inserts a separator between each element.
public struct IntersperseIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var separator: I.Item
    internal var needsSeparator: Bool
    internal var pendingItem: I.Item?

    /// Creates an iterator that inserts separator between elements.
    public init(inner inner: I, separator separator: I.Item) {
        self.inner = inner;
        self.separator = separator;
        self.needsSeparator = false;
        self.pendingItem = .None;
    }

    /// Returns the next element or separator.
    public mutating func next() -> I.Item? {
        if let .Some(item) = self.pendingItem {
            self.pendingItem = .None;
            return .Some(item)
        }

        if let .Some(item) = self.inner.next() {
            if self.needsSeparator {
                self.pendingItem = .Some(item);
                .Some(self.separator)
            } else {
                self.needsSeparator = true;
                .Some(item)
            }
        } else {
            .None
        }
    }
}

/// Inserts a lazily-generated separator between each element.
public struct IntersperseWithIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var separator: () -> I.Item
    internal var needsSeparator: Bool
    internal var pendingItem: I.Item?

    /// Creates an iterator that inserts separators generated by the function.
    public init(inner inner: I, separator separator: () -> I.Item) {
        self.inner = inner;
        self.separator = separator;
        self.needsSeparator = false;
        self.pendingItem = .None;
    }

    /// Returns the next element or generated separator.
    public mutating func next() -> I.Item? {
        if let .Some(item) = self.pendingItem {
            self.pendingItem = .None;
            return .Some(item)
        }

        if let .Some(item) = self.inner.next() {
            if self.needsSeparator {
                self.pendingItem = .Some(item);
                .Some(self.separator())
            } else {
                self.needsSeparator = true;
                .Some(item)
            }
        } else {
            .None
        }
    }
}

// ============================================================================
// CONVENIENCE FUNCTIONS
// ============================================================================

/// Creates an iterator that yields nothing.
public func empty[T]() -> EmptyIterator[T] {
    EmptyIterator()
}

/// Creates an iterator that yields value exactly once.
public func once[T](value: T) -> OnceIterator[T] {
    OnceIterator(value: value)
}

/// Creates an iterator that yields value forever.
public func repeatValue[T](value: T) -> RepeatIterator[T] {
    RepeatIterator(value: value)
}

/// Creates an iterator that yields value exactly count times.
public func repeatN[T](value: T, count: Int64) -> RepeatNIterator[T] {
    RepeatNIterator(value: value, count: count)
}
