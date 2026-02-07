// Formatting - Types and protocols for text formatting and string interpolation.

module std.text

import std.core.(Bool, Equatable, Matchable)
import std.num.(Int64)
import std.text.(String, Char)
import std.collections.(Array)
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
}

/// Sign display mode for numeric formatting.
public enum Sign: Equatable, Matchable {
    /// Only show sign for negative numbers (default).
    case negative
    /// Always show sign (+ or -).
    case always
    /// Show space for positive, minus for negative.
    case space
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
}

// ============================================================================
// FORMAT OPTIONS
// ============================================================================

/// Options for controlling formatted output.
///
/// Used by Formattable and string interpolation syntax: "\(expr:spec)"
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
///   "\(n:>8)"      right-align, width 8
///   "\(n:08x)"     zero-pad, width 8, hex
///   "\(n:#X)"      hex upper with 0x prefix
///   "\(pi:.2)"     precision 2 decimal places
///   "\(pi:.2e)"    scientific with 2 decimal places
///   "\(ratio:%)"   as percentage (0.5 -> "50%")
///   "\(name:^10)"  center, width 10
///   "\(value:?)"   debug format
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

    /// Debug mode: show structural representation (escapes strings, shows types).
    public var debug: Bool

    /// Default format options.
    public static let default: FormatOptions = FormatOptions()

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

    /// Creates format options with specified values.
    public init(
        width: Int64?,
        precision: Int64?,
        alignment: Alignment,
        fill: Char,
        radix: Int64,
        uppercase: Bool,
        sign: Sign,
        alternate: Bool,
        floatStyle: FloatStyle,
        debug: Bool
    ) {
        self.width = width;
        self.precision = precision;
        self.alignment = alignment;
        self.fill = fill;
        self.radix = radix;
        self.uppercase = uppercase;
        self.sign = sign;
        self.alternate = alternate;
        self.floatStyle = floatStyle;
        self.debug = debug;
    }

    /// Compares two format options for equality.
    public func equals(other: FormatOptions) -> Bool {
        self.width == other.width and
        self.precision == other.precision and
        self.alignment == other.alignment and
        self.fill == other.fill and
        self.radix == other.radix and
        self.uppercase == other.uppercase and
        self.sign == other.sign and
        self.alternate == other.alternate and
        self.floatStyle == other.floatStyle and
        self.debug == other.debug
    }
}

// ============================================================================
// STRING INTERPOLATION PROTOCOLS
// ============================================================================

/// Protocol for types that accumulate string interpolation parts.
///
/// Used by ExpressibleByStringInterpolation to build interpolated strings.
/// The compiler generates calls to appendLiteral and appendInterpolation
/// for each part of an interpolated string.
@builtin(.StringInterpolationProtocol)
public protocol StringInterpolationProtocol {
    /// Initialize with capacity hints for optimization.
    ///
    /// - Parameters:
    ///   - literalCapacity: Total length of literal string segments
    ///   - interpolationCount: Number of interpolation segments
    init(literalCapacity: Int64, interpolationCount: Int64)

    /// Append a literal string segment.
    mutating func appendLiteral(literal: String)

    /// Append a formatted value with options.
    mutating func appendInterpolation[T: Formattable](value: T, options: FormatOptions)
}

/// Protocol for types that can be created from string interpolation.
///
/// Refines ExpressibleByStringLiteral since any interpolation-capable type
/// should also handle plain strings.
///
/// Example:
/// ```
/// let greeting: String = "Hello, \(name)!"
/// let query: SQLQuery = "SELECT * FROM \(table)"
/// ```
@builtin(.ExpressibleByStringInterpolation)
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
///
/// Accumulates string parts into an array, then concatenates them.
public struct DefaultStringInterpolation: StringInterpolationProtocol {
    private var parts: Array[String]

    /// Initialize with capacity hints.
    public init(literalCapacity: Int64, interpolationCount: Int64) {
        self.parts = [];
    }

    /// Append a literal string segment.
    public mutating func appendLiteral(literal: String) {
        if !literal.isEmpty {
            self.parts.push(literal);
        }
    }

    /// Append a formatted value with options.
    public mutating func appendInterpolation[T: Formattable](value: T, options: FormatOptions = .default) {
        self.parts.push(value.format(options: options));
    }

    /// Build the final string by concatenating all parts.
    public func build() -> String {
        concat(self.parts)
    }
}

// ============================================================================
// STRING CONCATENATION
// ============================================================================

/// Concatenate an array of strings into a single string.
///
/// More efficient than repeated + operations as it calculates
/// the total length first and allocates once.
public func concat(parts: Array[String]) -> String {
    // Calculate total length
    var totalLength: Int64 = 0;
    for part in parts {
        totalLength = totalLength + part.length;
    }

    // Build result
    if totalLength == 0 {
        return ""
    }

    var result = String.withCapacity(totalLength);
    for part in parts {
        result.append(part);
    }
    result
}
