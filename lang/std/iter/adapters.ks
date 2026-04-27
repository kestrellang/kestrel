// Iterator adapter types
// These structs are the concrete iterators returned by the lazy
// adapter combinators on `Iterator`. End users normally reach for the
// builder methods (`it.map(...)`, `it.filter(...)`, …) rather than
// constructing these directly — the public surface lives in
// `iterator.ks`.

module std.iter

import std.result.(Optional)
import std.core.(Bool, Copyable, Cloneable)
import std.num.(Int64)

// ============================================================================
// TRANSFORMATION ADAPTERS
// ============================================================================

/// Lazy `map` — applies a transform to each element of `inner` as values
/// are pulled. Returned by `Iterator.map(_:)`.
///
/// # Representation
///
/// Wraps the source iterator and the transform closure. No buffering —
/// elements pass through one at a time.
public struct MapIterator[I, U]: Iterator where I: Iterator {
    type Item = U

    internal var inner: I
    internal var transform: (I.Item) -> U

    /// @name From Source
    /// Builds a `MapIterator` from `inner` and `transform`. Prefer
    /// `inner.map(transform)`.
    public init(inner inner: I, transform transform: (I.Item) -> U) {
        self.inner = inner;
        self.transform = transform;
    }

    /// Pulls the next element from `inner` and runs `transform` on it.
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

/// Lazy `filter` — yields only elements where the predicate returns
/// `true`. Returned by `Iterator.filter(_:)`.
///
/// # Representation
///
/// Source iterator + predicate closure. `next()` skips ahead until the
/// predicate accepts.
public struct FilterIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var predicate: (I.Item) -> Bool

    /// @name From Source
    /// Builds a `FilterIterator`. Prefer `inner.filter(predicate)`.
    public init(inner inner: I, predicate predicate: (I.Item) -> Bool) {
        self.inner = inner;
        self.predicate = predicate;
    }

    /// Pulls until an element satisfies `predicate`, returning it. `None`
    /// when the source is exhausted with no further match.
    public mutating func next() -> I.Item? {
        while let .Some(value) = self.inner.next() {
            if self.predicate(value) {
                return .Some(value)
            }
        }
        .None
    }
}

/// Lazy `filterMap` / `compactMap` — runs a transform that returns
/// `Optional[U]` and drops `None`s. Returned by both
/// `Iterator.filterMap(_:)` and `Iterator.compactMap()`.
///
/// # Representation
///
/// Source iterator + transform closure. `next()` skips ahead until the
/// transform yields `Some`.
public struct FilterMapIterator[I, U]: Iterator where I: Iterator {
    type Item = U

    internal var inner: I
    internal var transform: (I.Item) -> U?

    /// @name From Source
    /// Builds a `FilterMapIterator`. Prefer `inner.filterMap(...)` /
    /// `inner.compactMap()`.
    public init(inner inner: I, transform transform: (I.Item) -> U?) {
        self.inner = inner;
        self.transform = transform;
    }

    /// Pulls until `transform` returns `Some`, then yields it.
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

/// Lazy `takeWhile` — yields elements until the predicate first returns
/// `false`, then permanently stops. Returned by `Iterator.takeWhile(_:)`.
///
/// # Representation
///
/// Source iterator + predicate + a one-bit `done` flag that latches once
/// the predicate fails.
public struct TakeWhileIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var predicate: (I.Item) -> Bool
    internal var done: Bool

    /// @name From Source
    /// Builds a `TakeWhileIterator`. Prefer `inner.takeWhile(predicate)`.
    public init(inner inner: I, predicate predicate: (I.Item) -> Bool) {
        self.inner = inner;
        self.predicate = predicate;
        self.done = false;
    }

    /// Returns the next element if `predicate` still accepts; latches
    /// `done = true` and returns `None` on the first rejection or
    /// underlying exhaustion.
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

/// Lazy `skipWhile` — drops a leading run of elements satisfying the
/// predicate, then yields *every* remaining element. Returned by
/// `Iterator.skipWhile(_:)`.
///
/// # Representation
///
/// Source iterator + predicate + a one-bit `doneSkipping` flag that
/// latches once the skipping phase ends.
public struct SkipWhileIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var predicate: (I.Item) -> Bool
    internal var doneSkipping: Bool

    /// @name From Source
    /// Builds a `SkipWhileIterator`. Prefer `inner.skipWhile(predicate)`.
    public init(inner inner: I, predicate predicate: (I.Item) -> Bool) {
        self.inner = inner;
        self.predicate = predicate;
        self.doneSkipping = false;
    }

    /// On first call, drains source elements that match `predicate` and
    /// returns the first one that doesn't. After that, forwards `next()`
    /// directly.
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

/// Lazy `zip` — pairs elements from two iterators. Stops at the shorter
/// one. Returned by `Iterator.zip(other:)`.
///
/// # Representation
///
/// Holds both source iterators. No buffering.
public struct ZipIterator[A, B]: Iterator where A: Iterator, B: Iterator {
    type Item = (A.Item, B.Item)

    internal var first: A
    internal var second: B

    /// @name From Sources
    /// Builds a `ZipIterator`. Prefer `first.zip(other: second)`.
    public init(first first: A, second second: B) {
        self.first = first;
        self.second = second;
    }

    /// Pulls one element from each side and pairs them. `None` if either
    /// runs out.
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

/// Lazy `enumerate` — pairs each element with its zero-based position.
/// Returned by `Iterator.enumerate()`.
///
/// # Representation
///
/// Source iterator + a running `Int64` index that ticks per element.
public struct EnumerateIterator[I]: Iterator where I: Iterator {
    type Item = (Int64, I.Item)

    internal var inner: I
    internal var index: Int64

    /// @name From Source
    /// Builds an `EnumerateIterator` with the index starting at 0.
    /// Prefer `inner.enumerate()`.
    public init(inner inner: I) {
        self.inner = inner;
        self.index = 0;
    }

    /// Pulls the next element and pairs it with the current index, then
    /// increments the index.
    public mutating func next() -> (Int64, I.Item)? {
        if let .Some(item) = self.inner.next() {
            let i = self.index;
            self.index = self.index + 1;
            .Some((i, item))
        } else {
            .None
        }
    }
}

// ============================================================================
// SLICING ADAPTERS
// ============================================================================

/// Lazy `take` — yields at most `count` elements from the source.
/// Returned by `Iterator.take(count:)`.
///
/// # Representation
///
/// Source iterator + a counter that ticks down to zero.
public struct TakeIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var remaining: Int64

    /// @name From Source
    /// Builds a `TakeIterator` with `count` capacity.
    public init(inner inner: I, count count: Int64) {
        self.inner = inner;
        self.remaining = count;
    }

    /// Decrements `remaining` and forwards `next()`; returns `None` once
    /// the budget hits zero.
    public mutating func next() -> I.Item? {
        if self.remaining > 0 {
            self.remaining = self.remaining - 1;
            self.inner.next()
        } else {
            .None
        }
    }
}

/// Lazy `skip` — drops the first `count` elements, then yields the rest.
/// Returned by `Iterator.skip(count:)`.
///
/// # Representation
///
/// Source iterator + a counter; the first `next()` call drains the
/// budget by pulling the source.
public struct SkipIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var remaining: Int64

    /// @name From Source
    /// Builds a `SkipIterator` that will drop `count` elements before
    /// yielding.
    public init(inner inner: I, count count: Int64) {
        self.inner = inner;
        self.remaining = count;
    }

    /// On first call, walks past `remaining` source elements; subsequent
    /// calls forward `next()` directly.
    public mutating func next() -> I.Item? {
        // Skip remaining elements first
        while self.remaining > 0 {
            if let .Some(_) = self.inner.next() {
                self.remaining = self.remaining - 1
            } else {
                return .None
            }
        }
        self.inner.next()
    }
}

/// Lazy `chain` — yields all of `first`, then all of `second`. Returned
/// by `Iterator.chain(other:)`.
///
/// # Representation
///
/// Both source iterators + a one-bit `firstDone` flag that latches when
/// the first iterator runs out.
public struct ChainIterator[A, B]: Iterator where A: Iterator, B: Iterator, B.Item = A.Item {
    type Item = A.Item

    internal var first: A
    internal var second: B
    internal var firstDone: Bool

    /// @name From Sources
    /// Builds a `ChainIterator`. Prefer `first.chain(other: second)`.
    public init(first first: A, second second: B) {
        self.first = first;
        self.second = second;
        self.firstDone = false;
    }

    /// Pulls from `first` until it's empty, then forwards to `second`.
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

/// Iterator wrapper that lets you peek at the next element without
/// consuming it. Returned by `Iterator.peekable()`.
///
/// # Representation
///
/// Source iterator + a one-slot lookahead buffer (`peeked`). `peek()`
/// fills the buffer; `next()` drains it before pulling the source.
public struct PeekableIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var peeked: Optional[I.Item]?

    /// @name From Source
    /// Builds a `PeekableIterator` with no value buffered.
    public init(inner inner: I) {
        self.inner = inner;
        self.peeked = .None;
    }

    /// Returns the next element without consuming it. Subsequent
    /// `peek()` calls keep returning the same value until `next()` is
    /// called.
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

    /// Returns the buffered value if present, otherwise pulls the source.
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

/// Repeats a finite iterator forever by copying it on each lap. Returned
/// by `Iterator.cycle()`.
///
/// # Representation
///
/// Two copies of the source: `original` (immutable template) and
/// `current` (the working iterator). When `current` exhausts, it is
/// reset from `original`.
public struct CycleIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var original: I
    internal var current: I

    /// @name From Source
    /// Builds a `CycleIterator` that will replay `iter` forever.
    public init(iter iter: I) {
        self.original = iter;
        self.current = iter;
    }

    /// Pulls the current lap; on exhaustion, restarts and pulls again.
    public mutating func next() -> I.Item? {
        if let .Some(item) = self.current.next() {
            return .Some(item)
        }
        self.current = self.original;
        self.current.next()
    }
}

/// Wraps a source so that once `None` is returned, future calls also
/// return `None`. Returned by `Iterator.fuse()`.
///
/// # Representation
///
/// Source iterator + a one-bit latch.
public struct FuseIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var done: Bool

    /// @name From Source
    /// Builds a `FuseIterator` in the "still active" state.
    public init(inner inner: I) {
        self.inner = inner;
        self.done = false;
    }

    /// Forwards `next()`; latches `done = true` on the first `None` and
    /// returns `None` forever afterwards.
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

/// Iterator that yields no elements. Returned by `empty()`.
///
/// # Representation
///
/// Zero-sized — no fields.
public struct EmptyIterator[T]: Iterator {
    type Item = T

    /// @name Default
    /// Builds an `EmptyIterator`. Prefer the free `empty()` function.
    public init() {}

    /// Always `None`.
    public mutating func next() -> T? {
        .None
    }
}

/// Iterator that yields a single value, then nothing. Returned by
/// `once(value:)`.
///
/// # Representation
///
/// One `Optional[T]` field. `next()` empties it on first call.
public struct OnceIterator[T]: Iterator {
    type Item = T

    internal var value: T?

    /// @name From Value
    /// Builds a `OnceIterator` carrying `value`.
    public init(value value: T) {
        self.value = .Some(value);
    }

    /// Returns the value once, then `None` forever after.
    public mutating func next() -> T? {
        let result = self.value;
        self.value = .None;
        result
    }
}

/// Iterator that yields the same value indefinitely. Returned by
/// `repeatValue(value:)`.
///
/// # Representation
///
/// One `T` field that is copied on every `next()` call.
public struct RepeatIterator[T]: Iterator {
    type Item = T

    internal var value: T

    /// @name From Value
    /// Builds a `RepeatIterator` over `value`.
    public init(value value: T) {
        self.value = value;
    }

    /// Returns a fresh copy of the stored value every call.
    public mutating func next() -> T? {
        .Some(self.value)
    }
}

/// Iterator that yields the same value `count` times, then stops.
/// Returned by `repeatN(value:count:)`.
///
/// # Representation
///
/// `T` payload + an `Int64` countdown.
public struct RepeatNIterator[T]: Iterator {
    type Item = T

    internal var value: T
    internal var remaining: Int64

    /// @name From Value
    /// Builds a `RepeatNIterator` that will yield `value` exactly
    /// `count` times.
    public init(value value: T, count count: Int64) {
        self.value = value;
        self.remaining = count;
    }

    /// Decrements `remaining` and returns a fresh copy of the value;
    /// returns `None` once the counter hits zero.
    public mutating func next() -> T? {
        if self.remaining > 0 {
            self.remaining = self.remaining - 1;
            .Some(self.value)
        } else {
            .None
        }
    }
}

// ============================================================================
// FLATMAPPING ADAPTERS
// ============================================================================

/// Lazy `flatMap` — turns each element of the source into an iterator
/// and concatenates the results. Returned by `Iterator.flatMap(_:)`.
///
/// # Representation
///
/// Source iterator + transform closure + a one-slot buffer (`current`)
/// holding the inner iterator currently being drained.
public struct FlatMapIterator[I, U]: Iterator where I: Iterator, U: Iterator {
    type Item = U.Item

    internal var inner: I
    internal var transform: (I.Item) -> U
    internal var current: U?

    /// @name From Source
    /// Builds a `FlatMapIterator` with no inner iterator buffered.
    public init(inner inner: I, transform transform: (I.Item) -> U) {
        self.inner = inner;
        self.transform = transform;
        self.current = .None;
    }

    /// Drains the buffered inner iterator; when it runs out, pulls the
    /// next source element, transforms it into a fresh inner iterator,
    /// and continues.
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

/// Lazy `flatten` — concatenates the inner iterators of an
/// iterator-of-iterators. Returned by `Iterator.flatten()`.
///
/// # Representation
///
/// Source iterator + a one-slot buffer holding the inner iterator
/// currently being drained.
public struct FlattenIterator[I]: Iterator where I: Iterator, I.Item: Iterator {
    type Item = I.Item.Item

    internal var inner: I
    internal var current: I.Item?

    /// @name From Source
    /// Builds a `FlattenIterator` with no inner iterator buffered.
    public init(inner inner: I) {
        self.inner = inner;
        self.current = .None;
    }

    /// Drains the buffered inner iterator, then pulls the next inner
    /// iterator from the source.
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

/// Side-effecting passthrough. Calls `inspector` on each element and
/// then yields it unchanged. Returned by `Iterator.inspect(_:)`.
///
/// # Representation
///
/// Source iterator + inspector closure. No buffering.
public struct InspectIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var inspector: (I.Item) -> ()

    /// @name From Source
    /// Builds an `InspectIterator`. Prefer `inner.inspect(inspector)`.
    public init(inner inner: I, inspector inspector: (I.Item) -> ()) {
        self.inner = inner;
        self.inspector = inspector;
    }

    /// Pulls from the source, calls `inspector` on the value, and yields
    /// it.
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

/// Lazy `stepBy` — yields every `step`-th element, starting with the
/// first. Returned by `Iterator.stepBy(n:)`.
///
/// # Representation
///
/// Source iterator + step size + a one-bit `first` flag (the first
/// element is always emitted; subsequent ones consume `step - 1` extra
/// pulls).
public struct StepByIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var step: Int64
    internal var first: Bool

    /// @name From Source
    /// Builds a `StepByIterator`. Caller guarantees `step >= 1`; `step
    /// == 0` produces undefined behaviour.
    public init(inner inner: I, step step: Int64) {
        self.inner = inner;
        self.step = step;
        self.first = true;
    }

    /// Yields the first element on the first call; subsequently drains
    /// `step - 1` elements and yields the next.
    public mutating func next() -> I.Item? {
        if self.first {
            self.first = false;
            return self.inner.next()
        }

        var i = 0;
        while i < self.step - 1 {
            let _ = self.inner.next();
            i = i + 1;
        }
        self.inner.next()
    }
}

// ============================================================================
// REVERSAL ADAPTERS
// ============================================================================

/// Wraps a `DoubleEndedIterator` to walk it back to front. The
/// `Iterator` conformance is added by the `extend RevIterator[I]:
/// DoubleEndedIterator` block in `iterator.ks`. Returned by
/// `DoubleEndedIterator.rev()`.
///
/// # Representation
///
/// Just the inner iterator — no buffering.
public struct RevIterator[I]: Iterator where I: DoubleEndedIterator, I: Iterator {
    type Item = I.Item

    internal var inner: I

    /// @name From Source
    /// Builds a `RevIterator`. Prefer `inner.rev()`.
    public init(inner inner: I) {
        self.inner = inner;
    }
}



// ============================================================================
// SCANNING ADAPTERS
// ============================================================================

/// Lazy `scan` — yields the running fold accumulator after each step.
/// Returned by `Iterator.scan(initial:combine:)`.
///
/// # Representation
///
/// Source iterator + the running accumulator state + the combine
/// closure.
public struct ScanIterator[I, Acc]: Iterator where I: Iterator {
    type Item = Acc

    internal var inner: I
    internal var state: Acc
    internal var combine: (Acc, I.Item) -> Acc

    /// @name From Source
    /// Builds a `ScanIterator` seeded with `initial`.
    public init(inner inner: I, initial initial: Acc, combine combine: (Acc, I.Item) -> Acc) {
        self.inner = inner;
        self.state = initial;
        self.combine = combine;
    }

    /// Pulls the next element, updates `state`, and yields the new
    /// state.
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

/// Lazy `intersperse` — inserts a copy of `separator` between
/// consecutive elements. Returned by `Iterator.intersperse(separator:)`.
///
/// # Representation
///
/// Source iterator + separator value + a `needsSeparator` flag + a
/// one-slot pending-element buffer (used to remember an element while a
/// separator is being yielded).
public struct IntersperseIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var separator: I.Item
    internal var needsSeparator: Bool
    internal var pendingItem: I.Item?

    /// @name From Source
    /// Builds an `IntersperseIterator`.
    public init(inner inner: I, separator separator: I.Item) {
        self.inner = inner;
        self.separator = separator;
        self.needsSeparator = false;
        self.pendingItem = .None;
    }

    /// Returns the buffered element if one is pending, otherwise pulls
    /// the next source element — yielding a separator instead the second
    /// time around.
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

/// Lazy `intersperseWith` — like `IntersperseIterator`, but builds each
/// separator on demand by calling a closure. Returned by
/// `Iterator.intersperseWith(separator:)`.
///
/// # Representation
///
/// Same as `IntersperseIterator`, except the stored value is a
/// zero-arg closure producing fresh separators.
public struct IntersperseWithIterator[I]: Iterator where I: Iterator {
    type Item = I.Item

    internal var inner: I
    internal var separator: () -> I.Item
    internal var needsSeparator: Bool
    internal var pendingItem: I.Item?

    /// @name From Source
    /// Builds an `IntersperseWithIterator`.
    public init(inner inner: I, separator separator: () -> I.Item) {
        self.inner = inner;
        self.separator = separator;
        self.needsSeparator = false;
        self.pendingItem = .None;
    }

    /// Same logic as `IntersperseIterator.next`, but each separator is
    /// produced by calling `separator()`.
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

/// Returns an `EmptyIterator[T]`. Useful as a "neutral element" in
/// iterator algebra (`a.chain(other: empty())`).
public func empty[T]() -> EmptyIterator[T] {
    EmptyIterator()
}

/// Returns a `OnceIterator` that yields `value` and then nothing.
/// Equivalent to `[value].iter()` without the array allocation.
public func once[T](value: T) -> OnceIterator[T] {
    OnceIterator(value: value)
}

/// Returns a `RepeatIterator` that yields copies of `value` forever.
/// Combine with `take` to cap it.
public func repeatValue[T](value: T) -> RepeatIterator[T] {
    RepeatIterator(value: value)
}

/// Returns a `RepeatNIterator` that yields `count` copies of `value`,
/// then stops.
public func repeatN[T](value: T, count: Int64) -> RepeatNIterator[T] {
    RepeatNIterator(value: value, count: count)
}
