// Duration: an exact, signed span of time with nanosecond precision. Shared
// by every std timeout and by the datetime package.

module std.time

import std.numeric.(Int64, Float64)
import std.core.(Bool, Equatable, Comparable, Ordering, Hashable, Hasher, Cloneable, Addable, Subtractable, Multipliable, Divisible, Negatable)
import std.text.(String, StringBuilder, Formatter, Formattable, FormatOptions)
import std.result.(Optional)

/// An exact span of time: signed, with nanosecond precision.
///
/// Build one with a unit, `Duration.milliseconds(500)` or `.seconds(5)`
/// where a `Duration` is expected, and combine them with `+`, `-`, `*` and
/// `/`. Prints in ISO 8601 form (`PT1.5S`); `humanString()` gives `1.5s`.
///
/// Internal: secs + nanos with uniform sign (both positive or both negative).
public struct Duration: Equatable, Comparable, Hashable, Formattable, Cloneable {
    var secs: Int64
    var nanos: Int64

    internal static var NANOS_PER_SEC: Int64 { 1_000_000_000 }

    // Normalize so |nanos| < 1 billion and signs agree
    internal static func normalized(secs secs: Int64, nanos nanos: Int64) -> Duration {
        var s = secs + nanos / Duration.NANOS_PER_SEC;
        var n = nanos % Duration.NANOS_PER_SEC;
        // Fix sign disagreement: secs and nanos must have the same sign
        if s > 0 and n < 0 {
            s = s - 1;
            n = n + Duration.NANOS_PER_SEC;
        } else if s < 0 and n > 0 {
            s = s + 1;
            n = n - Duration.NANOS_PER_SEC;
        }
        var d = Duration(raw: 0, rawNanos: 0);
        d.secs = s;
        d.nanos = n;
        d
    }

    // Raw init (no normalization)
    internal init(raw secs: Int64, rawNanos nanos: Int64) {
        self.secs = secs;
        self.nanos = nanos;
    }

    // --- Construction ---

    public init(seconds seconds: Int64, nanoseconds nanoseconds: Int64 = 0) {
        let d = Duration.normalized(secs: seconds, nanos: nanoseconds);
        self.secs = d.secs;
        self.nanos = d.nanos;
    }

    public static func nanoseconds(n: Int64) -> Duration {
        Duration.normalized(secs: 0, nanos: n)
    }

    public static func microseconds(n: Int64) -> Duration {
        Duration.normalized(secs: 0, nanos: n * 1000)
    }

    public static func milliseconds(n: Int64) -> Duration {
        Duration.normalized(secs: 0, nanos: n * 1_000_000)
    }

    public static func seconds(n: Int64) -> Duration {
        Duration(raw: n, rawNanos: 0)
    }

    public static func minutes(n: Int64) -> Duration {
        Duration(raw: n * 60, rawNanos: 0)
    }

    public static func hours(n: Int64) -> Duration {
        Duration(raw: n * 3600, rawNanos: 0)
    }

    // Exact 24-hour days. For calendar days (DST-aware) use Period(days:).
    public static func days(n: Int64) -> Duration {
        Duration(raw: n * 86400, rawNanos: 0)
    }

    // From a fractional number of seconds, rounded to the nearest nanosecond.
    // Fails with `nil` for NaN, infinity, or values outside Int64 seconds.
    public init(fractionalSeconds seconds: Float64)? {
        guard let .Some(whole) = seconds.toInt64() else { return null; }
        let fraction = (seconds - Float64(from: whole)) * 1_000_000_000.0;
        guard let .Some(nanos) = fraction.round().toInt64() else { return null; }
        let d = Duration.normalized(secs: whole, nanos: nanos);
        self.secs = d.secs;
        self.nanos = d.nanos;
    }

    public static var zero: Duration { Duration(raw: 0, rawNanos: 0) }

    // --- Properties ---

    // Accessors treat a Duration as a signed magnitude: whole units truncate
    // toward zero and the remainder keeps the sign (-1.5s = -1s and -500ms).
    // Contrast Instant, a point on a line, whose seconds floor.

    public var totalSeconds: Int64 { self.secs }

    public var totalMilliseconds: Int64 {
        self.secs * 1000 + self.nanos / 1_000_000
    }

    public var totalMicroseconds: Int64 {
        self.secs * 1_000_000 + self.nanos / 1000
    }

    public var totalNanoseconds: Int64 {
        self.secs * Duration.NANOS_PER_SEC + self.nanos
    }

    // Nanoseconds past `totalSeconds`; same sign as the duration.
    public var subsecondNanoseconds: Int64 { self.nanos }

    // Seconds as a Float64 (loses precision beyond ~104 days at ns resolution).
    public var fractionalSeconds: Float64 {
        Float64(from: self.secs) + Float64(from: self.nanos) / 1_000_000_000.0
    }

    public var isNegative: Bool { self.secs < 0 or (self.secs == 0 and self.nanos < 0) }

    public var isZero: Bool { self.secs == 0 and self.nanos == 0 }

    // --- Arithmetic ---

    public func negated() -> Duration {
        Duration(raw: 0 - self.secs, rawNanos: 0 - self.nanos)
    }

    public func abs() -> Duration {
        if self.isNegative { self.negated() } else { self }
    }

    // Sums and differences are the `+` / `-` operators (operators.ks).

    public func multiplied(by factor: Int64) -> Duration {
        Duration.normalized(secs: self.secs * factor, nanos: self.nanos * factor)
    }

    public func divided(by divisor: Int64) -> Duration {
        let totalNs = self.totalNanoseconds / divisor;
        Duration.nanoseconds(totalNs)
    }

    // --- String representations ---

    public func isoString() -> String {
        let a = self.abs();
        let h = a.secs / 3600;
        let m = (a.secs % 3600) / 60;
        let s = a.secs % 60;
        var b = StringBuilder();
        if self.isNegative { b.append("-"); }
        b.append("PT");
        if h > 0 { b.append("\(h)H"); }
        if m > 0 { b.append("\(m)M"); }
        if s > 0 or (h == 0 and m == 0 and a.nanos == 0) {
            b.append("\(s)");
            if a.nanos > 0 {
                b.append(".");
                appendDurationFraction(into: b, a.nanos);
            }
            b.append("S");
        } else if a.nanos > 0 {
            b.append("0.");
            appendDurationFraction(into: b, a.nanos);
            b.append("S");
        }
        b.build()
    }

    // Compact human form: "1d 2h 3m 4.5s", "-1.5s", "0s". Days are 24 hours.
    public func humanString() -> String {
        let a = self.abs();
        let d = a.secs / 86400;
        let h = (a.secs % 86400) / 3600;
        let m = (a.secs % 3600) / 60;
        let s = a.secs % 60;
        var b = StringBuilder();
        if self.isNegative { b.append("-"); }
        var wrote = false;
        if d > 0 { b.append("\(d)d"); wrote = true; }
        if h > 0 {
            if wrote { b.append(" "); }
            b.append("\(h)h");
            wrote = true;
        }
        if m > 0 {
            if wrote { b.append(" "); }
            b.append("\(m)m");
            wrote = true;
        }
        if s > 0 or a.nanos > 0 or not wrote {
            if wrote { b.append(" "); }
            b.append("\(s)");
            if a.nanos > 0 {
                b.append(".");
                appendDurationFraction(into: b, a.nanos);
            }
            b.append("s");
        }
        b.build()
    }

    // --- Protocol conformances ---

    public func isEqual(to other: Duration) -> Bool {
        self.secs == other.secs and self.nanos == other.nanos
    }

    public func compare(other: Duration) -> Ordering {
        self.secs.compare(other.secs).then(self.nanos.compare(other.nanos))
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.secs.hash(into: hasher);
        self.nanos.hash(into: hasher);
    }

    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        writer.append(self.isoString());
    }

    public func clone() -> Duration {
        Duration(raw: self.secs, rawNanos: self.nanos)
    }
}



/// Appends `nanos` (0 < nanos < 1e9) as a fraction of a second, without
/// trailing zeros: 500_000_000 is "5", 1_000 is "000001".
func appendDurationFraction(mutating into b: StringBuilder, nanos: Int64) {
    var n = nanos;
    var digits: Int64 = 9;
    while digits > 0 and n % 10 == 0 {
        n = n / 10;
        digits = digits - 1;
    }
    var temp = n;
    var digitCount: Int64 = 0;
    while temp > 0 {
        temp = temp / 10;
        digitCount = digitCount + 1;
    }
    while digitCount < digits {
        b.append("0");
        digitCount = digitCount + 1;
    }
    b.append("\(n)");
}

// Duration + Duration
extend Duration: Addable[Duration] {
    type Output = Duration
    public static var zero: Duration { Duration.zero }

    public consuming func add(consuming other: Duration) -> Duration {
        Duration.normalized(secs: self.secs + other.secs, nanos: self.nanos + other.nanos)
    }
}

// Duration - Duration
extend Duration: Subtractable[Duration] {
    type Output = Duration

    public consuming func subtract(consuming other: Duration) -> Duration {
        Duration.normalized(secs: self.secs - other.secs, nanos: self.nanos - other.nanos)
    }
}

// Duration * Int64
extend Duration: Multipliable[Int64] {
    type Output = Duration
    public static var one: Duration { Duration.seconds(1) }

    public consuming func multiply(consuming other: Int64) -> Duration {
        self.multiplied(by: other)
    }
}

// Duration / Int64
extend Duration: Divisible[Int64] {
    type Output = Duration

    public consuming func divide(consuming other: Int64) -> Duration {
        self.divided(by: other)
    }
}

// -Duration
extend Duration: Negatable {
    type Output = Duration

    public consuming func negate() -> Duration {
        self.negated()
    }
}

// Duration / Duration -> how many whole `other`s fit (truncating)
extend Duration: Divisible[Duration] {
    type Output = Int64

    public consuming func divide(consuming other: Duration) -> Int64 {
        self.totalNanoseconds / other.totalNanoseconds
    }
}

// No `Int64 * Duration`: a second Multipliable conformance on Int64 makes the
// solver pick it for plain `Int64 * Int64` across std. Write `d * n`.
