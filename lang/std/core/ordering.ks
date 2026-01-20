// Ordering enum for comparison results

module std.core

import std.text.(String)

// Note: Equal[Self] and NotEqual[Self] are automatically provided by
// the extension `extend Equatable: Equal[Self], NotEqual[Self]`

public enum Ordering: Equatable, Formattable {
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
        if self.equals(other) { false } else { true }
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

    // Formattable
    public func format() -> String {
        match self {
            .Less => "Less",
            .Equal => "Equal",
            .Greater => "Greater"
        }
    }
}
