// ============================================================================
// std.datetime — Full API Draft (v2)
// ============================================================================
//
// Core types: Instant, Date, Time, DateTime, ZonedDateTime, Duration, Period
// Support:    TimeZone, Format, Clock protocol
//
// Design principles:
//   - Separate types enforce naive/aware distinction at compile time
//   - Immutable value types (all copyable)
//   - Named parameters replace builder types
//   - String interpolation for custom formats (no strftime, no CLDR)
//   - Duration (exact) and Period (calendar) are separate types
//   - TimeZone is interned (small integer ID, copyable)
//   - Common path never throws — only .Reject overflow/disambiguation throws
//   - Month/weekday names are English only in v1 (no locale support)

module std.datetime

// ============================================================================
// ENUMS
// ============================================================================

public enum Weekday: Equatable, Comparable, Hashable, Formattable {
    case Monday
    case Tuesday
    case Wednesday
    case Thursday
    case Friday
    case Saturday
    case Sunday
}

// Controls what happens when calendar arithmetic produces an invalid date
// (e.g. Jan 31 + 1 month = "Feb 31"). Clip and Rollover always succeed.
public enum Overflow {
    case Clip       // Feb 31 -> Feb 28 (clamp to last valid day)
    case Rollover   // Feb 31 -> Mar 3 (keep counting forward)
}

// Controls how ambiguous/nonexistent civil times are resolved during
// DST transitions. Compatible, Earlier, and Later always succeed.
public enum Disambiguation {
    case Compatible // gap -> later time, fold -> earlier time (RFC 5545)
    case Earlier    // always pick the earlier instant
    case Later      // always pick the later instant
}

public enum RoundMode {
    case Ceil       // toward +infinity
    case Floor      // toward -infinity
    case Expand     // away from zero
    case Trunc      // toward zero
    case HalfExpand // round half away from zero (default)
    case HalfEven   // round half to even (banker's rounding)
}

public enum TimeUnit {
    case Nanosecond
    case Microsecond
    case Millisecond
    case Second
    case Minute
    case Hour
    case Day
}

// ============================================================================
// CLOCK PROTOCOL
// ============================================================================

public protocol Clock {
    func now() -> Instant;
}

public struct SystemClock: Clock {
    public func now() -> Instant { ... }
    public static var shared: SystemClock { SystemClock() }
}

// For testing — manually control the clock.
public struct FakeClock: Clock {
    public init(at instant: Instant) { ... }
    public mutating func advance(by duration: Duration) { ... }
    public mutating func setTo(instant: Instant) { ... }
    public func now() -> Instant { ... }
}

// ============================================================================
// TIMEZONE
// ============================================================================

// Interned — TimeZone is a small integer ID into a process-global registry.
// Fully copyable. Loaded timezones live for process lifetime.
// Reads system zoneinfo (/usr/share/zoneinfo on Unix).
public struct TimeZone: Equatable, Hashable, Formattable {

    // --- Statics ---

    public static var utc: TimeZone { ... }
    public static func system() -> TimeZone { ... }
    public static func find(name: String) -> TimeZone? { ... }

    // --- Properties ---

    public var name: String { ... }

    // --- Query ---

    // Whether a civil datetime is ambiguous (DST fold) in this timezone.
    public func isAmbiguous(dateTime: DateTime) -> Bool { ... }

    // Whether a civil datetime is nonexistent (DST gap) in this timezone.
    public func isNonexistent(dateTime: DateTime) -> Bool { ... }

    // --- Equatable ---

    public func isEqual(to other: TimeZone) -> Bool { ... }
}

// ============================================================================
// INSTANT
// ============================================================================

// Absolute point in time. Nanosecond precision. No calendar, no timezone.
// Internal: Int64 seconds + Int32 nanoseconds since 1970-01-01T00:00:00Z.
// Default Formattable output: RFC 3339 ("2024-07-04T15:30:05Z").
// No custom formatted(as:) — an instant has no civil components to format.
// To format with custom components, convert to ZonedDateTime first.
public struct Instant: Equatable, Comparable, Hashable, Formattable {

    // --- Construction ---

    public static func now() -> Instant { ... }
    public static func now(from clock: some Clock) -> Instant { ... }
    public init(secondsSinceEpoch seconds: Int64, nanoseconds nanoseconds: Int64 = 0) { ... }
    public init(millisecondsSinceEpoch ms: Int64) { ... }

    // --- Properties ---

    public var secondsSinceEpoch: Int64 { ... }
    public var nanosecondsSinceEpoch: Int64 { ... }
    public var millisecondsSinceEpoch: Int64 { ... }
    public var subsecondNanoseconds: Int64 { ... }

    // --- Arithmetic ---

    public func advanced(by duration: Duration) -> Instant { ... }

    // --- Difference ---

    public func duration(to other: Instant) -> Duration { ... }

    // --- Conversion ---

    public func toDate(in zone: TimeZone) -> Date { ... }
    public func toTime(in zone: TimeZone) -> Time { ... }
    public func toDateTime(in zone: TimeZone) -> DateTime { ... }
    public func toZoned(in zone: TimeZone) -> ZonedDateTime { ... }

    // --- Rounding ---

    public func rounded(to unit: TimeUnit, mode mode: RoundMode = .HalfExpand) -> Instant { ... }

    // --- Parsing (RFC 3339 only) ---

    public static func parse(from input: String) -> Instant throws ParseError { ... }
}

// Instant + Duration -> Instant
extend Instant {
    public static func +(left: Instant, right: Duration) -> Instant { ... }
    public static func -(left: Instant, right: Duration) -> Instant { ... }
    public static func -(left: Instant, right: Instant) -> Duration { ... }
}

// ============================================================================
// DATE
// ============================================================================

// Calendar date without time or timezone.
// Internal: packed year (Int32) + month (Int8) + day (Int8).
// Default Formattable output: ISO 8601 ("2024-07-04").
public struct Date: Equatable, Comparable, Hashable, Formattable {

    // --- Construction ---

    public init(year year: Int64, month month: Int64, day day: Int64) throws DateError { ... }
    public static func unchecked(year year: Int64, month month: Int64, day day: Int64) -> Date { ... }
    public static func today() -> Date { ... }
    public static func today(in zone: TimeZone) -> Date { ... }
    public static func today(from clock: some Clock) -> Date { ... }
    public static func today(from clock: some Clock, in zone: TimeZone) -> Date { ... }

    // --- Validation ---

    public static func isValid(year year: Int64, month month: Int64, day day: Int64) -> Bool { ... }

    // --- Properties ---

    public var year: Int64 { ... }
    public var month: Int64 { ... }
    public var day: Int64 { ... }
    public var weekday: Weekday { ... }
    public var dayOfYear: Int64 { ... }
    public var isLeapYear: Bool { ... }
    public var daysInMonth: Int64 { ... }
    public var daysInYear: Int64 { ... }

    // --- Calendar Arithmetic (never throws — Clip/Rollover always succeed) ---

    public func adding(years y: Int64 = 0, months m: Int64 = 0, days d: Int64 = 0,
                       overflow o: Overflow = .Clip) -> Date { ... }
    public func adding(period p: Period, overflow o: Overflow = .Clip) -> Date { ... }

    // --- Navigation ---

    public func tomorrow() -> Date { ... }
    public func yesterday() -> Date { ... }
    public func startOfMonth() -> Date { ... }
    public func endOfMonth() -> Date { ... }
    public func startOfYear() -> Date { ... }
    public func endOfYear() -> Date { ... }

    // --- Difference ---

    public func days(to other: Date) -> Int64 { ... }
    public func period(to other: Date) -> Period { ... }

    // --- Conversion ---

    public func toDateTime(at time: Time) -> DateTime { ... }
    public func toZoned(at time: Time, in zone: TimeZone,
                        disambiguation d: Disambiguation = .Compatible) -> ZonedDateTime { ... }
    public func toInstant(at time: Time, in zone: TimeZone,
                          disambiguation d: Disambiguation = .Compatible) -> Instant { ... }

    // --- Formatting ---

    public func formatted(as fmt: Format) -> String { ... }

    // --- Parsing ---

    public static func parse(from input: String) -> Date throws ParseError { ... }
    public static func parse(from input: String, as fmt: Format) -> Date throws ParseError { ... }
}

// ============================================================================
// TIME
// ============================================================================

// Wall-clock time of day. Nanosecond precision within [00:00:00, 23:59:59.999999999].
// Internal: Int64 nanoseconds since midnight.
// Default Formattable output: ISO 8601 ("15:30:05").
public struct Time: Equatable, Comparable, Hashable, Formattable {

    // --- Construction ---

    public init(hour hour: Int64, minute minute: Int64 = 0, second second: Int64 = 0,
                nanosecond nanosecond: Int64 = 0) throws DateError { ... }
    public static var midnight: Time { ... }
    public static var noon: Time { ... }

    // --- Properties ---

    public var hour: Int64 { ... }
    public var minute: Int64 { ... }
    public var second: Int64 { ... }
    public var nanosecond: Int64 { ... }

    // --- Arithmetic ---

    // Wraps at day boundaries (23:59:59 + 1s = 00:00:00). Days are lost.
    public func advanced(by duration: Duration) -> Time { ... }

    // Wraps at day boundaries and reports how many days overflowed.
    public func advancedWithOverflow(by duration: Duration) -> (Time, Int64) { ... }

    // --- Difference ---

    public func duration(to other: Time) -> Duration { ... }

    // --- Rounding ---

    public func rounded(to unit: TimeUnit, mode mode: RoundMode = .HalfExpand) -> Time { ... }

    // --- Conversion ---

    public func toDateTime(on date: Date) -> DateTime { ... }

    // --- Formatting ---

    public func formatted(as fmt: Format) -> String { ... }

    // --- Parsing ---

    public static func parse(from input: String) -> Time throws ParseError { ... }
    public static func parse(from input: String, as fmt: Format) -> Time throws ParseError { ... }
}

// ============================================================================
// DATETIME
// ============================================================================

// Date + Time without timezone. "What the wall clock shows."
// Internal: Date + Time.
// Default Formattable output: ISO 8601 ("2024-07-04T15:30:05").
public struct DateTime: Equatable, Comparable, Hashable, Formattable {

    // --- Construction ---

    public init(year year: Int64, month month: Int64, day day: Int64,
                hour hour: Int64 = 0, minute minute: Int64 = 0,
                second second: Int64 = 0, nanosecond nanosecond: Int64 = 0) throws DateError { ... }
    public init(date date: Date, time time: Time) { ... }

    // --- Properties ---

    public var date: Date { ... }
    public var time: Time { ... }
    public var year: Int64 { ... }
    public var month: Int64 { ... }
    public var day: Int64 { ... }
    public var hour: Int64 { ... }
    public var minute: Int64 { ... }
    public var second: Int64 { ... }
    public var nanosecond: Int64 { ... }
    public var weekday: Weekday { ... }
    public var dayOfYear: Int64 { ... }

    // --- Exact Arithmetic ---

    public func advanced(by duration: Duration) -> DateTime { ... }

    // --- Calendar Arithmetic (never throws) ---

    public func adding(years y: Int64 = 0, months m: Int64 = 0, days d: Int64 = 0,
                       overflow o: Overflow = .Clip) -> DateTime { ... }
    public func adding(period p: Period, overflow o: Overflow = .Clip) -> DateTime { ... }

    // --- Navigation ---

    public func tomorrow() -> DateTime { ... }
    public func yesterday() -> DateTime { ... }
    public func startOfDay() -> DateTime { ... }
    public func endOfDay() -> DateTime { ... }
    public func startOfMonth() -> DateTime { ... }
    public func endOfMonth() -> DateTime { ... }
    public func startOfYear() -> DateTime { ... }
    public func endOfYear() -> DateTime { ... }

    // --- Difference ---

    public func duration(to other: DateTime) -> Duration { ... }
    public func period(to other: DateTime) -> Period { ... }

    // --- Query ---

    // Whether this civil datetime is ambiguous (DST fold) in the given timezone.
    public func isAmbiguous(in zone: TimeZone) -> Bool { ... }

    // --- Conversion ---

    public func toZoned(in zone: TimeZone,
                        disambiguation d: Disambiguation = .Compatible) -> ZonedDateTime { ... }
    public func toInstant(in zone: TimeZone,
                          disambiguation d: Disambiguation = .Compatible) -> Instant { ... }

    // --- Formatting ---

    public func formatted(as fmt: Format) -> String { ... }

    // --- Parsing ---

    public static func parse(from input: String) -> DateTime throws ParseError { ... }
    public static func parse(from input: String, as fmt: Format) -> DateTime throws ParseError { ... }
}

// ============================================================================
// ZONED DATETIME
// ============================================================================

// DateTime + IANA timezone. The "full" type — a precise instant tied to a
// geographic region. Handles DST transitions correctly.
// Internal: Instant + TimeZone (interned ID) + cached civil components.
// Default Formattable output: RFC 9557 ("2024-07-04T15:30:05-04:00[America/New_York]").
// Compared by underlying instant (timezone is ignored for ordering).
public struct ZonedDateTime: Equatable, Comparable, Hashable, Formattable {

    // --- Construction ---

    public init(year year: Int64, month month: Int64, day day: Int64,
                hour hour: Int64 = 0, minute minute: Int64 = 0,
                second second: Int64 = 0, nanosecond nanosecond: Int64 = 0,
                in zone: TimeZone,
                disambiguation d: Disambiguation = .Compatible) throws DateError { ... }
    public init(instant instant: Instant, in zone: TimeZone) { ... }
    public init(dateTime dateTime: DateTime, in zone: TimeZone,
                disambiguation d: Disambiguation = .Compatible) { ... }

    // --- Now ---

    public static func now() -> ZonedDateTime { ... }
    public static func now(from clock: some Clock) -> ZonedDateTime { ... }
    public static func now(in zone: TimeZone) -> ZonedDateTime { ... }
    public static func now(from clock: some Clock, in zone: TimeZone) -> ZonedDateTime { ... }

    // --- Properties ---

    public var instant: Instant { ... }
    public var dateTime: DateTime { ... }
    public var date: Date { ... }
    public var time: Time { ... }
    public var timeZone: TimeZone { ... }
    public var year: Int64 { ... }
    public var month: Int64 { ... }
    public var day: Int64 { ... }
    public var hour: Int64 { ... }
    public var minute: Int64 { ... }
    public var second: Int64 { ... }
    public var nanosecond: Int64 { ... }
    public var weekday: Weekday { ... }
    public var dayOfYear: Int64 { ... }

    // --- Exact Arithmetic (preserves absolute time difference) ---

    // Adding Duration.hours(24) always adds exactly 24 hours.
    public func advanced(by duration: Duration) -> ZonedDateTime { ... }

    // --- Calendar Arithmetic (preserves wall-clock time across DST) ---

    // Adding days: 1 preserves wall-clock time (may be 23 or 25 absolute hours).
    // If the result lands in a DST gap, Compatible disambiguation is used
    // implicitly. For explicit control, use dateTime.adding(...) then toZoned().
    public func adding(years y: Int64 = 0, months m: Int64 = 0, days d: Int64 = 0,
                       overflow o: Overflow = .Clip) -> ZonedDateTime { ... }
    public func adding(period p: Period, overflow o: Overflow = .Clip) -> ZonedDateTime { ... }

    // --- Navigation ---

    public func startOfDay() -> ZonedDateTime { ... }
    public func endOfDay() -> ZonedDateTime { ... }
    public func startOfMonth() -> ZonedDateTime { ... }
    public func endOfMonth() -> ZonedDateTime { ... }
    public func startOfYear() -> ZonedDateTime { ... }
    public func endOfYear() -> ZonedDateTime { ... }
    public func tomorrow() -> ZonedDateTime { ... }
    public func yesterday() -> ZonedDateTime { ... }

    // --- Difference ---

    public func duration(to other: ZonedDateTime) -> Duration { ... }

    // --- Timezone Conversion (preserves instant, recomputes civil time) ---

    public func inZone(zone: TimeZone) -> ZonedDateTime { ... }

    // --- Rounding ---

    public func rounded(to unit: TimeUnit, mode mode: RoundMode = .HalfExpand) -> ZonedDateTime { ... }

    // --- Formatting ---

    public func formatted(as fmt: Format) -> String { ... }

    // --- Parsing ---

    public static func parse(from input: String) -> ZonedDateTime throws ParseError { ... }
    public static func parse(from input: String, as fmt: Format) -> ZonedDateTime throws ParseError { ... }

    // --- Equatable / Comparable (by underlying instant) ---

    public func isEqual(to other: ZonedDateTime) -> Bool { ... }
    public func isLessThan(bound: ZonedDateTime) -> Bool { ... }
}

// ============================================================================
// DURATION
// ============================================================================

// Exact elapsed time. Signed. Nanosecond precision.
// Internal: Int64 seconds + Int32 nanoseconds (sign is uniform).
// For timers, benchmarks, network timeouts, exact intervals.
// Default Formattable output: ISO 8601 ("PT2H30M5S").
public struct Duration: Equatable, Comparable, Hashable, Formattable {

    // --- Construction ---

    public init(seconds seconds: Int64, nanoseconds nanoseconds: Int64 = 0) { ... }
    public static func nanoseconds(n: Int64) -> Duration { ... }
    public static func microseconds(n: Int64) -> Duration { ... }
    public static func milliseconds(n: Int64) -> Duration { ... }
    public static func seconds(n: Int64) -> Duration { ... }
    public static func minutes(n: Int64) -> Duration { ... }
    public static func hours(n: Int64) -> Duration { ... }
    public static var zero: Duration { ... }

    // --- Properties ---

    public var totalSeconds: Int64 { ... }
    public var totalMilliseconds: Int64 { ... }
    public var totalNanoseconds: Int64 { ... }
    public var subsecondNanoseconds: Int64 { ... }
    public var isNegative: Bool { ... }
    public var isZero: Bool { ... }

    // --- Arithmetic ---

    public func negated() -> Duration { ... }
    public func abs() -> Duration { ... }
    public func adding(other: Duration) -> Duration { ... }
    public func subtracting(other: Duration) -> Duration { ... }
    public func multiplied(by factor: Int64) -> Duration { ... }
    public func multiplied(by factor: Float64) -> Duration { ... }
    public func divided(by divisor: Int64) -> Duration { ... }
    public func divided(by divisor: Float64) -> Duration { ... }

    // --- Rounding ---

    public func rounded(to unit: TimeUnit, mode mode: RoundMode = .HalfExpand) -> Duration { ... }

    // --- Formatting ---

    // --- String Representations ---

    // "PT2H30M5S" (ISO 8601 duration)
    public func isoString() -> String { ... }

    // "2h 30m 5s" (human-readable)
    public func friendlyString() -> String { ... }

    // --- Parsing ---

    public static func parse(from input: String) -> Duration throws ParseError { ... }
}

// Duration operators
extend Duration {
    public static func +(left: Duration, right: Duration) -> Duration { ... }
    public static func -(left: Duration, right: Duration) -> Duration { ... }
    public static func *(left: Duration, right: Int64) -> Duration { ... }
    public static func *(left: Int64, right: Duration) -> Duration { ... }
    public static func /(left: Duration, right: Int64) -> Duration { ... }
    public static prefix func -(value: Duration) -> Duration { ... }
}

// ============================================================================
// PERIOD
// ============================================================================

// Calendar duration: years, months, days. Each field is independent.
// "1 month" means different absolute durations depending on which month.
// No Comparable — can't order without a reference date.
// Default Formattable output: ISO 8601 ("P1Y6M10D").
// Weeks are sugar — Period.weeks(2) stores days: 14 internally.
// Period(days: 14) == Period(weeks: 2) because both store the same fields.
public struct Period: Equatable, Hashable, Formattable {

    // --- Construction ---

    // Weeks are converted to days: weeks * 7 + days.
    public init(years years: Int64 = 0, months months: Int64 = 0,
                weeks weeks: Int64 = 0, days days: Int64 = 0) { ... }
    public static func years(n: Int64) -> Period { ... }
    public static func months(n: Int64) -> Period { ... }
    public static func weeks(n: Int64) -> Period { ... }
    public static func days(n: Int64) -> Period { ... }
    public static var zero: Period { ... }

    // --- Properties ---

    public var years: Int64 { ... }
    public var months: Int64 { ... }
    public var days: Int64 { ... }          // total days (including any weeks from construction)
    public var weeks: Int64 { ... }         // computed: self.days / 7
    public var remainingDays: Int64 { ... } // computed: self.days % 7
    public var isNegative: Bool { ... }
    public var isZero: Bool { ... }

    // --- Arithmetic ---

    public func negated() -> Period { ... }
    public func adding(other: Period) -> Period { ... }
    public func subtracting(other: Period) -> Period { ... }
    public func multiplied(by factor: Int64) -> Period { ... }

    // --- Normalization ---

    // Normalizes months into years only. Days are left as-is.
    // Period(months: 14, days: 10) -> Period(years: 1, months: 2, days: 10).
    public func normalized() -> Period { ... }

    // --- Conversion ---

    // Resolve to exact Duration starting from a given date.
    // Days are treated as 86400 seconds (DST-unaware).
    // For DST-aware conversion, add the period to a ZonedDateTime and
    // compute duration between the two instants.
    public func toDuration(from date: Date) -> Duration { ... }

    // --- String Representations ---

    // "P1Y6M10D" (ISO 8601 period)
    public func isoString() -> String { ... }

    // "1y 6mo 10d" (human-readable)
    public func friendlyString() -> String { ... }
}

// ============================================================================
// FORMAT — String interpolation-based formatting
// ============================================================================

// Format components for use in string interpolation.
// Bare names produce the ISO/numeric default. Name suffix = text form.
// All text output is English (no locale support in v1).
public enum FormatComponent {
    // Year
    case Year               // "2024" (4-digit)
    case ShortYear          // "24" (2-digit)

    // Month
    case Month              // "07" (zero-padded numeric)
    case MonthUnpadded      // "7" (unpadded numeric)
    case MonthName          // "July" (full English name)
    case ShortMonthName     // "Jul" (abbreviated)
    case NarrowMonthName    // "J" (single letter)

    // Day
    case Day                // "04" (zero-padded)
    case DayUnpadded        // "4" (unpadded)

    // Weekday
    case Weekday            // "Thursday" (full English name)
    case ShortWeekday       // "Thu" (abbreviated)
    case NarrowWeekday      // "T" (single letter)

    // Time
    case Hour               // "15" (24-hour, zero-padded — ISO default)
    case Hour12             // "3" (12-hour, unpadded)
    case Hour12Padded       // "03" (12-hour, zero-padded)
    case Minute             // "30" (zero-padded)
    case Second             // "05" (zero-padded)
    case Millisecond        // "123"
    case Microsecond        // "123456"
    case Nanosecond         // "123456789"
    case AmPm               // "PM"

    // Zone
    case TimeZoneName       // "EST" (short timezone name)
    case FullTimeZoneName   // "Eastern Standard Time"
    case Offset             // "-04:00"

    // Literal (populated by appendLiteral — not used directly in interpolation)
    case Literal(String)
}

// Accumulator for building Format values via string interpolation.
public struct FormatAccumulator: Interpolatable, Cloneable {
    var components: Array[FormatComponent]

    public init(literalCapacity literalCapacity: Int64, interpolationCount interpolationCount: Int64) {
        self.components = Array[FormatComponent]();
    }

    public func clone() -> FormatAccumulator {
        var c = FormatAccumulator(literalCapacity: 0, interpolationCount: 0);
        c.components = self.components.clone();
        c
    }

    public mutating func appendLiteral(literal: String) {
        if literal.isEmpty == false {
            self.components.append(.Literal(literal));
        }
    }

    // Accepts FormatComponent enum — enables .Year, .MonthName, etc. shorthand
    // in string interpolation targeting Format.
    public mutating func appendInterpolation(value: FormatComponent) {
        self.components.append(value);
    }
}

// A reusable format specification. Constructed via string interpolation
// or from preset static properties. Works for both formatting and parsing.
//
// Custom format via interpolation:
//   let fmt: Format = "\(.MonthName) \(.DayUnpadded), \(.Year)";
//   date.formatted(as: fmt);  // "July 4, 2024"
//
// Preset:
//   date.formatted(as: Format.isoDate);  // "2024-07-04"
public struct Format: ExpressibleByStringInterpolation, Cloneable {
    public type Interpolation = FormatAccumulator

    public var components: Array[FormatComponent]

    public init(components components: Array[FormatComponent]) {
        self.components = components;
    }

    public init(stringLiteral ptr: lang.ptr[lang.i8], length: lang.i64) {
        self.components = [.Literal(String(stringLiteral: ptr, length))];
    }

    public init(interpolation: FormatAccumulator) {
        self.components = interpolation.components;
    }

    public func clone() -> Format {
        Format(components: self.components.clone())
    }

    // --- Presets ---

    // "2024-07-04"
    public static var isoDate: Format {
        Format(components: [.Year, .Literal("-"), .Month, .Literal("-"), .Day])
    }

    // "15:30:05"
    public static var isoTime: Format {
        Format(components: [.Hour, .Literal(":"), .Minute, .Literal(":"), .Second])
    }

    // "2024-07-04T15:30:05"
    public static var isoDateTime: Format {
        Format(components: [
            .Year, .Literal("-"), .Month, .Literal("-"), .Day,
            .Literal("T"),
            .Hour, .Literal(":"), .Minute, .Literal(":"), .Second
        ])
    }

    // "2024-07-04T15:30:05Z" or "2024-07-04T15:30:05+00:00"
    public static var rfc3339: Format {
        Format(components: [
            .Year, .Literal("-"), .Month, .Literal("-"), .Day,
            .Literal("T"),
            .Hour, .Literal(":"), .Minute, .Literal(":"), .Second,
            .Offset
        ])
    }

    // "Thu, 04 Jul 2024 15:30:05 -0400"
    public static var rfc2822: Format {
        Format(components: [
            .ShortWeekday, .Literal(", "), .Day, .Literal(" "),
            .ShortMonthName, .Literal(" "), .Year, .Literal(" "),
            .Hour, .Literal(":"), .Minute, .Literal(":"), .Second,
            .Literal(" "), .Offset
        ])
    }

    // "2024-07-04T15:30:05-04:00[America/New_York]"
    public static var rfc9557: Format {
        Format(components: [
            .Year, .Literal("-"), .Month, .Literal("-"), .Day,
            .Literal("T"),
            .Hour, .Literal(":"), .Minute, .Literal(":"), .Second,
            .Offset, .Literal("["), .TimeZoneName, .Literal("]")
        ])
    }

}

// ============================================================================
// ERRORS
// ============================================================================

public enum DateError {
    case InvalidDate(year: Int64, month: Int64, day: Int64)
    case InvalidTime(hour: Int64, minute: Int64, second: Int64)
}

public enum ParseError {
    case InvalidFormat(String)
    case InvalidValue(String)
    case UnexpectedEnd
}

// ============================================================================
// USAGE EXAMPLES
// ============================================================================

// --- Basic construction ---
// let today = try Date(year: 2024, month: 7, day: 4);
// let christmas = Date.unchecked(year: 2024, month: 12, day: 25);
// let now = Instant.now();
// let meeting = try DateTime(year: 2024, month: 7, day: 4, hour: 14, minute: 30);

// --- Timezone conversion ---
// let tz = TimeZone.find("America/New_York")!;
// let zoned = meeting.toZoned(in: tz);                     // Compatible (default), never fails
// let utcInstant = zoned.instant;
// let inTokyo = zoned.inZone(TimeZone.find("Asia/Tokyo")!);

// --- Formatting (string interpolation) ---
// let fmt: Format = "\(.Year)-\(.Month)-\(.Day)";
// today.formatted(as: fmt);                                // "2024-07-04"
//
// let pretty: Format = "\(.MonthName) \(.DayUnpadded), \(.Year)";
// today.formatted(as: pretty);                             // "July 4, 2024"
//
// let full: Format = "\(.Weekday), \(.MonthName) \(.DayUnpadded), \(.Year) at \(.Hour12):\(.Minute) \(.AmPm) \(.TimeZoneName)";
// zoned.formatted(as: full);                               // "Thursday, July 4, 2024 at 2:30 PM EST"

// --- Formatting (presets) ---
// today.formatted(as: Format.isoDate);                     // "2024-07-04"
// zoned.formatted(as: Format.rfc3339);                     // "2024-07-04T18:30:05-04:00"
// zoned.formatted(as: Format.rfc9557);                     // "2024-07-04T14:30:05-04:00[America/New_York]"

// --- Formatting (default via Formattable / string interpolation) ---
// "\(today)";                                              // "2024-07-04"
// "\(now)";                                                // "2024-07-04T18:30:05Z"
// "\(zoned)";                                              // "2024-07-04T14:30:05-04:00[America/New_York]"

// --- Parsing ---
// let d = try Date.parse(from: "2024-07-04");
// let d = try Date.parse(from: "July 4, 2024", as: pretty);
// let i = try Instant.parse(from: "2024-07-04T18:30:05Z");

// --- Calendar arithmetic (no try needed — Clip never fails) ---
// let next = today.adding(months: 1);                      // 2024-08-04
// let end = today.adding(months: 1, days: -1);             // 2024-08-03
// Date.unchecked(year: 2024, month: 1, day: 31).adding(months: 1);                  // 2024-02-29 (Clip)
// Date.unchecked(year: 2024, month: 1, day: 31).adding(months: 1, overflow: .Rollover);  // 2024-03-02

// --- Exact arithmetic ---
// let later = now.advanced(by: Duration.hours(2));
// let elapsed = start.duration(to: end);
// let also = now + Duration.minutes(30);                   // operator

// --- Period ---
// let p = Period(years: 1, months: 6);
// let future = today.adding(period: p);
// let diff = today.period(to: future);                     // Period(years: 1, months: 6, days: 0)
// let biweekly = Period.weeks(2);                          // stores days: 14 internally
// biweekly.days;                                           // 14
// biweekly.weeks;                                          // 2 (computed)
// Period(days: 14) == Period(weeks: 2);                    // true
// p.friendlyString();                                      // "1y 6mo"
// p.isoString();                                           // "P1Y6M"

// --- Duration ---
// let timeout = Duration.minutes(90);
// timeout.totalSeconds;                                    // 5400
// timeout.friendlyString();                                // "1h 30m"
// timeout.isoString();                                     // "PT1H30M"
// "\(timeout)";                                            // "PT1H30M" (Formattable default)
// let half = timeout.multiplied(by: 0.5);                  // 45 minutes
// let d = Duration.hours(1) + Duration.minutes(30);        // operator

// --- DST handling ---
// let tz = TimeZone.find("America/New_York")!;
// let dt = try DateTime(year: 2024, month: 11, day: 3, hour: 1, minute: 30);
// dt.isAmbiguous(in: tz);                                  // true (fall-back fold)
// let zdt = dt.toZoned(in: tz);                            // Compatible: picks earlier
// let zdt = dt.toZoned(in: tz, disambiguation: .Later);    // picks later

// --- Time wrapping ---
// let t = try Time(hour: 23, minute: 30);
// let t2 = t.advanced(by: Duration.hours(2));              // 01:30 (wraps)
// let (t3, days) = t.advancedWithOverflow(by: Duration.hours(2));  // (01:30, 1)

// --- Clock injection for testing ---
// let clock = FakeClock(at: Instant(secondsSinceEpoch: 1_000_000));
// let now = Instant.now(from: clock);
// clock.advance(by: Duration.hours(1));
// let later = Instant.now(from: clock);
// let today = Date.today(from: clock);

// --- Today (system timezone by default) ---
// let today = Date.today();                                // system timezone
// let utcToday = Date.today(in: TimeZone.utc);             // explicit UTC
