// Formatting - Types and protocols for text formatting and string interpolation.

module std.text

import std.core.(Bool, Equatable, Matchable, ExpressibleByStringLiteral)
import std.numeric.(Int64)
import std.text.(String, Char, StringBuilder)
import std.result.(Optional)
import std.collections.(Array)

// ============================================================================
// FORMAT ENUMS
// ============================================================================

/// Horizontal alignment of formatted output within a fixed field width.
///
/// Pairs with `FormatOptions.width` and `FormatOptions.fill` to position
/// shorter values inside the requested column. When the value is already at
/// least as wide as the field, alignment has no visible effect. The
/// formatter for `String` is the canonical consumer; numeric and boolean
/// formatters honour the same convention.
///
/// # Examples
///
/// ```
/// var opts = FormatOptions();
/// opts.width = .Some(8);
/// opts.alignment = .Right;
/// "ab".format(options: opts);  // "      ab"
/// opts.alignment = .Center;
/// "ab".format(options: opts);  // "   ab   "
/// ```
public enum Alignment: Equatable, Matchable {
    /// Pad on the right; the value sits flush against the left edge of the field.
    case Left
    /// Pad on the left; the value sits flush against the right edge of the field.
    case Right
    /// Pad on both sides; if the padding is odd, the extra space goes on the right.
    case Center

    /// Returns true if both cases are the same variant.
    ///
    /// Equality is structural — there are no payloads. Used by the
    /// `Equatable` conformance so `FormatOptions.isEqual` can fall through
    /// without payload comparisons.
    ///
    /// # Examples
    ///
    /// ```
    /// Alignment.Left.isEqual(to: .Left);    // true
    /// Alignment.Left.isEqual(to: .Center);  // false
    /// ```
    public func isEqual(to other: Alignment) -> Bool {
        match (self, other) {
            (.Left, .Left) => true,
            (.Right, .Right) => true,
            (.Center, .Center) => true,
            _ => false
        }
    }

    /// Pattern-match form of equality — delegates to `isEqual`.
    ///
    /// Lets `Alignment` appear in `match` patterns against another value.
    ///
    /// # Examples
    ///
    /// ```
    /// Alignment.Right.matches(.Right);  // true
    /// ```
    public func matches(other: Alignment) -> Bool {
        self.isEqual(to: other)
    }
}

/// How the sign of a numeric value should be rendered.
///
/// Read by integer and float formatters before emitting the magnitude.
/// `Negative` is the conventional default — only `-` for negative values,
/// nothing for non-negatives. `Always` is useful for diffs or coordinates
/// where every value should carry an explicit sign; `Space` reserves a
/// column so columns of mixed signs line up.
///
/// # Examples
///
/// ```
/// var opts = FormatOptions();
/// opts.sign = .Always;
/// (3).format(options: opts);   // "+3"
/// (-3).format(options: opts);  // "-3"
/// opts.sign = .Space;
/// (3).format(options: opts);   // " 3"
/// ```
public enum Sign: Equatable, Matchable {
    /// Show `-` for negative values, no prefix for zero or positive (default).
    case Negative
    /// Always show a sign — `+` for non-negative, `-` for negative.
    case Always
    /// Use a leading space for non-negative, `-` for negative; keeps mixed-sign columns aligned.
    case Space

    /// Returns true if both cases are the same variant.
    ///
    /// Used by `Equatable` to lift case identity into a `Bool` for
    /// composite comparisons (see `FormatOptions.isEqual`).
    ///
    /// # Examples
    ///
    /// ```
    /// Sign.Always.isEqual(to: .Always);     // true
    /// Sign.Negative.isEqual(to: .Always);   // false
    /// ```
    public func isEqual(to other: Sign) -> Bool {
        match (self, other) {
            (.Negative, .Negative) => true,
            (.Always, .Always) => true,
            (.Space, .Space) => true,
            _ => false
        }
    }

    /// Pattern-match form of equality — delegates to `isEqual`.
    ///
    /// # Examples
    ///
    /// ```
    /// Sign.Space.matches(.Space);  // true
    /// ```
    public func matches(other: Sign) -> Bool {
        self.isEqual(to: other)
    }
}

/// How a floating-point value should be rendered.
///
/// Selected by the `:f` / `:e` / `:E` / `:%` type slots in the format
/// mini-language and read by the `Float32` / `Float64` formatters. Choice
/// of style is independent of `precision` — `Auto` honours precision as
/// "max significant digits", `Fixed` and `Scientific` treat it as
/// "decimal places". The non-`Auto` variants always emit a decimal point.
///
/// # Examples
///
/// ```
/// var opts = FormatOptions();
/// opts.precision = .Some(2);
/// opts.floatStyle = .Fixed;
/// (3.14159).format(options: opts);       // "3.14"
/// opts.floatStyle = .Scientific;
/// (3.14159).format(options: opts);       // "3.14e0"
/// opts.floatStyle = .Percent;
/// (0.5).format(options: opts);           // "50.00%"
/// ```
public enum FloatStyle: Equatable, Matchable {
    /// Shortest round-trippable representation; switches to scientific for very large or very small magnitudes.
    case Auto
    /// Fixed-point — `precision` controls decimal places.
    case Fixed
    /// Scientific notation with lowercase `e` exponent marker.
    case Scientific
    /// Scientific notation with uppercase `E` exponent marker.
    case ScientificUpper
    /// Multiplies by 100 and appends `%`.
    case Percent

    /// Returns true if both cases are the same variant.
    ///
    /// All cases are payload-less, so equality is purely structural.
    ///
    /// # Examples
    ///
    /// ```
    /// FloatStyle.Fixed.isEqual(to: .Fixed);       // true
    /// FloatStyle.Fixed.isEqual(to: .Scientific);  // false
    /// ```
    public func isEqual(to other: FloatStyle) -> Bool {
        match (self, other) {
            (.Auto, .Auto) => true,
            (.Fixed, .Fixed) => true,
            (.Scientific, .Scientific) => true,
            (.ScientificUpper, .ScientificUpper) => true,
            (.Percent, .Percent) => true,
            _ => false
        }
    }

    /// Pattern-match form of equality — delegates to `isEqual`.
    ///
    /// # Examples
    ///
    /// ```
    /// FloatStyle.Auto.matches(.Auto);  // true
    /// ```
    public func matches(other: FloatStyle) -> Bool {
        self.isEqual(to: other)
    }
}

// ============================================================================
// FORMAT OPTIONS
// ============================================================================

/// Mutable bag of formatting knobs threaded through every `Formattable.format` call.
///
/// `FormatOptions` is the parsed form of the format-spec mini-language.
/// String interpolation `"\{expr:spec}"` constructs one of these from the
/// trailing spec, then hands it to the formatter for `expr`'s type. Each
/// formatter reads only the fields that apply to it: integers ignore
/// `floatStyle`, strings ignore `radix`, and so on.
///
/// # Format spec mini-language
///
/// `[[fill]align][sign][#][0][width][.precision][type]`
///
/// Where `type` is one of:
///   - Integers: `b` (binary), `o` (octal), `x` (hex lower), `X` (hex upper)
///   - Floats:   `f` (fixed), `e` (scientific), `E` (scientific upper), `%` (percent)
///   - Any:      `?` (debug)
///
/// # Examples
///
/// ```
/// "\{n:>8}";      // right-align, width 8
/// "\{n:08x}";     // zero-pad, width 8, hex
/// "\{n:#X}";      // hex upper with 0x prefix
/// "\{pi:.2}";     // precision 2 decimal places
/// "\{pi:.2e}";    // scientific with 2 decimal places
/// "\{ratio:%}";   // as percentage (0.5 -> "50%")
/// "\{name:^10}";  // center, width 10
/// "\{value:?}";   // debug format
/// ```
///
/// # Representation
///
/// A flat record of independent fields — no validation across them. Each
/// formatter is responsible for ignoring fields outside its domain and
/// applying its own defaults when an option is absent.
@builtin(.FormatOptions)
public struct FormatOptions: Equatable {
    /// Minimum field width in characters; shorter values are padded with `fill` according to `alignment`.
    public var width: Int64?

    /// For floats: number of decimal places (or significant digits in `Auto` mode). For strings: maximum character count.
    public var precision: Int64?

    /// How to position the value inside `width` when padding is required.
    public var alignment: Alignment

    /// Padding character — defaults to `' '`. Only applies when `width` is set and the value is shorter.
    public var fill: Char

    /// Numeric base for integer formatting: 2 (binary), 8 (octal), 10 (decimal), 16 (hex).
    public var radix: Int64

    /// When `true`, integer hex digits are emitted as `A`–`F` rather than `a`–`f`.
    public var uppercase: Bool

    /// Sign-display strategy for numeric formatters.
    public var sign: Sign

    /// Alternate form: emit the conventional radix prefix (`0b`, `0o`, `0x`/`0X`) for non-decimal integers.
    public var alternate: Bool

    /// Float rendering style (fixed, scientific, percent, auto).
    public var floatStyle: FloatStyle

    /// When `true`, formatters should produce a structural / debug representation rather than a user-facing one.
    public var debug: Bool

    /// Returns a fresh `FormatOptions` with all fields at their default values.
    ///
    /// Equivalent to calling `FormatOptions()`; provided as a static so
    /// callers that want defaults without spelling out the constructor
    /// (e.g. default-arg expressions) have a clean call site.
    ///
    /// # Examples
    ///
    /// ```
    /// let opts = FormatOptions.default();
    /// (42).format(options: opts);  // "42"
    /// ```
    public static func default() -> FormatOptions {
        FormatOptions()
    }

    /// @name Default
    /// Creates a `FormatOptions` with every field at its default value.
    ///
    /// Defaults: no `width` or `precision`, left alignment, space fill,
    /// decimal radix, lowercase hex, negative-only sign, no alternate form,
    /// `Auto` float style, debug off.
    ///
    /// # Examples
    ///
    /// ```
    /// var opts = FormatOptions();
    /// opts.width = .Some(6);
    /// opts.alignment = .Right;
    /// "hi".format(options: opts);  // "    hi"
    /// ```
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

    /// Returns true if all fields are equal between the two options.
    ///
    /// `width` and `precision` are not compared — they typically reflect
    /// per-call overrides rather than logical identity. Compare them
    /// explicitly if needed.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = FormatOptions();
    /// let b = FormatOptions();
    /// a.isEqual(to: b);  // true
    /// var c = FormatOptions();
    /// c.alternate = true;
    /// a.isEqual(to: c);  // false
    /// ```
    public func isEqual(to other: FormatOptions) -> Bool {
        if self.alignment.isEqual(to: other.alignment) == false { return false }
        if self.radix != other.radix { return false }
        if self.uppercase != other.uppercase { return false }
        if self.sign.isEqual(to: other.sign) == false { return false }
        if self.alternate != other.alternate { return false }
        if self.floatStyle.isEqual(to: other.floatStyle) == false { return false }
        if self.debug != other.debug { return false }
        if self.fill.isEqual(to: other.fill) == false { return false }
        if self.width.isEqual(to: other.width) == false { return false }
        if self.precision.isEqual(to: other.precision) == false { return false }
        true
    }
}

// ============================================================================
// FORMATTABLE PROTOCOL
// ============================================================================

/// Protocol for types that can render themselves as a `String` under a `FormatOptions`.
///
/// Print routines and string interpolation `"\{expr}"` and `"\{expr:spec}"`
/// both ultimately bottom out in `format`. Implementors should honour
/// every `FormatOptions` field that is meaningful for their domain
/// (alignment and width are universal; `radix` only applies to integers,
/// `floatStyle` only to floats) and silently ignore fields that aren't.
///
/// # Examples
///
/// ```
/// "\{name}";         // "Alice"          (default formatting)
/// "\{name:>10}";     // "     Alice"     (right-align, width 10)
/// "\{n:08x}";        // "0000002a"       (zero-pad, hex, width 8)
/// "\{pi:.2}";        // "3.14"           (precision 2)
/// "\{value:?}";      // debug representation
/// ```
@builtin(.FormattableProtocol)
public protocol Formattable {
    /// Writes this value's formatted representation directly into `writer`.
    ///
    /// This is the kernel method — all formatting ultimately bottoms out
    /// here. The convenience `format(options:) -> String` in the protocol
    /// extension calls this under the hood.
    @builtin(.FormattableFormatInto)
    func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default())
}

extend Formattable {
    /// Returns this value rendered as a `String`.
    ///
    /// Convenience wrapper: creates a `StringBuilder`, calls
    /// `format(into:)`, and returns the built string. Uses a distinct
    /// name to avoid overload-resolution ambiguity with `format(into:)`.
    public func formatted(options: FormatOptions = FormatOptions.default()) -> String {
        var b = StringBuilder();
        self.format(into: b, options);
        b.build()
    }
}

// ============================================================================
// PADDING HELPER
// ============================================================================

/// Writes `content` into `writer` with width/alignment/fill padding applied.
/// Used by String, integer, and float `format(into:)` implementations.
public func _writePadded(mutating into writer: StringBuilder, content: String, options: FormatOptions) {
    if let .Some(width) = options.width {
        let currentLen = content.chars.count;
        if width > currentLen {
            let padding = width - currentLen;
            var padLeft: Int64 = 0;
            var padRight: Int64 = 0;

            if options.alignment == .Left {
                padRight = padding
            } else if options.alignment == .Right {
                padLeft = padding
            } else {
                padLeft = padding / 2;
                padRight = padding - padLeft
            }

            while padLeft > 0 {
                writer.append(char: options.fill);
                padLeft = padLeft - 1
            }
            writer.append(content);
            while padRight > 0 {
                writer.append(char: options.fill);
                padRight = padRight - 1
            }
            return
        }
    }
    writer.append(content)
}

// ============================================================================
// STRING INTERPOLATION PROTOCOLS
// ============================================================================

/// Protocol for the accumulator type that string interpolation builds into.
///
/// The compiler lowers `"hello, \{name}!"` to a sequence of
/// `appendLiteral` and `appendInterpolation` calls on a fresh value of
/// the implementor's type, then reads the final string out (typically
/// via a `build()` method on the concrete accumulator). `String` ships
/// `DefaultStringInterpolation` as its accumulator; custom string-like
/// types can supply their own to intercept literal pieces or coerce
/// formatted parts.
public protocol Interpolatable {
    /// @name With Capacity
    /// Constructs an empty accumulator with capacity hints derived from the literal at compile time.
    ///
    /// `literalCapacity` is the total byte count of the static segments;
    /// `interpolationCount` is the number of `\{...}` holes. Implementors
    /// can use these to preallocate.
    init(literalCapacity literalCapacity: Int64, interpolationCount interpolationCount: Int64)

    /// Appends a static literal segment.
    ///
    /// Called once per run of literal text between `\{...}` holes. May be
    /// called with the empty string; implementors should be cheap in
    /// that case.
    mutating func appendLiteral(literal: String)

    /// Appends one formatted interpolation hole.
    ///
    /// Receives the runtime `value`, the parsed `options` from the
    /// trailing spec (or defaults if no spec was given), and a generic
    /// constraint that the value is `Formattable`.
    mutating func appendInterpolation(value: some Formattable, options: FormatOptions)
}

/// Marker protocol for types constructible from a completed string interpolation.
///
/// Refines `ExpressibleByStringLiteral` so a single conformance covers
/// both pure-literal `"abc"` and interpolated `"a\{x}b"` forms. The
/// compiler picks `Interpolation` as the accumulator type, drives it via
/// `Interpolatable`, then hands it to `init(interpolation:)`.
public protocol ExpressibleByStringInterpolation: ExpressibleByStringLiteral {
    /// The accumulator type used to build interpolated values of `Self`.
    type Interpolation: Interpolatable

    /// @name From Interpolation
    /// Constructs `Self` from a fully built interpolation accumulator.
    init(interpolation: Interpolation)
}

// ============================================================================
// DEFAULT STRING INTERPOLATION
// ============================================================================

/// The default `Interpolatable` accumulator used for `String` interpolation.
///
/// Stores each literal and each formatted interpolation as a separate
/// `String` part, then concatenates them in `build()`. The two-pass
/// design lets `build()` size the result buffer exactly, avoiding the
/// repeated reallocation cost a single-buffer accumulator would pay.
///
/// # Examples
///
/// ```
/// var acc = DefaultStringInterpolation(literalCapacity: 7, interpolationCount: 1);
/// acc.appendLiteral("hello, ");
/// acc.appendInterpolation("world", options: FormatOptions.default());
/// acc.build();  // "hello, world"
/// ```
///
/// # Representation
///
/// A single `StringBuilder` that accumulates all literal and formatted
/// bytes in one buffer. Pre-sized using the compiler's capacity hints.
@builtin(.DefaultStringInterpolation)
public struct DefaultStringInterpolation: Interpolatable, Cloneable {
    private var builder: StringBuilder

    /// @name With Capacity
    /// Constructs an empty accumulator pre-sized from compile-time hints.
    ///
    /// `literalCapacity` is the exact byte count of static segments;
    /// `interpolationCount` estimates ~16 bytes per hole.
    ///
    /// # Examples
    ///
    /// ```
    /// var acc = DefaultStringInterpolation(literalCapacity: 0, interpolationCount: 0);
    /// acc.build();  // ""
    /// ```
    @builtin(.DefaultStringInterpolationInit)
    public init(literalCapacity literalCapacity: Int64, interpolationCount interpolationCount: Int64) {
        self.builder = StringBuilder(capacity: literalCapacity + interpolationCount * 16);
    }

    /// Returns a copy with a cloned builder buffer.
    public func clone() -> DefaultStringInterpolation {
        var c = DefaultStringInterpolation(literalCapacity: 0, interpolationCount: 0);
        c.builder = self.builder.clone();
        c
    }

    /// Appends a static literal segment directly into the buffer.
    @builtin(.DefaultStringInterpolationAppendLiteral)
    public mutating func appendLiteral(literal: String) {
        if literal.isEmpty == false {
            self.builder.append(literal);
        }
    }

    /// Formats one interpolation hole directly into the buffer.
    @builtin(.DefaultStringInterpolationAppendInterpolation)
    public mutating func appendInterpolation(value: some Formattable, options: FormatOptions = FormatOptions.default()) {
        value.format(into: self.builder, options);
    }

    /// Transfers the buffer into a `String` without copying.
    ///
    /// # Examples
    ///
    /// ```
    /// var acc = DefaultStringInterpolation(literalCapacity: 0, interpolationCount: 0);
    /// acc.appendLiteral("a");
    /// acc.appendLiteral("b");
    /// acc.build();  // "ab"
    /// ```
    @builtin(.DefaultStringInterpolationBuild)
    public mutating func build() -> String {
        self.builder.build()
    }
}
