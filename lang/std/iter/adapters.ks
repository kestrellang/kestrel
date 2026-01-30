// Iterator adapter types
// These types provide lazy transformation and filtering of sequences.

module std.iter

import std.result.(Optional)
import std.core.(Bool, Cloneable)
import std.num.(Int64)
import std.iter.(Iterator)

// ============================================================================
// TRANSFORMATION ADAPTERS
// ============================================================================

/// Transforms each element using a function.
public struct MapIterator[I, U]: Iterator where I: Iterator {
    type Item = U

    private var inner: I
    private var transform: (I.Item) -> U

    /// Creates a map iterator that applies transform to each element of inner.
    public init(inner: I, transform: (I.Item) -> U) {
        self.inner = inner;
        self.transform = transform;
    }

    /// Returns the next transformed element, or None if exhausted.
    public mutating func next() -> Optional[U] {
        let item = self.inner.next();
        if item.isSome() {
            .Some(self.transform(item.unwrap()))
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

    private var inner: I
    private var predicate: (I.Item) -> Bool

    /// Creates a filter iterator that yields only elements where predicate returns true.
    public init(inner: I, predicate: (I.Item) -> Bool) {
        self.inner = inner;
        self.predicate = predicate;
    }

    /// Returns the next matching element, or None if exhausted.
    public mutating func next() -> Optional[I.Item] {
        var done: Bool = false;
        var result: Optional[I.Item] = .None;
        while done == false {
            let item = self.inner.next();
            if item.isNone() {
                done = true
            } else {
                let value = item.unwrap();
                if self.predicate(value) {
                    result = .Some(value);
                    done = true
                }
            }
        }
        result
    }
}

/// Filters and transforms in one step.
/// Elements where transform returns None are skipped.
public struct FilterMapIterator[I, U]: Iterator where I: Iterator {
    type Item = U

    private var inner: I
    private var transform: (I.Item) -> Optional[U]

    /// Creates an iterator that applies transform and yields only Some results.
    public init(inner: I, transform: (I.Item) -> Optional[U]) {
        self.inner = inner;
        self.transform = transform;
    }

    /// Returns the next transformed element, or None if exhausted.
    public mutating func next() -> Optional[U] {
        var done: Bool = false;
        var result: Optional[U] = .None;
        while done == false {
            let item = self.inner.next();
            if item.isNone() {
                done = true
            } else {
                let transformed = self.transform(item.unwrap());
                if transformed.isSome() {
                    result = transformed;
                    done = true
                }
            }
        }
        result
    }
}

// ============================================================================
// CONDITIONAL ADAPTERS
// ============================================================================

/// Takes elements while predicate is true, then stops.
public struct TakeWhileIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    private var inner: I
    private var predicate: (I.Item) -> Bool
    private var done: Bool

    /// Creates an iterator that yields elements until predicate returns false.
    public init(inner: I, predicate: (I.Item) -> Bool) {
        self.inner = inner;
        self.predicate = predicate;
        self.done = false;
    }

    /// Returns the next element if predicate is still true, or None.
    public mutating func next() -> Optional[I.Item] {
        if self.done {
            return .None
        }

        let item = self.inner.next();
        if item.isNone() {
            self.done = true;
            return .None
        }

        let value = item.unwrap();
        if self.predicate(value) {
            .Some(value)
        } else {
            self.done = true;
            .None
        }
    }
}

/// Skips elements while predicate is true, then yields all remaining.
public struct SkipWhileIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    private var inner: I
    private var predicate: (I.Item) -> Bool
    private var doneSkipping: Bool

    /// Creates an iterator that skips elements until predicate returns false.
    public init(inner: I, predicate: (I.Item) -> Bool) {
        self.inner = inner;
        self.predicate = predicate;
        self.doneSkipping = false;
    }

    /// Returns the next element after skipping is complete, or None.
    public mutating func next() -> Optional[I.Item] {
        if self.doneSkipping {
            return self.inner.next()
        }

        // Skip while predicate is true
        var found: Bool = false;
        var result: Optional[I.Item] = .None;
        while found == false {
            let item = self.inner.next();
            if item.isNone() {
                self.doneSkipping = true;
                found = true
            } else {
                let value = item.unwrap();
                if self.predicate(value) == false {
                    self.doneSkipping = true;
                    result = .Some(value);
                    found = true
                }
            }
        }
        result
    }
}

// ============================================================================
// COMBINING ADAPTERS
// ============================================================================

/// Pairs elements from two iterators.
/// Stops when either iterator is exhausted.
public struct ZipIterator[A, B]: Iterator where A: Iterator, B: Iterator {
    type Item = (A.Item, B.Item)

    private var first: A
    private var second: B

    /// Creates an iterator that pairs elements from first and second.
    public init(first: A, second: B) {
        self.first = first;
        self.second = second;
    }

    /// Returns the next pair, or None if either iterator is exhausted.
    public mutating func next() -> Optional[(A.Item, B.Item)] {
        let a = self.first.next();
        if a.isNone() {
            return .None
        }
        let b = self.second.next();
        if b.isNone() {
            return .None
        }
        let pair = (a.unwrap(), b.unwrap());
        .Some(pair)
    }
}

/// Yields (index, item) pairs.
public struct EnumerateIterator[I]: Iterator where I: Iterator {
    type Item = (Int64, I.Item)

    private var inner: I
    private var index: Int64

    /// Creates an iterator that pairs each element with its zero-based index.
    public init(inner: I) {
        self.inner = inner;
        self.index = Int64(intLiteral: 0);
    }

    /// Returns the next (index, element) pair, or None if exhausted.
    public mutating func next() -> Optional[(Int64, I.Item)] {
        let item = self.inner.next();
        if item.isSome() {
            let i = self.index;
            self.index = self.index + Int64(intLiteral: 1);
            .Some((i, item.unwrap()))
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

    private var inner: I
    private var remaining: Int64

    /// Creates an iterator that yields at most count elements.
    public init(inner: I, count: Int64) {
        self.inner = inner;
        self.remaining = count;
    }

    /// Returns the next element if count not reached, or None.
    public mutating func next() -> Optional[I.Item] {
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

    private var inner: I
    private var remaining: Int64

    /// Creates an iterator that skips the first count elements.
    public init(inner: I, count: Int64) {
        self.inner = inner;
        self.remaining = count;
    }

    /// Returns the next element after skipping, or None if exhausted.
    public mutating func next() -> Optional[I.Item] {
        // Skip remaining elements first
        while self.remaining > Int64(intLiteral: 0) {
            let item = self.inner.next();
            if item.isNone() {
                return .None
            }
            self.remaining = self.remaining - Int64(intLiteral: 1)
        }
        self.inner.next()
    }
}

/// Chains two iterators together.
/// First yields all elements from first, then all from second.
public struct ChainIterator[A, B]: Iterator where A: Iterator, B: Iterator, B.Item = A.Item {
    type Item = A.Item

    private var first: A
    private var second: B
    private var firstDone: Bool

    /// Creates an iterator that chains first and second together.
    public init(first: A, second: B) {
        self.first = first;
        self.second = second;
        self.firstDone = false;
    }

    /// Returns the next element from first, or from second if first is exhausted.
    public mutating func next() -> Optional[A.Item] {
        if not self.firstDone {
            let item = self.first.next();
            if item.isSome() {
                return item
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

    private var inner: I
    private var peeked: Optional[Optional[I.Item]]

    /// Creates a peekable iterator wrapping inner.
    public init(inner: I) {
        self.inner = inner;
        self.peeked = .None;
    }

    /// Returns the next element without consuming it.
    public mutating func peek() -> Optional[I.Item] {
        if self.peeked.isNone() {
            self.peeked = .Some(self.inner.next())
        }
        self.peeked.unwrap()
    }

    /// Returns and consumes the next element.
    public mutating func next() -> Optional[I.Item] {
        if self.peeked.isSome() {
            let result = self.peeked.unwrap();
            self.peeked = .None;
            return result
        }
        self.inner.next()
    }
}

/// Repeats an iterator forever by cloning it when exhausted.
public struct CycleIterator[I]: Iterator where I: Iterator, I: Cloneable {
    type Item = I.Item

    private var original: I
    private var current: I

    /// Creates an iterator that repeats iter infinitely.
    public init(iter: I) {
        self.original = iter.clone();
        self.current = iter;
    }

    /// Returns the next element, restarting from the beginning if needed.
    public mutating func next() -> Optional[I.Item] {
        let item = self.current.next();
        if item.isSome() {
            return item
        }
        self.current = self.original.clone();
        self.current.next()
    }
}

/// Stops permanently after yielding None once.
/// Useful for iterators that might resume after None.
public struct FuseIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    private var inner: I
    private var done: Bool

    /// Creates a fused iterator that stops permanently after the first None.
    public init(inner: I) {
        self.inner = inner;
        self.done = false;
    }

    /// Returns the next element, or None permanently after first exhaustion.
    public mutating func next() -> Optional[I.Item] {
        if self.done {
            return .None
        }

        let item = self.inner.next();
        if item.isNone() {
            self.done = true
        }
        item
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
    public mutating func next() -> Optional[T] {
        .None
    }
}

/// An iterator that yields a single value.
public struct OnceIterator[T]: Iterator {
    type Item = T

    private var value: Optional[T]

    /// Creates an iterator that yields value exactly once.
    public init(value: T) {
        self.value = .Some(value);
    }

    /// Returns the value on first call, None thereafter.
    public mutating func next() -> Optional[T] {
        let result = self.value;
        self.value = .None;
        result
    }
}

/// An iterator that yields the same value forever.
public struct RepeatIterator[T]: Iterator where T: Cloneable {
    type Item = T

    private var value: T

    /// Creates an iterator that yields value forever.
    public init(value: T) {
        self.value = value;
    }

    /// Returns a clone of the value.
    public mutating func next() -> Optional[T] {
        .Some(self.value.clone())
    }
}

/// An iterator that yields the same value n times.
public struct RepeatNIterator[T]: Iterator where T: Cloneable {
    type Item = T

    private var value: T
    private var remaining: Int64

    /// Creates an iterator that yields value exactly count times.
    public init(value value: T, count count: Int64) {
        self.value = value;
        self.remaining = count;
    }

    /// Returns a clone of the value, or None after count iterations.
    public mutating func next() -> Optional[T] {
        if self.remaining > Int64(intLiteral: 0) {
            self.remaining = self.remaining - Int64(intLiteral: 1);
            .Some(self.value.clone())
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
    OnceIterator(value)
}

/// Creates an iterator that yields value forever.
public func repeatValue[T](value: T) -> RepeatIterator[T] where T: Cloneable {
    RepeatIterator(value)
}

/// Creates an iterator that yields value exactly count times.
public func repeatN[T](value: T, count: Int64) -> RepeatNIterator[T] where T: Cloneable {
    RepeatNIterator(value: value, count: count)
}
