// Ordering enum for comparison results

module std.core

import std.core.(Equal as EqualOp, NotEqual as NotEqualOp)

public enum Ordering: Equatable, EqualOp[Self], NotEqualOp[Self] {
    type EqualOp.Output = Bool
    type NotEqualOp.Output = Bool

    case Less
    case Equal
    case Greater

    public func equals(other: Ordering) -> Bool {
        match (self, other) {
            (.Less, .Less) => true,
            (.Equal, .Equal) => true,
            (.Greater, .Greater) => true,
            _ => false
        }
    }

    public func notEquals(other: Ordering) -> Bool {
        match (self, other) {
            (.Less, .Less) => false,
            (.Equal, .Equal) => false,
            (.Greater, .Greater) => false,
            _ => true
        }
    }

    public func reverse() -> Ordering {
        match self {
            .Less => .Greater,
            .Equal => .Equal,
            .Greater => .Less
        }
    }

    public func then(other: Ordering) -> Ordering {
        match self {
            .Equal => other,
            _ => self
        }
    }

    public func thenWith(compare: () -> Ordering) -> Ordering {
        match self {
            .Equal => compare(),
            _ => self
        }
    }
}
