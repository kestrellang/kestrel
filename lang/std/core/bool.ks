// Bool type

module std.core

import std.ffi.(FFISafe)
import std.ops.(And, Or, Not, ExpressibleByBoolLiteral)

public struct Bool:
    Equatable,
    Hashable,
    And[Bool],
    Or[Bool],
    Not,
    ExpressibleByBoolLiteral,
    FFISafe
{
    private var value: lang.bool

    // ExpressibleByBoolLiteral
    public init(boolLiteral value: lang.i1) {
        self.value = value
    }

    // Equatable
    public func equals(other: Bool) -> Bool {
        lang.bool_eq(self.value, other.value)
    }

    // Hashable
    public func hash[H](into hasher: H) where H: Hasher {
        if self.value {
            hasher.write(bytes: [1])
        } else {
            hasher.write(bytes: [0])
        }
    }

    // Associated type bindings (qualified to avoid ambiguity across protocols)
    type And.Output = Bool
    type Or.Output = Bool
    type Not.Output = Bool

    public func logicalAnd(other: Bool) -> Bool {
        Bool(value: lang.bool_and(self.value, other.value))
    }

    // Or
    public func logicalOr(other: Bool) -> Bool {
        Bool(value: lang.bool_or(self.value, other.value))
    }

    // Not
    public func logicalNot() -> Bool {
        Bool(value: lang.bool_not(self.value))
    }
}
