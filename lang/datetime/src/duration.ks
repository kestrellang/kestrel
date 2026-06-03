module datetime

// Exact elapsed time. Signed. Nanosecond precision.
// Internal: secs + nanos with uniform sign (both positive or both negative).
public struct Duration: Equatable, Comparable, Hashable, Formattable, Cloneable {
    var secs: Int64
    var nanos: Int64

    static var NANOS_PER_SEC: Int64 { 1_000_000_000 }

    // Normalize so |nanos| < 1 billion and signs agree
    static func normalized(secs secs: Int64, nanos nanos: Int64) -> Duration {
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

    // Private raw init (no normalization)
    init(raw secs: Int64, rawNanos nanos: Int64) {
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

    public static var zero: Duration { Duration(raw: 0, rawNanos: 0) }

    // --- Properties ---

    public var totalSeconds: Int64 { self.secs }

    public var totalMilliseconds: Int64 {
        self.secs * 1000 + self.nanos / 1_000_000
    }

    public var totalNanoseconds: Int64 {
        self.secs * Duration.NANOS_PER_SEC + self.nanos
    }

    public var subsecondNanoseconds: Int64 { self.nanos }

    public var isNegative: Bool { self.secs < 0 or (self.secs == 0 and self.nanos < 0) }

    public var isZero: Bool { self.secs == 0 and self.nanos == 0 }

    // --- Arithmetic ---

    public func negated() -> Duration {
        Duration(raw: 0 - self.secs, rawNanos: 0 - self.nanos)
    }

    public func abs() -> Duration {
        if self.isNegative { self.negated() } else { self }
    }

    public func adding(other: Duration) -> Duration {
        Duration.normalized(secs: self.secs + other.secs, nanos: self.nanos + other.nanos)
    }

    public func subtracting(other: Duration) -> Duration {
        Duration.normalized(secs: self.secs - other.secs, nanos: self.nanos - other.nanos)
    }

    public func multiplied(by factor: Int64) -> Duration {
        Duration.normalized(secs: self.secs * factor, nanos: self.nanos * factor)
    }

    public func divided(by divisor: Int64) -> Duration {
        let totalNs = self.totalNanoseconds / divisor;
        Duration.nanoseconds(totalNs)
    }

    // --- Rounding ---

    public func rounded(to unit: TimeUnit, mode mode: RoundMode = .HalfExpand) -> Duration {
        let divisor = unit.nanoseconds();
        if divisor <= 1 { return self }
        let total = self.totalNanoseconds;
        let rounded = roundValue(total, divisor, mode);
        Duration.nanoseconds(rounded)
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
                appendFractional(into: b, a.nanos);
            }
            b.append("S");
        } else if a.nanos > 0 {
            b.append("0.");
            appendFractional(into: b, a.nanos);
            b.append("S");
        }
        b.build()
    }

    public func humanString() -> String {
        let a = self.abs();
        let h = a.secs / 3600;
        let m = (a.secs % 3600) / 60;
        let s = a.secs % 60;
        var b = StringBuilder();
        if self.isNegative { b.append("-"); }
        var wrote = false;
        if h > 0 { b.append("\(h)h"); wrote = true; }
        if m > 0 {
            if wrote { b.append(" "); }
            b.append("\(m)m");
            wrote = true;
        }
        if s > 0 or not wrote {
            if wrote { b.append(" "); }
            b.append("\(s)s");
        }
        b.build()
    }

    // --- Parsing ---

    // Parse ISO 8601 duration: PT2H30M5S, PT1.5S, -PT3H, etc.
    public static func parse(from input: String) -> Duration throws ParseError {
        let bytes: Array[UInt8] = Array(from: input.bytes);
        var pos: Int64 = 0;
        var negative = false;

        // Optional leading '-'
        if pos < bytes.count and bytes(pos) == 45 {
            negative = true;
            pos = pos + 1;
        }

        // Expect 'P'
        guard pos < bytes.count and (bytes(pos) == 80 or bytes(pos) == 112) else {
            throw ParseError.InvalidFormat("expected 'P'");
        }
        pos = pos + 1;

        // Expect 'T' (we only support time durations, not date durations)
        guard pos < bytes.count and (bytes(pos) == 84 or bytes(pos) == 116) else {
            throw ParseError.InvalidFormat("expected 'T'");
        }
        pos = pos + 1;

        var totalSecs: Int64 = 0;
        var totalNanos: Int64 = 0;

        while pos < bytes.count {
            // Parse a number (possibly with decimal point)
            var intPart: Int64 = 0;
            var fracPart: Int64 = 0;
            var fracScale: Int64 = 1_000_000_000;
            var hasFrac = false;

            while pos < bytes.count {
                let b = Int64(from: bytes(pos));
                if b >= 48 and b <= 57 {
                    intPart = intPart * 10 + (b - 48);
                    pos = pos + 1;
                } else {
                    break;
                }
            }

            if pos < bytes.count and bytes(pos) == 46 {
                hasFrac = true;
                pos = pos + 1;
                while pos < bytes.count {
                    let b = Int64(from: bytes(pos));
                    if b >= 48 and b <= 57 and fracScale > 1 {
                        fracScale = fracScale / 10;
                        fracPart = fracPart + (b - 48) * fracScale;
                        pos = pos + 1;
                    } else {
                        break;
                    }
                }
            }

            // Expect unit designator: H, M, or S
            guard pos < bytes.count else { throw ParseError.UnexpectedEnd; }
            let unit = bytes(pos);
            pos = pos + 1;

            if unit == 72 or unit == 104 {
                // H/h = hours
                totalSecs = totalSecs + intPart * 3600;
                if hasFrac { totalNanos = totalNanos + fracPart * 3600; }
            } else if unit == 77 or unit == 109 {
                // M/m = minutes
                totalSecs = totalSecs + intPart * 60;
                if hasFrac { totalNanos = totalNanos + fracPart * 60; }
            } else if unit == 83 or unit == 115 {
                // S/s = seconds
                totalSecs = totalSecs + intPart;
                if hasFrac { totalNanos = totalNanos + fracPart; }
            } else {
                throw ParseError.InvalidFormat("expected H, M, or S");
            }
        }

        var d = Duration(seconds: totalSecs, nanoseconds: totalNanos);
        if negative { d = d.negated(); }
        d
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

    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        writer.append(self.isoString());
    }

    public func clone() -> Duration {
        Duration(raw: self.secs, rawNanos: self.nanos)
    }
}

// --- Helpers ---

// Append fractional nanoseconds, trimming trailing zeros
func appendFractional(mutating into b: StringBuilder, nanos: Int64) {
    var n = nanos;
    var digits: Int64 = 9;
    // Trim trailing zeros
    while digits > 0 and n % 10 == 0 {
        n = n / 10;
        digits = digits - 1;
    }
    // Pad leading zeros
    var temp = n;
    var digitCount: Int64 = 0;
    if temp == 0 {
        digitCount = 1;
    } else {
        while temp > 0 {
            temp = temp / 10;
            digitCount = digitCount + 1;
        }
    }
    while digitCount < digits {
        b.append("0");
        digitCount = digitCount + 1;
    }
    b.append("\(n)");
}

// Round a value to the nearest multiple of divisor using the given mode
func roundValue(value: Int64, divisor: Int64, mode: RoundMode) -> Int64 {
    let quot = value / divisor;
    let rem = value % divisor;
    if rem == 0 { return value }
    match mode {
        .Floor => quot * divisor,
        .Ceil => (quot + 1) * divisor,
        .Truncate => {
            if value >= 0 { quot * divisor } else { (quot + 1) * divisor }
        },
        .Expand => {
            if value >= 0 { (quot + 1) * divisor } else { quot * divisor }
        },
        .HalfExpand => {
            let half = divisor / 2;
            let absRem = if rem >= 0 { rem } else { 0 - rem };
            if absRem >= half {
                if value >= 0 { (quot + 1) * divisor } else { quot * divisor }
            } else {
                if value >= 0 { quot * divisor } else { (quot + 1) * divisor }
            }
        },
        .HalfEven => {
            let half = divisor / 2;
            let absRem = if rem >= 0 { rem } else { 0 - rem };
            if absRem > half {
                if value >= 0 { (quot + 1) * divisor } else { quot * divisor }
            } else if absRem < half {
                if value >= 0 { quot * divisor } else { (quot + 1) * divisor }
            } else {
                // Exactly half — round to even
                if quot % 2 == 0 { quot * divisor } else { (quot + 1) * divisor }
            }
        }
    }
}
