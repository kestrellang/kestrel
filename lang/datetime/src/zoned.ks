module datetime

// DateTime + IANA timezone. The "full" type.
// Stores an Instant + TimeZone. Civil components computed on demand.
// Equality and hashing include the zone; ordering is by instant only, so two
// values can be neither < nor > yet not == (use `isSameInstant(as:)`).
// Default Formattable output: RFC 9557 ("2024-07-04T15:30:05-04:00[America/New_York]").
public struct ZonedDateTime: Equatable, Comparable, Hashable, Formattable, Cloneable {
    var inst: Instant
    var tz: TimeZone

    // --- Internal construction ---

    static func fromDateTime(dt: DateTime, in zone: TimeZone,
                              disambiguation d: Disambiguation = .Compatible) -> ZonedDateTime {
        let (naiveSecs, nanos) = dt.toEpochSecs();
        let epoch = zone.resolveEpochSecs(dateTime: dt, naiveSecs: naiveSecs, disambiguation: d);
        ZonedDateTime(instant: Instant(secondsSinceEpoch: epoch, nanoseconds: nanos), in: zone)
    }

    // --- Public construction ---

    public init(year year: Int64, month month: Int64, day day: Int64,
                in zone: TimeZone,
                hour hour: Int64 = 0, minute minute: Int64 = 0,
                second second: Int64 = 0, nanosecond nanosecond: Int64 = 0,
                disambiguation d: Disambiguation = .Compatible) throws DateTimeError {
        // Validate via match/throw rather than `try`: a `try` early-return is
        // rejected by definite-init before all fields are set, but an explicit
        // `throw` (unwind) is allowed.
        let dt = match DateTime(year: year, month: month, day: day,
                                hour: hour, minute: minute, second: second, nanosecond: nanosecond) {
            .Ok(v) => v,
            .Err(e) => throw e,
        };
        let zdt = ZonedDateTime.fromDateTime(dt, in: zone, disambiguation: d);
        self.inst = zdt.inst;
        self.tz = zdt.tz;
    }

    public init(instant instant: Instant, in zone: TimeZone) {
        self.inst = instant;
        self.tz = zone;
    }

    public init(dateTime dateTime: DateTime, in zone: TimeZone,
                disambiguation d: Disambiguation = .Compatible) {
        let zdt = ZonedDateTime.fromDateTime(dateTime, in: zone, disambiguation: d);
        self.inst = zdt.inst;
        self.tz = zdt.tz;
    }

    // --- Now ---

    public static func now() -> ZonedDateTime {
        Instant.now().toZoned(in: TimeZone.system())
    }

    public static func now(in zone: TimeZone) -> ZonedDateTime {
        Instant.now().toZoned(in: zone)
    }

    // --- Properties ---

    public var instant: Instant { self.inst }
    public var timeZone: TimeZone { self.tz }

    public var dateTime: DateTime { self.inst.toDateTime(in: self.tz) }
    public var date: Date { self.inst.toDate(in: self.tz) }
    public var time: Time { self.inst.toTime(in: self.tz) }

    public var year: Int64 { self.dateTime.year }
    public var month: Int64 { self.dateTime.month }
    public var day: Int64 { self.dateTime.day }
    public var hour: Int64 { self.dateTime.hour }
    public var minute: Int64 { self.dateTime.minute }
    public var second: Int64 { self.dateTime.second }
    public var nanosecond: Int64 { self.dateTime.nanosecond }
    public var weekday: Weekday { self.dateTime.weekday }
    public var dayOfYear: Int64 { self.dateTime.dayOfYear }

    // UTC offset in effect at this instant, in seconds (east of UTC is positive).
    public var offsetSeconds: Int64 { self.tz.offsetAt(self.inst.secs) }

    // --- Field replacement ---

    // A copy with the given wall-clock fields replaced, re-resolved in this
    // zone; fails if the civil result is invalid.
    public func with(year year: Int64? = .None, month month: Int64? = .None, day day: Int64? = .None,
                     hour hour: Int64? = .None, minute minute: Int64? = .None,
                     second second: Int64? = .None, nanosecond nanosecond: Int64? = .None,
                     disambiguation d: Disambiguation = .Compatible) -> ZonedDateTime throws DateTimeError {
        let dt = try self.dateTime.with(year: year, month: month, day: day, hour: hour,
                                        minute: minute, second: second, nanosecond: nanosecond);
        .Ok(ZonedDateTime.fromDateTime(dt, in: self.tz, disambiguation: d))
    }

    // --- Exact Arithmetic ---

    public func advanced(by duration: Duration) -> ZonedDateTime {
        ZonedDateTime(instant: self.inst.advanced(by: duration), in: self.tz)
    }

    // --- Calendar Arithmetic ---

    // Preserves wall-clock time across DST. Uses Compatible disambiguation.
    public func adding(years y: Int64 = 0, months m: Int64 = 0, days d: Int64 = 0,
                       overflow o: Overflow = .Clip) -> ZonedDateTime {
        let newDt = self.dateTime.adding(years: y, months: m, days: d, overflow: o);
        ZonedDateTime.fromDateTime(newDt, in: self.tz, disambiguation: .Compatible)
    }

    public func adding(period p: Period, overflow o: Overflow = .Clip) -> ZonedDateTime {
        self.adding(years: p.years, months: p.months, days: p.days, overflow: o)
    }

    // --- Navigation ---

    // Boundaries are the wall-clock boundaries of DateTime, resolved in this zone.
    public func startOfDay() -> ZonedDateTime {
        ZonedDateTime.fromDateTime(self.dateTime.startOfDay(), in: self.tz, disambiguation: .Compatible)
    }

    public func endOfDay() -> ZonedDateTime {
        ZonedDateTime.fromDateTime(self.dateTime.endOfDay(), in: self.tz, disambiguation: .Compatible)
    }

    public func startOfMonth() -> ZonedDateTime {
        ZonedDateTime.fromDateTime(self.dateTime.startOfMonth(), in: self.tz, disambiguation: .Compatible)
    }

    public func endOfMonth() -> ZonedDateTime {
        ZonedDateTime.fromDateTime(self.dateTime.endOfMonth(), in: self.tz, disambiguation: .Compatible)
    }

    public func startOfYear() -> ZonedDateTime {
        ZonedDateTime.fromDateTime(self.dateTime.startOfYear(), in: self.tz, disambiguation: .Compatible)
    }

    public func endOfYear() -> ZonedDateTime {
        ZonedDateTime.fromDateTime(self.dateTime.endOfYear(), in: self.tz, disambiguation: .Compatible)
    }

    public func tomorrow() -> ZonedDateTime {
        self.adding(years: 0, months: 0, days: 1)
    }

    public func yesterday() -> ZonedDateTime {
        self.adding(years: 0, months: 0, days: -1)
    }

    // --- Difference ---

    public func duration(to other: ZonedDateTime) -> Duration {
        self.inst.duration(to: other.inst)
    }

    // Calendar difference between the two dates, both read in this value's
    // zone; the time of day is ignored (as DateTime.period(to:)).
    public func period(to other: ZonedDateTime) -> Period {
        self.dateTime.period(to: other.toZoned(in: self.tz).dateTime)
    }

    // --- Timezone Conversion ---

    // Same instant, viewed in a different time zone (civil fields recomputed).
    public func toZoned(in zone: TimeZone) -> ZonedDateTime {
        ZonedDateTime(instant: self.inst, in: zone)
    }

    // --- Rounding ---

    public func rounded(to unit: TimeUnit, mode mode: RoundMode = .HalfExpand) -> ZonedDateTime {
        ZonedDateTime(instant: self.inst.rounded(to: unit, mode: mode), in: self.tz)
    }

    // --- Parsing ---

    // Parse RFC 9557: "2024-07-04T09:00:00.5-04:00[America/New_York]". The
    // bracketed zone is required; the offset is optional and, when present,
    // must be one the zone actually has at that wall time (it selects the side
    // of a DST fold).
    public static func parse(from input: String) -> ZonedDateTime throws ParseError {
        let bytes: Array[UInt8] = Array(from: input.bytes);
        guard bytes.count > 19 and bytes(bytes.count - 1) == 93 else {
            throw ParseError.InvalidFormat("expected '[zone]' suffix");
        }
        var open: Int64 = 19;
        while open < bytes.count and bytes(open) != 91 { open = open + 1; }
        guard open < bytes.count else { throw ParseError.InvalidFormat("expected '['"); }
        let zoneName = stringFromBytes(bytes, open + 1, bytes.count - open - 2);

        let offStart = findOffsetStart(bytes, open);
        let dt = try DateTime.parse(from: stringFromBytes(bytes, 0, offStart));
        if offStart == open {
            return ZonedDateTime.resolveParsed(dt, zoneName: .Some(zoneName), offset: .None);
        }
        let (off, len) = try parseOffset(bytes, offStart);
        guard offStart + len == open else { throw ParseError.InvalidFormat("expected '[' after offset"); }
        ZonedDateTime.resolveParsed(dt, zoneName: .Some(zoneName), offset: .Some(off))
    }

    // Shared zone resolution for every ZonedDateTime parser. A zone name wins;
    // an offset alongside it must match the zone at that instant. An offset
    // alone yields a fixed-offset zone; neither yields UTC.
    static func resolveParsed(dt: DateTime, zoneName zoneName: String?,
                              offset offset: Int64?) -> ZonedDateTime throws ParseError {
        if let .Some(name) = zoneName {
            guard let .Some(zone) = TimeZone(name) else {
                throw ParseError.InvalidValue("unknown time zone '\(name)'");
            }
            guard let .Some(off) = offset else {
                return .Ok(ZonedDateTime.fromDateTime(dt, in: zone, disambiguation: .Compatible));
            }
            let (naiveSecs, nanos) = dt.toEpochSecs();
            let epoch = naiveSecs - off;
            guard zone.offsetAt(epoch) == off else {
                throw ParseError.InvalidValue("offset does not match time zone '\(name)'");
            }
            return .Ok(ZonedDateTime(instant: Instant(secondsSinceEpoch: epoch, nanoseconds: nanos), in: zone));
        }
        guard let .Some(off) = offset else {
            return .Ok(ZonedDateTime.fromDateTime(dt, in: TimeZone.utc, disambiguation: .Compatible));
        }
        guard let .Some(fixed) = TimeZone(offsetSeconds: off) else {
            throw ParseError.InvalidValue("unsupported offset");
        }
        .Ok(ZonedDateTime.fromDateTime(dt, in: fixed, disambiguation: .Compatible))
    }

    // --- Protocol conformances ---

    // Same instant AND same zone. For instant-only equality use isSameInstant(as:).
    public func isEqual(to other: ZonedDateTime) -> Bool {
        self.inst.isEqual(to: other.inst) and self.tz.isEqual(to: other.tz)
    }

    // True if both refer to the same point in time, whatever their zones.
    public func isSameInstant(as other: ZonedDateTime) -> Bool {
        self.inst.isEqual(to: other.inst)
    }

    public func compare(other: ZonedDateTime) -> Ordering {
        self.inst.compare(other.inst)
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.inst.hash(into: hasher);
        self.tz.hash(into: hasher);
    }

    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        // RFC 9557: "2024-07-04T15:30:05-04:00[America/New_York]"
        self.dateTime.format(into: writer);
        appendOffset(into: writer, self.offsetSeconds);
        writer.append("[");
        writer.append(self.tz.name);
        writer.append("]");
    }

    public func clone() -> ZonedDateTime {
        ZonedDateTime(instant: self.inst.clone(), in: self.tz)
    }
}
