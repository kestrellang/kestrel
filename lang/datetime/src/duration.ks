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

    // --- Rounding ---

    public func rounded(to unit: TimeUnit, mode mode: RoundMode = .HalfExpand) -> Duration {
        let divisor = unit.nanoseconds;
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
                appendFractional(into: b, a.nanos);
            }
            b.append("s");
        }
        b.build()
    }

    // --- Parsing ---

    // Parse ISO 8601 duration: PT2H30M5S, PT1.5S, -PT3H, P1DT2H (days = 24h), etc.
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

        var totalSecs: Int64 = 0;
        var totalNanos: Int64 = 0;

        // Optional date part. Only days are accepted, as exact 24-hour units:
        // years/months/weeks are calendar quantities and belong in Period.
        let daysStart = pos;
        var days: Int64 = 0;
        while pos < bytes.count and bytes(pos) >= 48 and bytes(pos) <= 57 {
            days = days * 10 + Int64(from: bytes(pos) - 48);
            pos = pos + 1;
        }
        let hasDays = pos > daysStart;
        if hasDays {
            guard pos < bytes.count and (bytes(pos) == 68 or bytes(pos) == 100) else {
                throw ParseError.InvalidFormat("expected 'D' (only days are allowed before 'T')");
            }
            pos = pos + 1;
            totalSecs = days * 86400;
        }

        // Optional time part: 'T' followed by at least one component.
        if pos < bytes.count {
            guard bytes(pos) == 84 or bytes(pos) == 116 else {
                throw ParseError.InvalidFormat("expected 'T'");
            }
            pos = pos + 1;
            guard pos < bytes.count else { throw ParseError.InvalidFormat("expected a component after 'T'"); }
        } else if not hasDays {
            throw ParseError.InvalidFormat("empty duration");
        }

        while pos < bytes.count {
            // Parse a number (possibly with decimal point)
            var intPart: Int64 = 0;
            var fracPart: Int64 = 0;
            var fracScale: Int64 = 1_000_000_000;
            var hasFrac = false;
            let numStart = pos;

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

            guard pos > numStart else { throw ParseError.InvalidFormat("expected a number at position \(pos)"); }

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

    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        writer.append(self.isoString());
    }

    public func clone() -> Duration {
        Duration(raw: self.secs, rawNanos: self.nanos)
    }
}

// --- Helpers ---

// Append fractional nanoseconds, trimming trailing zeros
func appendFractional(mutating into b: some Formatter, nanos: Int64) {
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
