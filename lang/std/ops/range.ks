// Range operator protocols and types

module std.ops

import std.core.(Equatable, Comparable, Steppable)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)

@builtin(.ExclusiveRangeOperatorProtocol)
public protocol RangeConstructible[Rhs = Self] {
    type Output

    @builtin(.ExclusiveRangeOperatorMethod)
    func exclusiveRange(to end: Rhs) -> Output
}

@builtin(.InclusiveRangeOperatorProtocol)
public protocol ClosedRangeConstructible[Rhs = Self] {
    type Output

    @builtin(.InclusiveRangeOperatorMethod)
    func inclusiveRange(to end: Rhs) -> Output
}

// Range type (exclusive end)
public struct Range[T]: Equatable where T: Steppable and Comparable {
    public var start: T
    public var end: T

    public init(start: T, end: T) {
        self.start = start;
        self.end = end;
    }

    public func contains(value: T) -> Bool {
        value >= self.start and value < self.end
    }

    public var isEmpty: Bool {
        self.start >= self.end
    }

    public func equals(other: Range[T]) -> Bool {
        self.start == other.start and self.end == other.end
    }
}

extend Range[T]: Iterable where T: Steppable {
    type Item = T
    type Iter = RangeIterator[T]

    public func iter() -> RangeIterator[T] {
        RangeIterator(current: self.start, end: self.end)
    }
}

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

// ClosedRange type (inclusive end)
public struct ClosedRange[T]: Equatable where T: Steppable, T: Comparable {
    public var start: T
    public var end: T

    public init(start: T, end: T) {
        self.start = start;
        self.end = end;
    }

    public func contains(value: T) -> Bool {
        value >= self.start and value <= self.end
    }

    public var isEmpty: Bool {
        self.start > self.end
    }

    public func equals(other: ClosedRange[T]) -> Bool {
        self.start == other.start and self.end == other.end
    }
}

extend ClosedRange[T]: Iterable where T: Steppable {
    type Item = T
    type Iter = ClosedRangeIterator[T]

    public func iter() -> ClosedRangeIterator[T] {
        ClosedRangeIterator(current: self.start, end: self.end, finished: false)
    }
}

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
