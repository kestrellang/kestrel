module datetime

// Duration itself lives in std (`std.time`, visible everywhere), shared with
// every std timeout. The calendar-aware extras stay here: rounding to a
// TimeUnit and ISO 8601 parsing.

extend Duration {
    // --- Rounding ---

    public func rounded(to unit: TimeUnit, mode mode: RoundMode = .HalfExpand) -> Duration {
        let divisor = unit.nanoseconds;
        if divisor <= 1 { return self }
        let total = self.totalNanoseconds;
        let rounded = roundValue(total, divisor, mode);
        Duration.nanoseconds(rounded)
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
