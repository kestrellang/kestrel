// Bool type

import std.ffi.(FFISafe)

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
    public init(boolLiteral value: Bool) {
        self.value = value.value
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

    // And
    type Output = Bool

    public func and(other: Bool) -> Bool {
        Bool(value: lang.bool_and(self.value, other.value))
    }

    // Or
    public func or(other: Bool) -> Bool {
        Bool(value: lang.bool_or(self.value, other.value))
    }

    // Not
    public func not() -> Bool {
        Bool(value: lang.bool_not(self.value))
    }
}

// Constants
public let true: Bool = Bool(value: lang.bool_true)
public let false: Bool = Bool(value: lang.bool_false)
