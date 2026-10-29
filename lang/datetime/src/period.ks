module datetime

// Calendar duration: years, months, days.
// Weeks are sugar — converted to days at construction.
// No Comparable (can't order without a reference date).
public struct Period: Equatable, Hashable, Formattable, Cloneable {
    fileprivate var yrs: Int64
    fileprivate var mos: Int64
    // Stores total days including any weeks from construction
    fileprivate var d: Int64

    // --- Construction ---

    public init(years years: Int64 = 0, months months: Int64 = 0,
                weeks weeks: Int64 = 0, days days: Int64 = 0) {
        self.yrs = years;
        self.mos = months;
        self.d = weeks * 7 + days;
    }

    // Construct via the labeled init: `Period(years: 5)`, `Period(weeks: 2)`, etc.
    public static var zero: Period { Period() }

    // --- Properties ---

    public var years: Int64 { self.yrs }
    public var months: Int64 { self.mos }
    public var days: Int64 { self.d }
    // Whole weeks in `days`, truncated toward zero; remainingDays keeps the
    // sign (-10 days = -1 week and -3 days), matching Duration's accessors.
    public var weeks: Int64 { self.d / 7 }
    public var remainingDays: Int64 { self.d % 7 }

    // True when no field is positive and at least one is negative. A
    // mixed-sign period (P1M-3D) is neither negative nor positive.
    public var isNegative: Bool {
        self.yrs <= 0 and self.mos <= 0 and self.d <= 0 and not self.isZero
    }

    public var isZero: Bool {
        self.yrs == 0 and self.mos == 0 and self.d == 0
    }

    // --- Arithmetic ---

    public func negated() -> Period {
        var p = Period();
        p.yrs = 0 - self.yrs;
        p.mos = 0 - self.mos;
        p.d = 0 - self.d;
        p
    }

    public func adding(other: Period) -> Period {
        var p = Period();
        p.yrs = self.yrs + other.yrs;
        p.mos = self.mos + other.mos;
        p.d = self.d + other.d;
        p
    }

    public func subtracting(other: Period) -> Period {
        var p = Period();
        p.yrs = self.yrs - other.yrs;
        p.mos = self.mos - other.mos;
        p.d = self.d - other.d;
        p
    }

    public func multiplied(by factor: Int64) -> Period {
        var p = Period();
        p.yrs = self.yrs * factor;
        p.mos = self.mos * factor;
        p.d = self.d * factor;
        p
    }

    // --- Normalization ---

    // Normalizes months into years. Days are left as-is.
    public func normalized() -> Period {
        let totalMonths = self.yrs * 12 + self.mos;
        var p = Period();
        p.yrs = totalMonths / 12;
        p.mos = totalMonths % 12;
        // Fix sign: if years and months disagree, adjust
        if p.yrs > 0 and p.mos < 0 {
            p.yrs = p.yrs - 1;
            p.mos = p.mos + 12;
        } else if p.yrs < 0 and p.mos > 0 {
            p.yrs = p.yrs + 1;
            p.mos = p.mos - 12;
        }
        p.d = self.d;
        p
    }

    // --- Conversion ---

    // Resolve to exact Duration starting from a given date.
    // Days are treated as 86400 seconds (DST-unaware).
    public func toDuration(from date: Date) -> Duration {
        let end = date.adding(years: self.yrs, months: self.mos, days: self.d);
        let dayDiff = date.days(to: end);
        Duration.seconds(dayDiff * 86400)
    }

    // --- String representations ---

    // ISO 8601 form. Mixed-sign periods (which ISO can't express) sign each
    // field instead, as java.time.Period does: P1M-3D.
    public func isoString() -> String {
        var b = StringBuilder();
        let uniform = self.isNegative;
        let sign: Int64 = if uniform { -1 } else { 1 };
        if uniform { b.append("-"); }
        b.append("P");
        if self.yrs != 0 { b.append("\(self.yrs * sign)Y"); }
        if self.mos != 0 { b.append("\(self.mos * sign)M"); }
        if self.d != 0 { b.append("\(self.d * sign)D"); }
        if self.isZero { b.append("0D"); }
        b.build()
    }

    // Compact human form: "1y 2mo 3d", "-3d"; mixed signs per field: "1mo -3d".
    public func humanString() -> String {
        var b = StringBuilder();
        let uniform = self.isNegative;
        let sign: Int64 = if uniform { -1 } else { 1 };
        if uniform { b.append("-"); }
        var wrote = false;
        if self.yrs != 0 { b.append("\(self.yrs * sign)y"); wrote = true; }
        if self.mos != 0 {
            if wrote { b.append(" "); }
            b.append("\(self.mos * sign)mo");
            wrote = true;
        }
        if self.d != 0 or not wrote {
            if wrote { b.append(" "); }
            b.append("\(self.d * sign)d");
        }
        b.build()
    }

    // --- Parsing ---

    // Parse an ISO 8601 calendar duration: "P1Y2M3W4D", "-P1M", "P0D", and the
    // mixed-sign "P1M-3D" form isoString() emits. Units
    // must appear in Y, M, W, D order; weeks fold into days. Time parts
    // ("T...") belong to Duration and are rejected.
    public static func parse(from input: String) -> Period throws ParseError {
        let bytes: Array[UInt8] = Array(from: input.bytes);
        var pos: Int64 = 0;
        let negative = pos < bytes.count and bytes(pos) == 45;
        if negative { pos = pos + 1; }
        guard pos < bytes.count and (bytes(pos) == 80 or bytes(pos) == 112) else {
            throw ParseError.InvalidFormat("expected 'P'");
        }
        pos = pos + 1;
        guard pos < bytes.count else { throw ParseError.InvalidFormat("empty period"); }

        // Designators in required order; `rank` rejects repeats and reordering.
        let designators: Array[UInt8] = [89, 77, 87, 68]; // Y M W D
        var values: Array[Int64] = [0, 0, 0, 0];
        var rank: Int64 = 0;
        while pos < bytes.count {
            // A per-field '-' is the mixed-sign form isoString() emits.
            let fieldNegative = pos < bytes.count and bytes(pos) == 45;
            if fieldNegative { pos = pos + 1; }
            let numStart = pos;
            var n: Int64 = 0;
            while pos < bytes.count and bytes(pos) >= 48 and bytes(pos) <= 57 {
                n = n * 10 + Int64(from: bytes(pos) - 48);
                pos = pos + 1;
            }
            guard pos > numStart else { throw ParseError.InvalidFormat("expected a number at position \(pos)"); }
            if fieldNegative { n = 0 - n; }
            guard pos < bytes.count else { throw ParseError.UnexpectedEnd; }
            // Upper-case the designator byte
            let unit = if bytes(pos) >= 97 { bytes(pos) - 32 } else { bytes(pos) };
            var slot = rank;
            while slot < 4 and designators(slot) != unit { slot = slot + 1; }
            guard slot < 4 else {
                throw ParseError.InvalidFormat("expected Y, M, W or D in that order at position \(pos)");
            }
            values(slot) = n;
            rank = slot + 1;
            pos = pos + 1;
        }

        let p = Period(years: values(0), months: values(1), weeks: values(2), days: values(3));
        .Ok(if negative { p.negated() } else { p })
    }

    // --- Protocol conformances ---

    public func isEqual(to other: Period) -> Bool {
        self.yrs == other.yrs and self.mos == other.mos and self.d == other.d
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.yrs.hash(into: hasher);
        self.mos.hash(into: hasher);
        self.d.hash(into: hasher);
    }

    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        writer.append(self.isoString());
    }

    public func clone() -> Period {
        var p = Period();
        p.yrs = self.yrs;
        p.mos = self.mos;
        p.d = self.d;
        p
    }
}
