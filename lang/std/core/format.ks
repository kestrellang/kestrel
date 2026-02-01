// Format Options - Core types for text formatting.

module std.core

import std.core.(Bool, Equatable, Matchable)
import std.num.(Int64)
import std.text.(String, Char)
import std.result.(Optional)

// ============================================================================
// FORMAT ENUMS
// ============================================================================

/// Text alignment for formatted output.
public enum Alignment: Equatable, Matchable {
    /// Align text to the left.
    case left
    /// Align text to the right.
    case right
    /// Center the text.
    case center

    /// Compares for equality.
    public func equals(other: Alignment) -> Bool {
        match (self, other) {
            (.left, .left) => true,
            (.right, .right) => true,
            (.center, .center) => true,
            _ => false
        }
    }

    /// Matches for pattern matching.
    public func matches(other: Alignment) -> Bool {
        self.equals(other)
    }
}

/// Sign display mode for numeric formatting.
public enum Sign: Equatable, Matchable {
    /// Only show sign for negative numbers (default).
    case negative
    /// Always show sign (+ or -).
    case always
    /// Show space for positive, minus for negative.
    case space

    /// Compares for equality.
    public func equals(other: Sign) -> Bool {
        match (self, other) {
            (.negative, .negative) => true,
            (.always, .always) => true,
            (.space, .space) => true,
            _ => false
        }
    }

    /// Matches for pattern matching.
    public func matches(other: Sign) -> Bool {
        self.equals(other)
    }
}

/// Float display style.
public enum FloatStyle: Equatable, Matchable {
    /// Default: use shortest representation.
    case auto
    /// Fixed-point notation (e.g., "3.14").
    case fixed
    /// Scientific notation with lowercase 'e' (e.g., "3.14e0").
    case scientific
    /// Scientific notation with uppercase 'E' (e.g., "3.14E0").
    case scientificUpper
    /// Percentage (multiplies by 100, adds '%').
    case percent

    /// Compares for equality.
    public func equals(other: FloatStyle) -> Bool {
        match (self, other) {
            (.auto, .auto) => true,
            (.fixed, .fixed) => true,
            (.scientific, .scientific) => true,
            (.scientificUpper, .scientificUpper) => true,
            (.percent, .percent) => true,
            _ => false
        }
    }

    /// Matches for pattern matching.
    public func matches(other: FloatStyle) -> Bool {
        self.equals(other)
    }
}

// ============================================================================
// FORMAT OPTIONS
// ============================================================================

/// Options for controlling formatted output.
public struct FormatOptions: Equatable {
    /// Minimum field width.
    public var width: Int64?

    /// For floats: decimal places. For strings: max characters.
    public var precision: Int64?

    /// Text alignment within the field width.
    public var alignment: Alignment

    /// Character used for padding (default: ' ').
    public var fill: Char

    /// Numeric base: 2 (binary), 8 (octal), 10 (decimal), 16 (hex).
    public var radix: Int64

    /// Use uppercase for hex digits (A-F vs a-f).
    public var uppercase: Bool

    /// How to display the sign for numbers.
    public var sign: Sign

    /// Alternate form: show 0x/0b/0o prefix for non-decimal radix.
    public var alternate: Bool

    /// Float display style (fixed, scientific, percent).
    public var floatStyle: FloatStyle

    /// Debug mode: show structural representation.
    public var debug: Bool

    /// Default format options.
    public static func default() -> FormatOptions {
        FormatOptions()
    }

    /// Creates default format options.
    public init() {
        self.width = .None;
        self.precision = .None;
        self.alignment = .left;
        self.fill = ' ';
        self.radix = 10;
        self.uppercase = false;
        self.sign = .negative;
        self.alternate = false;
        self.floatStyle = .auto;
        self.debug = false;
    }

    /// Compares two format options for equality.
    public func equals(other: FormatOptions) -> Bool {
        if self.alignment != other.alignment { return false }
        if self.radix != other.radix { return false }
        if self.uppercase != other.uppercase { return false }
        if self.sign != other.sign { return false }
        if self.alternate != other.alternate { return false }
        if self.floatStyle != other.floatStyle { return false }
        if self.debug != other.debug { return false }
        true
    }
}
