// Bool - The boolean type representing true/false values

module std.core

import std.ffi.(FFISafe)
import std.core.(Hash, Hasher)
import std.text.(String, FormatOptions, Formattable)
import std.numeric.(UInt8, Int64)
import std.memory.(Slice, Pointer)

/// Two-state truth value with `true` and `false` as its only inhabitants.
///
/// `Bool` is the canonical conformer of every logical, conditional, and
/// equality protocol in `std.core`: equality, matching, hashing, formatting,
/// `and`/`or`/`not`, plus FFI compatibility for crossing the C boundary as
/// a single byte. Custom types rarely need to wrap `Bool`; conform to the
/// individual protocols (e.g. `BooleanConditional`) instead.
///
/// # Examples
///
/// ```
/// let alive = true;
/// if alive { greet() }
///
/// let votes = [true, false, true];
/// let yesCount = votes.iter().filter({ |b| b }).count();   // 2
/// ```
///
/// # Representation
///
/// Wraps a single `lang.i1`. The runtime promotes to a byte at FFI
/// boundaries (`FFISafe` conformance).
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

    /// @name Bool Literal
    /// Builds a `Bool` from the primitive `lang.i1` produced by a literal.
    public init(boolLiteral value: lang.i1) {
        self.value = value
    }

    // ========================================================================
    // EQUALITY AND MATCHING
    // ========================================================================

    /// Returns `true` if both bits agree. Drives `==` for `Bool`.
    public func equals(other: Bool) -> Bool {
        Bool(boolLiteral: lang.i1_eq(self.value, other.value))
    }

    /// Pattern-match form of `equals`: `case true =>` and `case false =>`
    /// dispatch through here.
    public func matches(other: Bool) -> Bool {
        Bool(boolLiteral: lang.i1_eq(self.value, other.value))
    }

    // ========================================================================
    // HASHING
    // ========================================================================

    /// Feeds a single `0` or `1` byte into `hasher`. Compatible with how the
    /// stdlib hashes other primitives — equal `Bool`s always hash equal.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        if self.value {
            hasher.write(Slice(pointer: Pointer(to: 1), count: 1))
        } else {
            hasher.write(Slice(pointer: Pointer(to: 0), count: 1))
        }
    }

    // ========================================================================
    // LOGICAL OPERATIONS
    // ========================================================================

    // Associated type bindings
    type And.Output = Bool
    type Or.Output = Bool
    type Not.Output = Bool

    /// Short-circuiting `and`: `other` runs only when `self` is `true`.
    /// The closure form is what the `and` keyword lowers into; users
    /// typically write `a and b` rather than calling this directly.
    public func logicalAnd(other: () -> Bool) -> Bool {
        if self.value { other() } else { Bool(boolLiteral: false) }
    }

    /// Short-circuiting `or`: `other` runs only when `self` is `false`.
    public func logicalOr(other: () -> Bool) -> Bool {
        if self.value { Bool(boolLiteral: true) } else { other() }
    }

    /// Bit-flip; `not true == false`.
    public func logicalNot() -> Bool {
        Bool(boolLiteral: lang.i1_not(self.value))
    }

    // ========================================================================
    // CONDITIONAL SUPPORT
    // ========================================================================

    /// Returns the wrapped `lang.i1` so `if`/`while` can branch on it
    /// without a redundant `Bool` round-trip.
    public func boolValue() -> lang.i1 {
        self.value
    }

    // ========================================================================
    // FORMATTING
    // ========================================================================

    /// Renders as `"true"` / `"false"`. With `options.debug`, wraps as
    /// `"Bool(true)"` / `"Bool(false)"` for diagnostic dumps.
    ///
    /// # Examples
    ///
    /// ```
    /// true.format()                                       // "true"
    /// false.format(FormatOptions.debug())                 // "Bool(false)"
    /// ```
    public func format(options: FormatOptions = FormatOptions.default()) -> String {
        let value = if self.value { "true" } else { "false" };
        if options.debug { "Bool(" + value + ")" } else { value }
    }
}
