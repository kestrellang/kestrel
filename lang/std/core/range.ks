// Range operator protocols and types
// Provides range types for iteration and containment checks.

module std.core

import std.core.(Equatable, Comparable, Bool)
import std.num.(Steppable)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)

/// Raw protocol backing the half-open `..<` operator (`start..<end`).
///
/// `Output` is the range type produced — usually `Range[Self]`, but
/// custom types may produce their own range flavor (e.g. a date range).
@builtin(.ExclusiveRangeOperatorProtocol)
public protocol RangeConstructible[Rhs = Self] {
    type Output

    /// Builds the half-open range `[self, end)`.
    @builtin(.ExclusiveRangeOperatorMethod)
    func exclusiveRange(to end: Rhs) -> Output
}

/// Raw protocol backing the closed `..=` operator (`start..=end`).
@builtin(.InclusiveRangeOperatorProtocol)
public protocol ClosedRangeConstructible[Rhs = Self] {
    type Output

    /// Builds the closed range `[self, end]`.
    @builtin(.InclusiveRangeOperatorMethod)
    func inclusiveRange(to end: Rhs) -> Output
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
    type Iter = RangeIterator[T]

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

    /// Returns `true` when `start >= end` (no values are produced).
    public func isEmpty() -> Bool {
        self.start >= self.end
    }

    /// Equal when both bounds match. Useful for range-keyed lookups and
    /// tests, not a structural property of the iteration order.
    public func equals(other: Range[T]) -> Bool {
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
    type Iter = ClosedRangeIterator[T]

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

    /// Returns `true` when `start > end` (no values are produced).
    public func isEmpty() -> Bool {
        self.start > self.end
    }

    /// Equal when both bounds match.
    public func equals(other: ClosedRange[T]) -> Bool {
        self.start == other.start and self.end == other.end
    }

    /// Returns a fresh iterator over the range.
    public func iter() -> ClosedRangeIterator[T] {
        ClosedRangeIterator(current: self.start, end: self.end, finished: false)
    }
}
