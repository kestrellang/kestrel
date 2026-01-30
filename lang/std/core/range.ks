// Range operator protocols and types
// Provides range types for iteration and containment checks.

module std.core

import std.core.(Equatable, Comparable, Bool)
import std.num.(Steppable)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)

/// Protocol for types that support the exclusive range operator (..<).
/// Example: 0..<10 creates a range from 0 to 9.
@builtin(.ExclusiveRangeOperatorProtocol)
public protocol RangeConstructible[Rhs = Self] {
    type Output

    /// Creates an exclusive range from this value to the end value.
    @builtin(.ExclusiveRangeOperatorMethod)
    func exclusiveRange(to end: Rhs) -> Output
}

/// Protocol for types that support the inclusive range operator (..=).
/// Example: 0..=10 creates a range from 0 to 10 (inclusive).
@builtin(.InclusiveRangeOperatorProtocol)
public protocol ClosedRangeConstructible[Rhs = Self] {
    type Output

    /// Creates an inclusive range from this value to the end value.
    @builtin(.InclusiveRangeOperatorMethod)
    func inclusiveRange(to end: Rhs) -> Output
}

/// Iterator for Range that yields values from start up to (but not including) end.
public struct RangeIterator[T]: Iterator where T: Steppable, T: Comparable {
    type Item = T

    private var current: T
    private var end: T

    /// Creates an iterator starting at current and ending before end.
    public init(current current: T, end end: T) {
        self.current = current;
        self.end = end;
    }

    /// Returns the next value in the range, or None if exhausted.
    public mutating func next() -> Optional[T] {
        if self.current < self.end {
            let value = self.current;
            self.current = self.current.successor();
            .Some(value)
        } else {
            .None
        }
    }
}

/// A half-open range [start, end) that excludes the end value.
/// Created with the ..< operator. Example: 0..<10 includes 0 through 9.
public struct Range[T]: Equatable, Iterable where T: Steppable, T: Comparable {
    type Item = T
    type Iter = RangeIterator[T]

    /// The lower bound of the range (inclusive).
    public var start: T
    /// The upper bound of the range (exclusive).
    public var end: T

    /// Creates a range from start (inclusive) to end (exclusive).
    public init(start: T, end: T) {
        self.start = start;
        self.end = end;
    }

    /// Returns true if the value is within the range [start, end).
    public func contains(value: T) -> Bool {
        value >= self.start and value < self.end
    }

    /// Returns true if the range contains no elements (start >= end).
    public func isEmpty() -> Bool {
        self.start >= self.end
    }

    /// Compares this range with another for equality.
    public func equals(other: Range[T]) -> Bool {
        self.start == other.start and self.end == other.end
    }

    /// Returns an iterator over the range values.
    public func iter() -> RangeIterator[T] {
        RangeIterator(current: self.start, end: self.end)
    }
}

/// Iterator for ClosedRange that yields values from start through end (inclusive).
public struct ClosedRangeIterator[T]: Iterator where T: Steppable, T: Comparable {
    type Item = T

    private var current: T
    private var end: T
    private var finished: Bool

    /// Creates an iterator starting at current and ending at end (inclusive).
    public init(current current: T, end end: T, finished finished: Bool) {
        self.current = current;
        self.end = end;
        self.finished = finished;
    }

    /// Returns the next value in the range, or None if exhausted.
    public mutating func next() -> Optional[T] {
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

/// A closed range [start, end] that includes both endpoints.
/// Created with the ..= operator. Example: 0..=10 includes 0 through 10.
public struct ClosedRange[T]: Equatable, Iterable where T: Steppable, T: Comparable {
    type Item = T
    type Iter = ClosedRangeIterator[T]

    /// The lower bound of the range (inclusive).
    public var start: T
    /// The upper bound of the range (inclusive).
    public var end: T

    /// Creates a range from start to end (both inclusive).
    public init(start: T, end: T) {
        self.start = start;
        self.end = end;
    }

    /// Returns true if the value is within the range [start, end].
    public func contains(value: T) -> Bool {
        value >= self.start and value <= self.end
    }

    /// Returns true if the range contains no elements (start > end).
    public func isEmpty() -> Bool {
        self.start > self.end
    }

    /// Compares this range with another for equality.
    public func equals(other: ClosedRange[T]) -> Bool {
        self.start == other.start and self.end == other.end
    }

    /// Returns an iterator over the range values.
    public func iter() -> ClosedRangeIterator[T] {
        ClosedRangeIterator(current: self.start, end: self.end, finished: false)
    }
}
