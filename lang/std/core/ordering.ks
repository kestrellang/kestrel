// Ordering enum for comparison results

module std.core

import std.core.(Equatable)
import std.ops.(Equal, NotEqual)

public enum Ordering: Equatable, Equal[Self], NotEqual[Self] {
    type Equal.Output = Bool
    type NotEqual.Output = Bool
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
            .Greater => .Less,

            _ => .Equal, // todo: remove this case
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
