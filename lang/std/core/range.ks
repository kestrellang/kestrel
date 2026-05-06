// Range operator protocols and types
// Provides range types for iteration and containment checks.

module std.core

import std.core.(Equatable, Comparable, Bool)
import std.numeric.(Steppable)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)

// ----- Binary range construction protocols -----

/// Raw protocol backing the half-open `..<` operator (`start..<end`).
///
/// `Output` is the range type produced — usually `Range[Self]`, but
/// custom types may produce their own range flavor (e.g. a date range).
@builtin(.ExclusiveRangeOperatorProtocol)
public protocol RangeConstructible[Other = Self] {
    type Output

    /// Builds the half-open range `[self, end)`.
    @builtin(.ExclusiveRangeOperatorMethod)
    func exclusiveRange(to end: Other) -> Output
}

/// Raw protocol backing the closed `..=` operator (`start..=end`).
@builtin(.InclusiveRangeOperatorProtocol)
public protocol ClosedRangeConstructible[Other = Self] {
    type Output

    /// Builds the closed range `[self, end]`.
    @builtin(.InclusiveRangeOperatorMethod)
    func inclusiveRange(to end: Other) -> Output
}

/// Iterator over a half-open `Range[T]`. Yields successive values via
/// `Steppable.successor()` until reaching (but not including) `end`.
///
/// # Representation
///
/// Two values: `current` (next yield) and `end` (sentinel).
public struct RangeIterator[T]: Iterator where T: Steppable, T: Comparable {
    type Item = T

    private var current: T
    private var end: T

    /// @name From Bounds
    /// Builds an iterator that yields `current`, `current.successor()`, …
    /// stopping before `end`.
    public init(current current: T, end end: T) {
        self.current = current;
        self.end = end;
    }

    /// Yields the next value, or `.None` when exhausted.
    public mutating func next() -> T? {
        if self.current < self.end {
            let value = self.current;
            self.current = self.current.successor();
            .Some(value)
        } else {
            .None
        }
    }
}

/// Half-open range `[start, end)` — produced by the `..<` operator.
///
/// `Range` is `Iterable`, so `for x in 0..<10 { … }` works directly.
/// `T` must be `Steppable` (defines `successor()`) and `Comparable` (so
/// the iterator knows when to stop). Empty ranges (`start >= end`) yield
/// nothing.
///
/// # Examples
///
/// ```
/// for i in 0..<3 { print(i) }   // 0, 1, 2
/// (0..<10).contains(5)          // true
/// (0..<0).isEmpty()             // true
/// ```
///
/// # Representation
///
/// Two values: `start` and `end`. No heap allocation.
public struct Range[T]: Equatable, Iterable where T: Steppable, T: Comparable {
    type Item = T
    type TargetIterator = RangeIterator[T]

    /// Lower bound — included in the range.
    public var start: T
    /// Upper bound — excluded from the range.
    public var end: T

    /// @name From Bounds
    /// Builds the range `[start, end)`.
    public init(start: T, end: T) {
        self.start = start;
        self.end = end;
    }

    /// Returns `true` iff `start <= value < end`.
    public func contains(value: T) -> Bool {
        value >= self.start and value < self.end
    }

    /// `true` when `start >= end` (no values are produced).
    public var isEmpty: Bool {
        self.start >= self.end
    }

    /// Equal when both bounds match. Useful for range-keyed lookups and
    /// tests, not a structural property of the iteration order.
    public func isEqual(to other: Range[T]) -> Bool {
        self.start == other.start and self.end == other.end
    }

    /// Returns a fresh iterator over the range. Multiple calls produce
    /// independent iterators — `Range` is value-typed.
    public func iter() -> RangeIterator[T] {
        RangeIterator(current: self.start, end: self.end)
    }
}

/// Iterator over a `ClosedRange[T]`. Differs from `RangeIterator` in
/// that it yields `end` and uses an extra `finished` bit so it can
/// terminate after emitting the upper bound.
///
/// # Representation
///
/// `current`, `end`, and a one-bit `finished` flag.
public struct ClosedRangeIterator[T]: Iterator where T: Steppable, T: Comparable {
    type Item = T

    private var current: T
    private var end: T
    private var finished: Bool

    /// @name From Bounds
    /// Builds an iterator yielding `current` through `end` inclusive.
    /// Pass `finished: true` to construct an already-exhausted iterator.
    public init(current current: T, end end: T, finished finished: Bool) {
        self.current = current;
        self.end = end;
        self.finished = finished;
    }

    /// Yields the next value, or `.None` when past `end`.
    public mutating func next() -> T? {
        if self.finished {
            .None
        } else if self.current == self.end {
            self.finished = true;
            .Some(self.current)
        } else if self.current < self.end {
            let value = self.current;
            self.current = self.current.successor();
            .Some(value)
        } else {
            .None
        }
    }
}

/// Closed range `[start, end]` — produced by the `..=` operator. Both
/// endpoints are included in iteration.
///
/// # Examples
///
/// ```
/// for i in 0..=3 { print(i) }   // 0, 1, 2, 3
/// (0..=10).contains(10)         // true (vs Range, which excludes the upper)
/// ```
///
/// # Representation
///
/// Two values: `start` and `end`. No heap allocation.
public struct ClosedRange[T]: Equatable, Iterable where T: Steppable, T: Comparable {
    type Item = T
    type TargetIterator = ClosedRangeIterator[T]

    /// Lower bound — included.
    public var start: T
    /// Upper bound — included.
    public var end: T

    /// @name From Bounds
    /// Builds the closed range `[start, end]`.
    public init(start: T, end: T) {
        self.start = start;
        self.end = end;
    }

    /// Returns `true` iff `start <= value <= end`.
    public func contains(value: T) -> Bool {
        value >= self.start and value <= self.end
    }

    /// `true` when `start > end` (no values are produced).
    public var isEmpty: Bool {
        self.start > self.end
    }

    /// Equal when both bounds match.
    public func isEqual(to other: ClosedRange[T]) -> Bool {
        self.start == other.start and self.end == other.end
    }

    /// Returns a fresh iterator over the range.
    public func iter() -> ClosedRangeIterator[T] {
        ClosedRangeIterator(current: self.start, end: self.end, finished: false)
    }
}

// ----- Partial range construction protocols -----

/// Protocol backing the postfix `..` operator (`start..`).
///
/// `Output` is the range type produced — usually `RangeFrom[Self]`.
@builtin(.RangeFromOperatorProtocol)
public protocol RangeFromConstructible {
    type Output

    /// Builds the partial range `[self, +∞)`.
    @builtin(.RangeFromOperatorMethod)
    func rangeFrom() -> Output
}

/// Protocol backing the prefix `..<` operator (`..<end`).
///
/// `Output` is the range type produced — usually `RangeUpTo[Self]`.
@builtin(.RangeUpToOperatorProtocol)
public protocol RangeUpToConstructible {
    type Output

    /// Builds the partial range `(-∞, self)`.
    @builtin(.RangeUpToOperatorMethod)
    func rangeUpTo() -> Output
}

/// Protocol backing the prefix `..=` operator (`..=end`).
///
/// `Output` is the range type produced — usually `RangeThrough[Self]`.
@builtin(.RangeThroughOperatorProtocol)
public protocol RangeThroughConstructible {
    type Output

    /// Builds the partial range `(-∞, self]`.
    @builtin(.RangeThroughOperatorMethod)
    func rangeThrough() -> Output
}

// ----- Partial range types -----

/// Iterator over a `RangeFrom[T]`. Yields successive values via
/// `Steppable.successor()` with no upper bound — callers must `break`.
///
/// # Representation
///
/// Single value: `current` (next yield).
public struct RangeFromIterator[T]: Iterator where T: Steppable, T: Comparable {
    type Item = T

    private var current: T

    /// @name From Start
    public init(current current: T) {
        self.current = current;
    }

    /// Yields the next value. Never returns `.None` — infinite iterator.
    public mutating func next() -> T? {
        let value = self.current;
        self.current = self.current.successor();
        .Some(value)
    }
}

/// Partial range `[start, +∞)` — produced by the postfix `..` operator.
///
/// `RangeFrom` is `Iterable` and produces an infinite iterator. Use
/// `break` to terminate iteration.
///
/// # Examples
///
/// ```
/// for i in 0.. {
///     if i >= 5 { break; }
///     print(i)
/// }
/// (10..).contains(42)   // true
/// ```
///
/// # Representation
///
/// Single value: `start`. No heap allocation.
public struct RangeFrom[T]: Equatable, Iterable where T: Steppable, T: Comparable {
    type Item = T
    type TargetIterator = RangeFromIterator[T]

    /// Lower bound — included in the range.
    public var start: T

    /// @name From Start
    public init(start: T) {
        self.start = start;
    }

    /// Returns `true` iff `value >= start`.
    public func contains(value: T) -> Bool {
        value >= self.start
    }

    /// Structural equality.
    public func isEqual(to other: RangeFrom[T]) -> Bool {
        self.start == other.start
    }

    /// Returns a fresh infinite iterator starting at `start`.
    public func iter() -> RangeFromIterator[T] {
        RangeFromIterator(current: self.start)
    }
}

/// Partial range `(-∞, end)` — produced by the prefix `..<` operator.
///
/// Not `Iterable` — there is no start to iterate from.
///
/// # Examples
///
/// ```
/// (..<10).contains(5)    // true
/// (..<10).contains(10)   // false
/// ```
///
/// # Representation
///
/// Single value: `end`. No heap allocation.
public struct RangeUpTo[T]: Equatable where T: Comparable {
    /// Upper bound — excluded from the range.
    public var end: T

    /// @name From End
    public init(end: T) {
        self.end = end;
    }

    /// Returns `true` iff `value < end`.
    public func contains(value: T) -> Bool {
        value < self.end
    }

    /// Structural equality.
    public func isEqual(to other: RangeUpTo[T]) -> Bool {
        self.end == other.end
    }
}

/// Partial range `(-∞, end]` — produced by the prefix `..=` operator.
///
/// Not `Iterable` — there is no start to iterate from.
///
/// # Examples
///
/// ```
/// (..=10).contains(10)   // true
/// (..=10).contains(11)   // false
/// ```
///
/// # Representation
///
/// Single value: `end`. No heap allocation.
public struct RangeThrough[T]: Equatable where T: Comparable {
    /// Upper bound — included in the range.
    public var end: T

    /// @name From End
    public init(end: T) {
        self.end = end;
    }

    /// Returns `true` iff `value <= end`.
    public func contains(value: T) -> Bool {
        value <= self.end
    }

    /// Structural equality.
    public func isEqual(to other: RangeThrough[T]) -> Bool {
        self.end == other.end
    }
}
