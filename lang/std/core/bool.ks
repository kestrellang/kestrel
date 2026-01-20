// Bool type

module std.core

import std.ffi.(FFISafe)
import std.text.(String)

public struct Bool:
    Equatable,
    Matchable,
    Formattable,
    And[Bool],
    Or[Bool],
    Not,
    ExpressibleByBoolLiteral,
    BooleanConditional,
    FFISafe
{
    private var value: lang.i1

    // ExpressibleByBoolLiteral
    public init(boolLiteral value: lang.i1) {
        self.value = value
    }

    // Equatable
    public func equals(other: Bool) -> Bool {
        Bool(boolLiteral: lang.i1_eq(self.value, other.value))
    }

    // Matchable
    public func matches(other: Bool) -> Bool {
        Bool(boolLiteral: lang.i1_eq(self.value, other.value))
    }

    // Hashable - deferred until Hasher has write method
    // public func hash[H](mutating into hasher: H) where H: Hasher {
    //     ...
    // }

    // Associated type bindings
    type And.Output = Bool
    type Or.Output = Bool
    type Not.Output = Bool

    // And
    public func logicalAnd(other: Bool) -> Bool {
        Bool(boolLiteral: lang.i1_and(self.value, other.value))
    }

    // Or
    public func logicalOr(other: Bool) -> Bool {
        Bool(boolLiteral: lang.i1_or(self.value, other.value))
    }

    // Not
    public func logicalNot() -> Bool {
        Bool(boolLiteral: lang.i1_not(self.value))
    }

    // BooleanConditional
    public func boolValue() -> lang.i1 {
        self.value
    }

    // Formattable
    public func format() -> String {
        if self.value { "true" } else { "false" }
    }
}
