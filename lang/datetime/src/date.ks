module datetime

import std.memory.(Pointer)

// Calendar date without time or timezone.
// Default Formattable output: ISO 8601 ("2024-07-04").
public struct Date: Equatable, Comparable, Hashable, Formattable, Cloneable {
    var y: Int64
    var m: Int64
    var d: Int64

    // --- Construction ---

    public init(year year: Int64, month month: Int64, day day: Int64) throws DateError {
        guard isValidDate(year, month, day) else {
            throw DateError.InvalidDate(year: year, month: month, day: day);
        }
        self.y = year;
        self.m = month;
        self.d = day;
    }

    // Direct field construction — no validation, no recursion. The primitive
    // every other non-throwing constructor builds on (mirrors Instant's
    // `init(secondsSinceEpoch:)`). Callers that produce values by construction
    // (date arithmetic, parsing) guarantee validity.
    init(rawYear y: Int64, rawMonth m: Int64, rawDay d: Int64) {
        self.y = y;
        self.m = m;
        self.d = d;
    }

    // No validation — traps on truly insane values but trusts the caller
    public static func unchecked(year year: Int64, month month: Int64, day day: Int64) -> Date {
        Date(rawYear: year, rawMonth: month, rawDay: day)
    }

    public static func today() -> Date {
        var secs: Int64 = 0;
        var nanos: Int64 = 0;
        kestrel_clock_gettime(Pointer(to: secs), Pointer(to: nanos));
        let offset = kestrel_localtime_gmtoff(secs);
        let localSecs = secs + offset;
        dateFromEpochDay(localSecs / 86400)
    }

    public static func today(in zone: TimeZone) -> Date {
        var secs: Int64 = 0;
        var nanos: Int64 = 0;
        kestrel_clock_gettime(Pointer(to: secs), Pointer(to: nanos));
        let offset = Int64(from: kestrel_tz_offset(zone.id, secs));
        let localSecs = secs + offset;
        dateFromEpochDay(localSecs / 86400)
    }

    // --- Validation ---

    public static func isValid(year year: Int64, month month: Int64, day day: Int64) -> Bool {
        isValidDate(year, month, day)
    }

    // --- Properties ---

    public var year: Int64 { self.y }
    public var month: Int64 { self.m }
    public var day: Int64 { self.d }

    public var weekday: Weekday {
        weekdayOf(self.y, self.m, self.d)
    }

    public var dayOfYear: Int64 {
        dayOfYearOf(self.y, self.m, self.d)
    }

    public var isLeapYear: Bool {
        isLeapYearCheck(self.y)
    }

    public var daysInMonth: Int64 {
        daysInMonthOf(self.y, self.m)
    }

    public var daysInYear: Int64 {
        if self.isLeapYear { 366 } else { 365 }
    }

    // --- Calendar Arithmetic ---

    public func adding(years y: Int64 = 0, months m: Int64 = 0, days d: Int64 = 0,
                       overflow o: Overflow = .Clip) -> Date {
        // Step 1: Add years and months
        let totalMonths = (self.y * 12 + (self.m - 1)) + (y * 12 + m);
        var newYear = floorDiv(totalMonths, 12);
        var newMonth = floorMod(totalMonths, 12) + 1;

        // Step 2: Handle day overflow
        let maxDay = daysInMonthOf(newYear, newMonth);
        var newDay = self.d;

        if newDay > maxDay {
            match o {
                .Clip => { newDay = maxDay; },
                .Rollover => {
                    // Roll excess days into next month(s)
                    var excess = newDay - maxDay;
                    newDay = maxDay;
                    let base = daysToCivil(newYear, newMonth, newDay);
                    let result = dateFromEpochDay(base + excess);
                    // Add the days parameter too
                    if d != 0 {
                        let finalDay = daysToCivil(result.y, result.m, result.d) + d;
                        return dateFromEpochDay(finalDay);
                    }
                    return result;
                }
            };
        }

        // Step 3: Add days
        if d != 0 {
            let dayNum = daysToCivil(newYear, newMonth, newDay) + d;
            return dateFromEpochDay(dayNum);
        }

        Date.unchecked(year: newYear, month: newMonth, day: newDay)
    }

    public func adding(period p: Period, overflow o: Overflow = .Clip) -> Date {
        self.adding(years: p.years, months: p.months, days: p.days, overflow: o)
    }

    // --- Navigation ---

    public func tomorrow() -> Date {
        dateFromEpochDay(daysToCivil(self.y, self.m, self.d) + 1)
    }

    public func yesterday() -> Date {
        dateFromEpochDay(daysToCivil(self.y, self.m, self.d) - 1)
    }

    public func startOfMonth() -> Date {
        Date.unchecked(year: self.y, month: self.m, day: 1)
    }

    public func endOfMonth() -> Date {
        Date.unchecked(year: self.y, month: self.m, day: self.daysInMonth)
    }

    public func startOfYear() -> Date {
        Date.unchecked(year: self.y, month: 1, day: 1)
    }

    public func endOfYear() -> Date {
        Date.unchecked(year: self.y, month: 12, day: 31)
    }

    // --- Difference ---

    public func days(to other: Date) -> Int64 {
        daysToCivil(other.y, other.m, other.d) - daysToCivil(self.y, self.m, self.d)
    }

    public func period(to other: Date) -> Period {
        var years = other.y - self.y;
        var months = other.m - self.m;
        var days = other.d - self.d;

        // Normalize negative days
        if days < 0 {
            months = months - 1;
            // Days in the month before the target
            let prevMonth = if other.m == 1 { 12 } else { other.m - 1 };
            let prevYear = if other.m == 1 { other.y - 1 } else { other.y };
            days = days + daysInMonthOf(prevYear, prevMonth);
        }

        // Normalize negative months
        if months < 0 {
            years = years - 1;
            months = months + 12;
        }

        Period(years: years, months: months, weeks: 0, days: days)
    }

    // --- Conversion ---

    public func toDateTime(at time: Time) -> DateTime {
        DateTime(date: self, time: time)
    }

    // --- Parsing ---

    public static func parse(from input: String) -> Date throws ParseError {
        // Parse ISO 8601: YYYY-MM-DD
        let bytes: Array[UInt8] = Array(from: input.bytes);
        guard bytes.count >= 10 else { throw ParseError.UnexpectedEnd; }
        let year = try parseDigits(bytes, 0, 4);
        guard bytes(4) == 45 else { throw ParseError.InvalidFormat("expected '-' at position 4"); } // '-' = 45
        let month = try parseDigits(bytes, 5, 2);
        guard bytes(7) == 45 else { throw ParseError.InvalidFormat("expected '-' at position 7"); }
        let day = try parseDigits(bytes, 8, 2);
        guard isValidDate(year, month, day) else {
            throw ParseError.InvalidValue("invalid date: \(year)-\(month)-\(day)");
        }
        .Ok(Date.unchecked(year: year, month: month, day: day))
    }

    // --- Protocol conformances ---

    public func isEqual(to other: Date) -> Bool {
        self.y == other.y and self.m == other.m and self.d == other.d
    }

    public func compare(other: Date) -> Ordering {
        self.y.compare(other.y)
            .then(self.m.compare(other.m))
            .then(self.d.compare(other.d))
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.y.hash(into: hasher);
        self.m.hash(into: hasher);
        self.d.hash(into: hasher);
    }

    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        // ISO 8601: YYYY-MM-DD
        appendPadded(into: writer, self.y, 4);
        writer.append("-");
        appendPadded(into: writer, self.m, 2);
        writer.append("-");
        appendPadded(into: writer, self.d, 2);
    }

    public func clone() -> Date {
        Date.unchecked(year: self.y, month: self.m, day: self.d)
    }
}

// ============================================================================
// Calendar helpers (module-internal)
// ============================================================================

func isLeapYearCheck(year: Int64) -> Bool {
    year % 4 == 0 and (year % 100 != 0 or year % 400 == 0)
}

func daysInMonthOf(year: Int64, month: Int64) -> Int64 {
    match month {
        1 => 31, 2 => if isLeapYearCheck(year) { 29 } else { 28 },
        3 => 31, 4 => 30, 5 => 31, 6 => 30,
        7 => 31, 8 => 31, 9 => 30, 10 => 31,
        11 => 30, 12 => 31,
        _ => 30
    }
}

func dayOfYearOf(year: Int64, month: Int64, day: Int64) -> Int64 {
    var result = day;
    var m: Int64 = 1;
    while m < month {
        result = result + daysInMonthOf(year, m);
        m = m + 1;
    }
    result
}

func isValidDate(year: Int64, month: Int64, day: Int64) -> Bool {
    if month < 1 or month > 12 { return false }
    if day < 1 or day > daysInMonthOf(year, month) { return false }
    true
}

// Tomohiko Sakamoto's day-of-week algorithm
// Returns 0=Sunday, 1=Monday, ..., 6=Saturday, then maps to Weekday enum
func weekdayOf(year: Int64, month: Int64, day: Int64) -> Weekday {
    var y = year;
    let t = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    if month < 3 { y = y - 1; }
    let dow = (y + y / 4 - y / 100 + y / 400 + t(month - 1) + day) % 7;
    // dow: 0=Sunday, 1=Monday, ...
    let adjusted = if dow <= 0 { (dow + 7) % 7 } else { dow };
    match adjusted {
        0 => Weekday.Sunday,
        1 => Weekday.Monday,
        2 => Weekday.Tuesday,
        3 => Weekday.Wednesday,
        4 => Weekday.Thursday,
        5 => Weekday.Friday,
        6 => Weekday.Saturday,
        _ => Weekday.Monday
    }
}

// Howard Hinnant's civil_from_days algorithm
// dayNumber: days since 1970-01-01 (can be negative)
func dateFromEpochDay(dayNumber: Int64) -> Date {
    let z = dayNumber + 719468;
    let era = floorDiv(z, 146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + (if mp < 10 { 3 } else { -9 });
    let yr = y + (if m <= 2 { 1 } else { 0 });
    Date.unchecked(year: yr, month: m, day: d)
}

// Inverse: civil date to day number since 1970-01-01
func daysToCivil(year: Int64, month: Int64, day: Int64) -> Int64 {
    let y = year - (if month <= 2 { 1 } else { 0 });
    let era = floorDiv(y, 400);
    let yoe = y - era * 400;
    let doy = (153 * (month + (if month > 2 { -3 } else { 9 })) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

// Floor division (rounds toward negative infinity)
func floorDiv(a: Int64, b: Int64) -> Int64 {
    let q = a / b;
    let r = a % b;
    if (r != 0) and ((r < 0) != (b < 0)) { q - 1 } else { q }
}

// Floor modulo (result has same sign as divisor)
func floorMod(a: Int64, b: Int64) -> Int64 {
    let r = a % b;
    if (r != 0) and ((r < 0) != (b < 0)) { r + b } else { r }
}

// Parse `count` decimal digits from utf8 bytes starting at `offset`
func parseDigits(bytes: Array[UInt8], offset: Int64, count: Int64) -> Int64 throws ParseError {
    var result: Int64 = 0;
    var i: Int64 = 0;
    while i < count {
        let b = Int64(from: bytes(offset + i));
        if b < 48 or b > 57 {
            throw ParseError.InvalidFormat("expected digit at position \(offset + i)");
        }
        result = result * 10 + (b - 48);
        i = i + 1;
    }
    result
}

// Zero-pad a number to `width` digits and append to builder
func appendPadded(mutating into writer: StringBuilder, value: Int64, width: Int64) {
    let v = value.abs();
    if value < 0 { writer.append("-"); }
    // Count digits
    var temp = v;
    var digits: Int64 = 0;
    if temp == 0 {
        digits = 1;
    } else {
        while temp > 0 {
            temp = temp / 10;
            digits = digits + 1;
        }
    }
    // Pad
    var pad = width - digits;
    while pad > 0 {
        writer.append("0");
        pad = pad - 1;
    }
    writer.append("\(v)");
}
