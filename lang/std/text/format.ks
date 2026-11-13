// Formatting - Types and protocols for text formatting and string interpolation.

module std.text

import std.core.(Bool, Equatable, Matchable, ExpressibleByStringLiteral)
import std.numeric.(Int64)
import std.text.(String, Char, StringSlice, StringBuilder)
import std.result.(Optional)
import std.collections.(Array)
import std.memory.(Allocator, GlobalAllocator)

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
/// "ab".formatted(opts);  // "      ab"
/// opts.alignment = .Center;
/// "ab".formatted(opts);  // "   ab   "
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
/// (3).formatted(opts);   // "+3"
/// (-3).formatted(opts);  // "-3"
/// opts.sign = .Space;
/// (3).formatted(opts);   // " 3"
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
/// (3.14159).formatted(opts);       // "3.14"
/// opts.floatStyle = .Scientific;
/// (3.14159).formatted(opts);       // "3.14e0"
/// opts.floatStyle = .Percent;
/// (0.5).formatted(opts);           // "50.00%"
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
/// String interpolation `"\(expr:spec)"` constructs one of these from the
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
/// "\(n:>8)";      // right-align, width 8
/// "\(n:08x)";     // zero-pad, width 8, hex
/// "\(n:#X)";      // hex upper with 0x prefix
/// "\(pi:.2)";     // precision 2 decimal places
/// "\(pi:.2e)";    // scientific with 2 decimal places
/// "\(ratio:%)";   // as percentage (0.5 -> "50%")
/// "\(name:^10)";  // center, width 10
/// "\(value:?)";   // debug format
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

    /// For floats: number of decimal places (an explicit precision prints `Auto` in fixed notation). For strings: maximum character count.
    public var precision: Int64?

    /// How to position the value inside `width` when padding is required.
    public var alignment: Alignment

    /// Padding character — defaults to `' '`. Only applies when `width` is set and the value is shorter.
    public var fill: Char

    /// Numeric base for integer formatting: 2 (binary), 8 (octal), 10 (decimal), 16 (hex).
    /// Floats honor only 16, printing hexadecimal floating point (`0x1.8p+1` for 3.0).
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
    /// (42).formatted(opts);  // "42"
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
    /// "hi".formatted(opts);  // "    hi"
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

/// The `FormatOptions` a `{value:spec}` format specifier spells.
///
/// The compiler parses each specifier and builds its options through this
/// function, so it never names `FormatOptions`' fields or cases itself.
/// `width` / `precision` apply only when their `has` flag is set.
/// `alignment`, `sign` and `floatStyle` arrive as codes: alignment 0 left,
/// 1 right, 2 center; sign 0 negative-only, 1 always, 2 space; float style
/// 0 auto, 1 fixed, 2 scientific, 3 scientific uppercase, 4 percent.
@builtin(.FormatSpecOptions)
public func _formatSpecOptions(hasWidth: Bool, width: Int64, hasPrecision: Bool, precision: Int64, alignment: Int64, fill: Char, radix: Int64, uppercase: Bool, sign: Int64, alternate: Bool, floatStyle: Int64, debug: Bool) -> FormatOptions {
    var opts = FormatOptions();
    if hasWidth { opts.width = .Some(width) }
    if hasPrecision { opts.precision = .Some(precision) }
    if alignment == 1 { opts.alignment = Alignment.Right }
    if alignment == 2 { opts.alignment = Alignment.Center }
    opts.fill = fill;
    opts.radix = radix;
    opts.uppercase = uppercase;
    if sign == 1 { opts.sign = Sign.Always }
    if sign == 2 { opts.sign = Sign.Space }
    opts.alternate = alternate;
    if floatStyle == 1 { opts.floatStyle = FloatStyle.Fixed }
    if floatStyle == 2 { opts.floatStyle = FloatStyle.Scientific }
    if floatStyle == 3 { opts.floatStyle = FloatStyle.ScientificUpper }
    if floatStyle == 4 { opts.floatStyle = FloatStyle.Percent }
    opts.debug = debug;
    opts
}

// ============================================================================
// FORMATTABLE PROTOCOL
// ============================================================================

/// Protocol for types that can render themselves as a `String` under a `FormatOptions`.
///
/// Print routines and string interpolation `"\(expr)"` and `"\(expr:spec)"`
/// both ultimately bottom out in `format`. Implementors should honour
/// every `FormatOptions` field that is meaningful for their domain
/// (alignment and width are universal; `radix` applies to integers, and to
/// floats only as `16`, hexadecimal floating point; `floatStyle` only to
/// floats) and silently ignore fields that aren't.
///
/// # Examples
///
/// ```
/// "\(name)";         // "Alice"          (default formatting)
/// "\(name:>10)";     // "     Alice"     (right-align, width 10)
/// "\(n:08x)";        // "0000002a"       (zero-pad, hex, width 8)
/// "\(pi:.2)";        // "3.14"           (precision 2)
/// "\(value:?)";      // debug representation
/// ```
/// Mutable destination for formatted text.
///
/// Formatters only need to accept string-like values and individual code
/// points. `StringBuilder` is the default implementation; streaming sinks can
/// implement this protocol without forcing an intermediate `String`.
public protocol Formatter {
    mutating func append(other: &StringSlice)
    mutating func append(char c: Char)
}

@builtin(.FormattableProtocol)
public protocol Formattable {
    /// Writes this value's formatted representation directly into `writer`.
    ///
    /// This is the kernel method — all formatting ultimately bottoms out
    /// here. The convenience `formatted(options:) -> String` in the protocol
    /// extension calls this under the hood.
    @builtin(.FormattableFormatInto)
    func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default())

    /// Returns this value rendered as a `String`.
    ///
    /// The default builds the string with `format(into:)`, so conformers
    /// rarely implement it. It is a requirement, not just an extension
    /// method, because `any Formattable` formats through it: unlike the
    /// generic `format(into:)`, it can sit in the existential's function
    /// table.
    func formatted(options: FormatOptions = FormatOptions.default()) -> String
}

extend Formattable {
    public func formatted(options: FormatOptions = FormatOptions.default()) -> String {
        var b = StringBuilder();
        self.format(into: b, options);
        b.build()
    }

    /// `any Formattable`'s `format(into:)`: renders the payload with its
    /// `formatted(options:)` and appends the result, so the options (width,
    /// padding, precision) still reach the payload. Concrete conformers
    /// implement `format(into:)` themselves.
    @existentialOnly
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        writer.append(self.formatted(options));
    }
}

// ============================================================================
// PADDING HELPERS
// ============================================================================

/// Padding helpers for `format(into:)` implementations.
///
/// A type's `format(into:)` receives the hole's whole `FormatOptions` and
/// decides what width means for it; nothing pads on its behalf. These do the
/// common cases: `pad` for text, `padIntegral` for numbers.
extend Formatter {
    /// Writes `content` padded to `options.width` with `options.fill`,
    /// placed by `options.alignment`. A `precision` truncates `content` to
    /// that many characters first, as for strings (`"\(s:.3)"`).
    ///
    /// Width and precision count code points, not bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// struct Pt: Formattable {
    ///     var x: Int64
    ///     func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
    ///         writer.pad("Pt(\(self.x))", options)
    ///     }
    /// }
    /// "[\(Pt(x: 7):>8)]";   // "[   Pt(7)]"
    /// ```
    public mutating func pad(content: &StringSlice, options: FormatOptions) {
        var count = content.chars.count;
        var truncated = false;
        if let .Some(precision) = options.precision {
            if precision < count {
                count = precision;
                truncated = true
            }
        }
        let (left, right) = paddingSplit(count, options);
        self.appendFill(left, options.fill);
        if truncated {
            var it = content.chars.iter();
            var i: Int64 = 0;
            while i < count {
                if let .Some(c) = it.next() { self.append(char: c) };
                i = i + 1
            }
        } else {
            self.append(content)
        }
        self.appendFill(right, options.fill)
    }

    /// Writes a number as its sign, then `prefix`, then `digits`, padded to
    /// `options.width`. The sign follows `options.sign` (`-` only when
    /// `nonNegative` is false, by default). A `'0'` fill puts every padding
    /// zero between the prefix and the digits, whatever the alignment:
    /// `-0x002a`, not `000-0x2a` or `-0x2a000`.
    ///
    /// The caller decides the prefix (`"0x"` only under `alternate`, say)
    /// and passes `""` for none.
    ///
    /// # Examples
    ///
    /// ```
    /// var options = FormatOptions();
    /// options.width = .Some(4);
    /// options.fill = '0';
    /// var b = StringBuilder();
    /// b.padIntegral(nonNegative: false, prefix: "", digits: "5", options);
    /// b.build();   // "-005"
    /// ```
    public mutating func padIntegral(nonNegative nonNegative: Bool, prefix prefix: &StringSlice, digits digits: &StringSlice, options: FormatOptions) {
        let sign: Char? = if nonNegative == false {
            .Some('-')
        } else if options.sign == .Always {
            .Some('+')
        } else if options.sign == .Space {
            .Some(' ')
        } else {
            .None
        };
        let signCount: Int64 = if let .Some(_) = sign { 1 } else { 0 };
        let count = signCount + prefix.chars.count + digits.chars.count;
        let (before, after) = paddingSplit(count, options);
        // Zero fill ignores alignment, as in Rust: `-5` is `-005`, never `-500`.
        let zeroFill = options.fill == '0';
        let left = if zeroFill { before + after } else { before };
        let right = if zeroFill { 0 } else { after };
        if zeroFill == false { self.appendFill(left, options.fill) };
        if let .Some(c) = sign { self.append(char: c) };
        self.append(prefix);
        if zeroFill { self.appendFill(left, options.fill) };
        self.append(digits);
        self.appendFill(right, options.fill)
    }

    /// Appends `count` copies of `fill`.
    private mutating func appendFill(count: Int64, fill: Char) {
        var n = count;
        while n > 0 {
            self.append(char: fill);
            n = n - 1
        }
    }
}

/// The fill counts before and after content `count` characters wide.
func paddingSplit(count: Int64, options: FormatOptions) -> (Int64, Int64) {
    guard let .Some(width) = options.width else { return (0, 0) }
    if width <= count { return (0, 0) }
    let padding = width - count;
    match options.alignment {
        .Left => (0, padding),
        .Right => (padding, 0),
        .Center => (padding / 2, padding - padding / 2)
    }
}

// ============================================================================
// STRING INTERPOLATION PROTOCOLS
// ============================================================================

/// Protocol for the accumulator type that string interpolation builds into.
///
/// The compiler lowers `"hello, \(name)!"` to a fresh accumulator, an
/// `appendLiteral` call per static segment and an `appendInterpolation` call
/// per hole, then hands the accumulator to the result type's
/// `init(interpolation:)`. `String` ships `DefaultStringInterpolation` as its
/// accumulator; custom string-like types supply their own.
///
/// `appendInterpolation` is deliberately NOT a requirement: each accumulator
/// declares its own overloads, choosing which values a hole accepts (`some
/// Formattable`, only `Bindable` values for SQL, …) and whether it takes
/// format options. Holes are ordinary method calls on the accumulator.
public protocol Interpolatable {
    /// @name With Capacity
    /// Constructs an empty accumulator with capacity hints derived from the literal at compile time.
    ///
    /// `literalCapacity` is the total byte count of the static segments;
    /// `interpolationCount` is the number of `\(...)` holes. Implementors
    /// can use these to preallocate.
    @sugarOnly
    @builtin(.InterpolatableInit)
    init(literalCapacity literalCapacity: Int64, interpolationCount interpolationCount: Int64)

    /// Appends a static literal segment.
    ///
    /// Called once per run of literal text between `\(...)` holes. May be
    /// called with the empty string; implementors should be cheap in
    /// that case.
    @builtin(.InterpolatableAppendLiteral)
    mutating func appendLiteral(literal: String)

}

/// Marker protocol for types constructible from a completed string interpolation.
///
/// Refines `ExpressibleByStringLiteral` so a single conformance covers
/// both pure-literal `"abc"` and interpolated `"a\(x)b"` forms. The
/// compiler picks `Interpolation` as the accumulator type, drives it via
/// `Interpolatable`, then hands it to `init(interpolation:)`.
@builtin(.ExpressibleByStringInterpolation)
public protocol ExpressibleByStringInterpolation: ExpressibleByStringLiteral {
    /// The accumulator type used to build interpolated values of `Self`.
    type Interpolation: Interpolatable

    /// @name From Interpolation
    /// Constructs `Self` from a fully built interpolation accumulator.
    ///
    /// Takes the accumulator by value, so an implementation can move its
    /// buffer out without copying (accumulators need not be `Cloneable`).
    @sugarOnly
    @builtin(.StringInterpolationInit)
    init(consuming interpolation: Interpolation)
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
    public mutating func appendLiteral(literal: String) {
        if literal.isEmpty == false {
            self.builder.append(literal);
        }
    }

    /// Formats one interpolation hole directly into the buffer.
    public mutating func appendInterpolation(value: &some Formattable and not Static and not Sized, options: FormatOptions = FormatOptions.default()) {
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
    public mutating func build() -> String {
        self.builder.build()
    }
}

/// `String` interpolates through `DefaultStringInterpolation`.
// Default strings only: the accumulator's buffer comes from the global
// allocator, so another `A` would need a copy.
extend String[GlobalAllocator]: ExpressibleByStringInterpolation {
    /// The accumulator `String` interpolation builds into.
    public type Interpolation = DefaultStringInterpolation

    /// @name From Interpolation
    /// Takes over the accumulator's buffer without copying.
    public init(consuming interpolation: DefaultStringInterpolation) {
        var accumulator = interpolation;
        self = accumulator.build()
    }
}
