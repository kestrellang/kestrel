// Formatting - Types and protocols for text formatting and string interpolation.

module std.text

import std.core.(Bool, Equatable, Matchable, ExpressibleByStringLiteral)
import std.num.(Int64)
import std.text.(String, Char)
import std.result.(Optional)
import std.collections.(Array)

// ============================================================================
// FORMAT ENUMS
// ============================================================================

/// Text alignment for formatted output.
public enum Alignment: Equatable, Matchable {
    /// Align text to the left.
    case Left
    /// Align text to the right.
    case Right
    /// Center the text.
    case Center

    /// Compares for equality.
    public func equals(other: Alignment) -> Bool {
        match (self, other) {
            (.Left, .Left) => true,
            (.Right, .Right) => true,
            (.Center, .Center) => true,
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
    case Negative
    /// Always show sign (+ or -).
    case Always
    /// Show space for positive, minus for negative.
    case Space

    /// Compares for equality.
    public func equals(other: Sign) -> Bool {
        match (self, other) {
            (.Negative, .Negative) => true,
            (.Always, .Always) => true,
            (.Space, .Space) => true,
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
    case Auto
    /// Fixed-point notation (e.g., "3.14").
    case Fixed
    /// Scientific notation with lowercase 'e' (e.g., "3.14e0").
    case Scientific
    /// Scientific notation with uppercase 'E' (e.g., "3.14E0").
    case ScientificUpper
    /// Percentage (multiplies by 100, adds '%').
    case Percent

    /// Compares for equality.
    public func equals(other: FloatStyle) -> Bool {
        match (self, other) {
            (.Auto, .Auto) => true,
            (.Fixed, .Fixed) => true,
            (.Scientific, .Scientific) => true,
            (.ScientificUpper, .ScientificUpper) => true,
            (.Percent, .Percent) => true,
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
/// Used by Formattable and string interpolation syntax: "\{expr:spec}"
///
/// Format spec mini-language:
///   [[fill]align][sign][#][0][width][.precision][type]
///
/// Where type is one of:
///   Integers: 'b' (binary), 'o' (octal), 'x' (hex lower), 'X' (hex upper)
///   Floats:   'f' (fixed), 'e' (scientific), 'E' (scientific upper), '%' (percent)
///   Any:      '?' (debug)
///
/// Examples:
///   "\{n:>8}"      right-align, width 8
///   "\{n:08x}"     zero-pad, width 8, hex
///   "\{n:#X}"      hex upper with 0x prefix
///   "\{pi:.2}"     precision 2 decimal places
///   "\{pi:.2e}"    scientific with 2 decimal places
///   "\{ratio:%}"   as percentage (0.5 -> "50%")
///   "\{name:^10}"  center, width 10
///   "\{value:?}"   debug format
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
        self.alignment = Alignment.Left;
        self.fill = ' ';
        self.radix = 10;
        self.uppercase = false;
        self.sign = Sign.Negative;
        self.alternate = false;
        self.floatStyle = FloatStyle.Auto;
        self.debug = false;
    }

    /// Compares two format options for equality.
    public func equals(other: FormatOptions) -> Bool {
        if self.alignment.equals(other.alignment) == false { return false }
        if self.radix != other.radix { return false }
        if self.uppercase != other.uppercase { return false }
        if self.sign.equals(other.sign) == false { return false }
        if self.alternate != other.alternate { return false }
        if self.floatStyle.equals(other.floatStyle) == false { return false }
        if self.debug != other.debug { return false }
        true
    }
}

// ============================================================================
// FORMATTABLE PROTOCOL
// ============================================================================

/// Protocol for types that can be formatted as a string.
/// Used by print functions and string interpolation.
///
/// String interpolation syntax:
///   "\{expr}"        uses default formatting
///   "\{expr:spec}"   uses format spec (parsed into FormatOptions)
///
/// Examples:
///   "\{name}"        "Alice"
///   "\{name:>10}"    "     Alice"
///   "\{n:08x}"       "0000002a"
///   "\{pi:.2}"       "3.14"
///   "\{value:?}"     debug representation
@builtin(.FormattableProtocol)
public protocol Formattable {
    /// Returns this value formatted as a string with the given options.
    @builtin(.FormattableFormat)
    func format(options: FormatOptions = FormatOptions.default()) -> String
}

// ============================================================================
// STRING INTERPOLATION PROTOCOLS
// ============================================================================

/// Protocol for types that accumulate string interpolation parts.
public protocol StringInterpolationProtocol {
    /// Initialize with capacity hints for optimization.
    init(literalCapacity: Int64, interpolationCount: Int64)

    /// Append a literal string segment.
    mutating func appendLiteral(literal: String)

    /// Append a formatted value with options.
    mutating func appendInterpolation[T](value: T, options: FormatOptions) where T: Formattable
}

/// Protocol for types that can be created from string interpolation.
public protocol ExpressibleByStringInterpolation: ExpressibleByStringLiteral {
    /// The type used to accumulate interpolation parts.
    type Interpolation: StringInterpolationProtocol

    /// Create from a completed interpolation.
    init(interpolation: Interpolation)
}

// ============================================================================
// DEFAULT STRING INTERPOLATION
// ============================================================================

/// Default implementation of StringInterpolationProtocol for String.
@builtin(.DefaultStringInterpolation)
public struct DefaultStringInterpolation: StringInterpolationProtocol, Cloneable {
    private var parts: Array[String]

    /// Initialize with capacity hints.
    @builtin(.DefaultStringInterpolationInit)
    public init(literalCapacity: Int64, interpolationCount: Int64) {
        self.parts = [];
    }

    public func clone() -> DefaultStringInterpolation {
        var c = DefaultStringInterpolation(literalCapacity: 0, interpolationCount: 0);
        c.parts = self.parts.clone();
        c
    }

    /// Append a literal string segment.
    @builtin(.DefaultStringInterpolationAppendLiteral)
    public mutating func appendLiteral(literal: String) {
        if literal.isEmpty == false {
            self.parts.append(literal);
        }
    }

    /// Append a formatted value with options.
    @builtin(.DefaultStringInterpolationAppendInterpolation)
    public mutating func appendInterpolation[T](value: T, options: FormatOptions = FormatOptions.default()) where T: Formattable {
        self.parts.append(value.format(options));
    }

    /// Build the final string by concatenating all parts.
    @builtin(.DefaultStringInterpolationBuild)
    public func build() -> String {
        let partsCount = self.parts.count;
        if partsCount == 0 {
            return ""
        }
        if partsCount == 1 {
            return self.parts(unchecked: 0)
        }
        var totalBytes: Int64 = 0;
        var j: Int64 = 0;
        while j < partsCount {
            totalBytes = totalBytes + self.parts(unchecked: j).byteCount;
            j = j + 1;
        }
        var result = String(capacity: totalBytes);
        var i: Int64 = 0;
        while i < partsCount {
            result.append(self.parts(unchecked: i));
            i = i + 1;
        }
        result
    }
}
