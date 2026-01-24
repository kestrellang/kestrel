// Bool type

module std.core

import std.ffi.(FFISafe)
import std.text.(String)
import std.num.(UInt8)
import std.memory.(Slice, Pointer)

public struct Bool:
    Equatable,
    Matchable,
    Formattable,
    Hash,
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

    // Hash
    public func hash[H](mutating into hasher: H) where H: Hasher {
        if self.value {
            hasher.write(Slice(pointer: Pointer(to: UInt8(intLiteral: 1)), count: std.num.Int64(intLiteral: 1)))
        } else {
            hasher.write(Slice(pointer: Pointer(to: UInt8(intLiteral: 0)), count: std.num.Int64(intLiteral: 1)))
        }
    }

    // Associated type bindings
    type And.Output = Bool
    type Or.Output = Bool
    type Not.Output = Bool

    // And - short-circuit: only evaluate other() if self is true
    public func logicalAnd(other: () -> Bool) -> Bool {
        if self.value { other() } else { Bool(boolLiteral: false) }
    }

    // Or - short-circuit: only evaluate other() if self is false
    public func logicalOr(other: () -> Bool) -> Bool {
        if self.value { Bool(boolLiteral: true) } else { other() }
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
