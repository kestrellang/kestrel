module datetime

// Date + Time without timezone. "What the wall clock shows."
// Default Formattable output: ISO 8601 ("2024-07-04T15:30:05").
public struct DateTime: Equatable, Comparable, Hashable, Formattable, Cloneable {
    internal var dateVal: Date
    internal var timeVal: Time

    // --- Construction ---

    public init(year year: Int64, month month: Int64, day day: Int64,
                hour hour: Int64 = 0, minute minute: Int64 = 0,
                second second: Int64 = 0, nanosecond nanosecond: Int64 = 0) throws DateError {
        // Validate both components via match/throw before assigning any field:
        // definite-init rejects a `try` early-return while fields are unset,
        // but an explicit `throw` (unwind) is allowed.
        let d = match Date(year: year, month: month, day: day) {
            .Ok(v) => v,
            .Err(e) => throw e,
        };
        let t = match Time(hour: hour, minute: minute, second: second, nanosecond: nanosecond) {
            .Ok(v) => v,
            .Err(e) => throw e,
        };
        self.dateVal = d;
        self.timeVal = t;
    }

    public init(date date: Date, time time: Time) {
        self.dateVal = date;
        self.timeVal = time;
    }

    // Internal: construct from epoch seconds + nanoseconds (UTC)
    static func fromEpochSecs(secs: Int64, nanos: Int64) -> DateTime {
        var daySecs = secs;
        var dayNum = floorDiv(daySecs, 86400);
        var secOfDay = floorMod(daySecs, 86400);
        let date = dateFromEpochDay(dayNum);
        let time = Time.fromNanos(secOfDay * Time.NANOS_PER_SECOND + nanos);
        DateTime(date: date, time: time)
    }

    // Convert to epoch seconds + nanosecond component (UTC, no timezone)
    func toEpochSecs() -> (Int64, Int64) {
        let dayNum = daysToCivil(self.dateVal.y, self.dateVal.m, self.dateVal.d);
        let secs = dayNum * 86400 +
                   self.timeVal.hour * 3600 +
                   self.timeVal.minute * 60 +
                   self.timeVal.second;
        (secs, self.timeVal.nanosecond)
    }

    // --- Properties ---

    public var date: Date { self.dateVal }
    public var time: Time { self.timeVal }
    public var year: Int64 { self.dateVal.year }
    public var month: Int64 { self.dateVal.month }
    public var day: Int64 { self.dateVal.day }
    public var hour: Int64 { self.timeVal.hour }
    public var minute: Int64 { self.timeVal.minute }
    public var second: Int64 { self.timeVal.second }
    public var nanosecond: Int64 { self.timeVal.nanosecond }
    public var weekday: Weekday { self.dateVal.weekday }
    public var dayOfYear: Int64 { self.dateVal.dayOfYear }

    // --- Exact Arithmetic ---

    public func advanced(by duration: Duration) -> DateTime {
        let (newTime, days) = self.timeVal.advancedWithOverflow(by: duration);
        let newDate = self.dateVal.adding(years: 0, months: 0, days: days);
        DateTime(date: newDate, time: newTime)
    }

    // --- Calendar Arithmetic ---

    public func adding(years y: Int64 = 0, months m: Int64 = 0, days d: Int64 = 0,
                       overflow o: Overflow = .Clip) -> DateTime {
        let newDate = self.dateVal.adding(years: y, months: m, days: d, overflow: o);
        DateTime(date: newDate, time: self.timeVal)
    }

    public func adding(period p: Period, overflow o: Overflow = .Clip) -> DateTime {
        let newDate = self.dateVal.adding(period: p, overflow: o);
        DateTime(date: newDate, time: self.timeVal)
    }

    // --- Navigation ---

    public func tomorrow() -> DateTime {
        DateTime(date: self.dateVal.tomorrow(), time: self.timeVal)
    }

    public func yesterday() -> DateTime {
        DateTime(date: self.dateVal.yesterday(), time: self.timeVal)
    }

    public func startOfDay() -> DateTime {
        DateTime(date: self.dateVal, time: Time.midnight)
    }

    public func endOfDay() -> DateTime {
        DateTime(date: self.dateVal, time: Time.fromNanos(Time.NANOS_PER_DAY - 1))
    }

    public func startOfMonth() -> DateTime {
        DateTime(date: self.dateVal.startOfMonth(), time: self.timeVal)
    }

    public func endOfMonth() -> DateTime {
        DateTime(date: self.dateVal.endOfMonth(), time: self.timeVal)
    }

    public func startOfYear() -> DateTime {
        DateTime(date: self.dateVal.startOfYear(), time: self.timeVal)
    }

    public func endOfYear() -> DateTime {
        DateTime(date: self.dateVal.endOfYear(), time: self.timeVal)
    }

    // --- Difference ---

    public func duration(to other: DateTime) -> Duration {
        let (selfSecs, selfNanos) = self.toEpochSecs();
        let (otherSecs, otherNanos) = other.toEpochSecs();
        Duration(seconds: otherSecs - selfSecs, nanoseconds: otherNanos - selfNanos)
    }

    public func period(to other: DateTime) -> Period {
        self.dateVal.period(to: other.dateVal)
    }

    // --- Query ---

    public func isAmbiguous(in zone: TimeZone) -> Bool {
        zone.isAmbiguous(dateTime: self)
    }

    // --- Conversion ---

    public func toZoned(in zone: TimeZone,
                        disambiguation d: Disambiguation = .Compatible) -> ZonedDateTime {
        ZonedDateTime.fromDateTime(self, in: zone, disambiguation: d)
    }

    public func toInstant(in zone: TimeZone,
                          disambiguation d: Disambiguation = .Compatible) -> Instant {
        self.toZoned(in: zone, disambiguation: d).instant
    }

    // --- Parsing ---

    public static func parse(from input: String) -> DateTime throws ParseError {
        // Parse ISO 8601: YYYY-MM-DDTHH:MM:SS
        let bytes: Array[UInt8] = Array(from: input.bytes);
        guard bytes.count >= 19 else { throw ParseError.UnexpectedEnd; }
        let date = try Date.parse(from: input.bytes.substring(0..<10));
        // Expect 'T' or ' ' separator
        let sep = bytes(10);
        guard sep == 84 or sep == 32 else {
            throw ParseError.InvalidFormat("expected 'T' or space at position 10");
        }
        let time = try Time.parse(from: input.bytes.substring(11..<bytes.count));
        DateTime(date: date, time: time)
    }

    // --- Protocol conformances ---

    public func isEqual(to other: DateTime) -> Bool {
        self.dateVal.isEqual(to: other.dateVal) and self.timeVal.isEqual(to: other.timeVal)
    }

    public func compare(other: DateTime) -> Ordering {
        self.dateVal.compare(other.dateVal).then(self.timeVal.compare(other.timeVal))
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.dateVal.hash(into: hasher);
        self.timeVal.hash(into: hasher);
    }

    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        self.dateVal.format(into: writer);
        writer.append("T");
        self.timeVal.format(into: writer);
    }

    public func clone() -> DateTime {
        DateTime(date: self.dateVal.clone(), time: self.timeVal.clone())
    }
}
