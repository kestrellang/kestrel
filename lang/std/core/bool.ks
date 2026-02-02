// Bool - The boolean type representing true/false values

module std.core

import std.ffi.(FFISafe)
import std.text.(String)
import std.core.(FormatOptions)
import std.num.(UInt8)
import std.memory.(Slice, Pointer)

/// The boolean type with support for logical operations, equality, hashing, and formatting.
/// Bool conforms to Equatable, Matchable, Formattable, Hash, And, Or, Not,
/// ExpressibleByBoolLiteral, BooleanConditional, and FFISafe protocols.
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

    // ========================================================================
    // INITIALIZATION
    // ========================================================================

    /// Creates a Bool from a boolean literal value.
    public init(boolLiteral value: lang.i1) {
        self.value = value
    }

    // ========================================================================
    // EQUALITY AND MATCHING
    // ========================================================================

    /// Compares this Bool with another for equality.
    /// Returns true if both values are the same.
    public func equals(other: Bool) -> Bool {
        Bool(boolLiteral: lang.i1_eq(self.value, other.value))
    }

    /// Matches this Bool against another in pattern matching contexts.
    /// Returns true if both values are the same.
    public func matches(other: Bool) -> Bool {
        Bool(boolLiteral: lang.i1_eq(self.value, other.value))
    }

    // ========================================================================
    // HASHING
    // ========================================================================

    /// Hashes this Bool value into the given hasher.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        if self.value {
            hasher.write(Slice(pointer: Pointer(to: UInt8(intLiteral: 1)), count: std.num.Int64(intLiteral: 1)))
        } else {
            hasher.write(Slice(pointer: Pointer(to: UInt8(intLiteral: 0)), count: std.num.Int64(intLiteral: 1)))
        }
    }

    // ========================================================================
    // LOGICAL OPERATIONS
    // ========================================================================

    // Associated type bindings
    type And.Output = Bool
    type Or.Output = Bool
    type Not.Output = Bool

    /// Logical AND with short-circuit evaluation.
    /// The closure is only evaluated if self is true.
    public func logicalAnd(other: () -> Bool) -> Bool {
        if self.value { other() } else { Bool(boolLiteral: false) }
    }

    /// Logical OR with short-circuit evaluation.
    /// The closure is only evaluated if self is false.
    public func logicalOr(other: () -> Bool) -> Bool {
        if self.value { Bool(boolLiteral: true) } else { other() }
    }

    /// Logical NOT - returns the inverse of this boolean.
    public func logicalNot() -> Bool {
        Bool(boolLiteral: lang.i1_not(self.value))
    }

    // ========================================================================
    // CONDITIONAL SUPPORT
    // ========================================================================

    /// Returns the underlying boolean value for use in conditionals.
    public func boolValue() -> lang.i1 {
        self.value
    }

    // ========================================================================
    // FORMATTING
    // ========================================================================

    /// Formats this Bool as a string.
    /// Default: "true" or "false".
    /// Debug: "Bool(true)" or "Bool(false)".
    public func format(options: FormatOptions = FormatOptions.default()) -> String {
        let value = if self.value { "true" } else { "false" };
        if options.debug { "Bool(" + value + ")" } else { value }
    }
}
