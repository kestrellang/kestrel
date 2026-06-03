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
    public var weeks: Int64 { self.d / 7 }
    public var remainingDays: Int64 { self.d % 7 }

    public var isNegative: Bool {
        if self.yrs != 0 { return self.yrs < 0 }
        if self.mos != 0 { return self.mos < 0 }
        self.d < 0
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

    public func isoString() -> String {
        var b = StringBuilder();
        if self.isNegative { b.append("-"); }
        b.append("P");
        let ay = self.yrs.abs();
        let am = self.mos.abs();
        let ad = self.d.abs();
        if ay > 0 { b.append("\(ay)Y"); }
        if am > 0 { b.append("\(am)M"); }
        if ad > 0 { b.append("\(ad)D"); }
        if ay == 0 and am == 0 and ad == 0 { b.append("0D"); }
        b.build()
    }

    public func humanString() -> String {
        var b = StringBuilder();
        if self.isNegative { b.append("-"); }
        let ay = self.yrs.abs();
        let am = self.mos.abs();
        let ad = self.d.abs();
        var wrote = false;
        if ay > 0 { b.append("\(ay)y"); wrote = true; }
        if am > 0 {
            if wrote { b.append(" "); }
            b.append("\(am)mo");
            wrote = true;
        }
        if ad > 0 or not wrote {
            if wrote { b.append(" "); }
            b.append("\(ad)d");
        }
        b.build()
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

    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
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
