// Ordering enum for comparison results

module std.core

import std.text.(String)
import std.core.(FormatOptions)

/// Represents the result of comparing two values.
/// Used by the Comparable protocol to express total ordering.
public enum Ordering: Equatable, Formattable {
    /// The first value is less than the second.
    case Less
    /// The values are equal.
    case Equal
    /// The first value is greater than the second.
    case Greater

    /// Compares this ordering with another for equality.
    public func equals(other: Ordering) -> Bool {
        match (self, other) {
            (.Less, .Less) => true,
            (.Equal, .Equal) => true,
            (.Greater, .Greater) => true,
            _ => false
        }
    }

    /// Compares this ordering with another for inequality.
    public func notEquals(other: Ordering) -> Bool {
        if self.equals(other) { false } else { true }
    }

    /// Reverses this ordering (Less becomes Greater and vice versa).
    public func reverse() -> Ordering {
        match self {
            .Less => .Greater,
            .Equal => .Equal,
            .Greater => .Less
        }
    }

    /// Returns this ordering if not Equal, otherwise returns the other ordering.
    /// Useful for chaining comparisons by multiple fields.
    public func then(other: Ordering) -> Ordering {
        match self {
            .Equal => other,
            _ => self
        }
    }

    /// Returns this ordering if not Equal, otherwise evaluates and returns the comparison.
    /// Lazy version of then() that only evaluates the closure if needed.
    public func thenWith(compare: () -> Ordering) -> Ordering {
        match self {
            .Equal => compare(),
            _ => self
        }
    }

    /// Formats this ordering as a string.
    public func format(options: FormatOptions = FormatOptions.default()) -> String {
        let value = match self {
            .Less => "Less",
            .Equal => "Equal",
            .Greater => "Greater"
        };
        if options.debug { "Ordering." + value } else { value }
    }
}
