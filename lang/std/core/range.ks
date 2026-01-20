// Range operator protocols and types

module std.core

import std.core.(Equatable, Comparable, Bool)
import std.num.(Steppable)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)

/// Protocol for types that support the exclusive range operator (..<)
@builtin(.ExclusiveRangeOperatorProtocol)
public protocol RangeConstructible[Rhs = Self] {
    type Output

    @builtin(.ExclusiveRangeOperatorMethod)
    func exclusiveRange(to end: Rhs) -> Output
}

/// Protocol for types that support the inclusive range operator (..=)
@builtin(.InclusiveRangeOperatorProtocol)
public protocol ClosedRangeConstructible[Rhs = Self] {
    type Output

    @builtin(.InclusiveRangeOperatorMethod)
    func inclusiveRange(to end: Rhs) -> Output
}

// Iterator for Range - must be defined before Range for Iterable conformance
public struct RangeIterator[T]: Iterator where T: Steppable, T: Comparable {
    type Item = T

    private var current: T
    private var end: T

    public init(current: T, end: T) {
        self.current = current;
        self.end = end;
    }

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

// Range type (exclusive end) - e.g., 0..<10 includes 0..9
public struct Range[T]: Equatable, Iterable where T: Steppable, T: Comparable {
    type Item = T
    type Iter = RangeIterator[T]

    public var start: T
    public var end: T

    public init(start: T, end: T) {
        self.start = start;
        self.end = end;
    }

    public func contains(value: T) -> Bool {
        value >= self.start and value < self.end
    }

    public func isEmpty() -> Bool {
        self.start >= self.end
    }

    public func equals(other: Range[T]) -> Bool {
        self.start == other.start and self.end == other.end
    }

    public func iter() -> RangeIterator[T] {
        RangeIterator(current: self.start, end: self.end)
    }
}

// Iterator for ClosedRange - must be defined before ClosedRange for Iterable conformance
public struct ClosedRangeIterator[T]: Iterator where T: Steppable, T: Comparable {
    type Item = T

    private var current: T
    private var end: T
    private var finished: Bool

    public init(current: T, end: T, finished: Bool) {
        self.current = current;
        self.end = end;
        self.finished = finished;
    }

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

// ClosedRange type (inclusive end) - e.g., 0..=10 includes 0..10
public struct ClosedRange[T]: Equatable, Iterable where T: Steppable, T: Comparable {
    type Item = T
    type Iter = ClosedRangeIterator[T]

    public var start: T
    public var end: T

    public init(start: T, end: T) {
        self.start = start;
        self.end = end;
    }

    public func contains(value: T) -> Bool {
        value >= self.start and value <= self.end
    }

    public func isEmpty() -> Bool {
        self.start > self.end
    }

    public func equals(other: ClosedRange[T]) -> Bool {
        self.start == other.start and self.end == other.end
    }

    public func iter() -> ClosedRangeIterator[T] {
        ClosedRangeIterator(current: self.start, end: self.end, finished: false)
    }
}
