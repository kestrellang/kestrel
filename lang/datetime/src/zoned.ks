module datetime

// DateTime + IANA timezone. The "full" type.
// Stores an Instant + TimeZone. Civil components computed on demand.
// Compared by underlying instant (timezone ignored for ordering).
// Default Formattable output: RFC 9557 ("2024-07-04T15:30:05-04:00[America/New_York]").
public struct ZonedDateTime: Equatable, Comparable, Hashable, Formattable, Cloneable {
    var inst: Instant
    var tz: TimeZone

    // --- Internal construction ---

    static func fromDateTime(dt: DateTime, in zone: TimeZone,
                              disambiguation d: Disambiguation = .Compatible) -> ZonedDateTime {
        let (naiveSecs, nanos) = dt.toEpochSecs();
        // First guess: subtract the offset at the naive epoch
        let offset1 = zone.offsetAt(naiveSecs);
        let epoch1 = naiveSecs - offset1;
        let actualOffset1 = zone.offsetAt(epoch1);

        if actualOffset1 == offset1 {
            // Clean match — no DST transition issue
            return ZonedDateTime(instant: Instant.raw(secs: epoch1, nanos: nanos), in: zone);
        }

        // We're near a transition. Try the actual offset.
        let epoch2 = naiveSecs - actualOffset1;
        let actualOffset2 = zone.offsetAt(epoch2);

        if actualOffset2 == actualOffset1 {
            // Second guess is consistent
            return ZonedDateTime(instant: Instant.raw(secs: epoch2, nanos: nanos), in: zone);
        }

        // Ambiguous or gap — apply disambiguation
        let earlierEpoch = if epoch1 < epoch2 { epoch1 } else { epoch2 };
        let laterEpoch = if epoch1 < epoch2 { epoch2 } else { epoch1 };

        match d {
            .Compatible => {
                // Gap: pick later. Fold: pick earlier.
                if actualOffset1 > offset1 {
                    // Spring forward (gap) — pick later
                    ZonedDateTime(instant: Instant.raw(secs: laterEpoch, nanos: nanos), in: zone)
                } else {
                    // Fall back (fold) — pick earlier
                    ZonedDateTime(instant: Instant.raw(secs: earlierEpoch, nanos: nanos), in: zone)
                }
            },
            .Earlier => ZonedDateTime(instant: Instant.raw(secs: earlierEpoch, nanos: nanos), in: zone),
            .Later => ZonedDateTime(instant: Instant.raw(secs: laterEpoch, nanos: nanos), in: zone)
        }
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

    // Current UTC offset in seconds
    var offsetSeconds: Int64 { self.tz.offsetAt(self.inst.secs) }

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

    public func startOfDay() -> ZonedDateTime {
        let dt = self.dateTime.startOfDay();
        ZonedDateTime.fromDateTime(dt, in: self.tz, disambiguation: .Compatible)
    }

    public func endOfDay() -> ZonedDateTime {
        let dt = self.dateTime.endOfDay();
        ZonedDateTime.fromDateTime(dt, in: self.tz, disambiguation: .Compatible)
    }

    public func startOfMonth() -> ZonedDateTime {
        let dt = self.dateTime.startOfMonth().startOfDay();
        ZonedDateTime.fromDateTime(dt, in: self.tz, disambiguation: .Compatible)
    }

    public func endOfMonth() -> ZonedDateTime {
        let dt = self.dateTime.endOfMonth();
        ZonedDateTime.fromDateTime(dt, in: self.tz, disambiguation: .Compatible)
    }

    public func startOfYear() -> ZonedDateTime {
        let dt = self.dateTime.startOfYear().startOfDay();
        ZonedDateTime.fromDateTime(dt, in: self.tz, disambiguation: .Compatible)
    }

    public func endOfYear() -> ZonedDateTime {
        let dt = self.dateTime.endOfYear();
        ZonedDateTime.fromDateTime(dt, in: self.tz, disambiguation: .Compatible)
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

    // --- Timezone Conversion ---

    // Same instant, viewed in a different time zone (civil fields recomputed).
    public func inTimeZone(zone: TimeZone) -> ZonedDateTime {
        ZonedDateTime(instant: self.inst, in: zone)
    }

    // --- Rounding ---

    public func rounded(to unit: TimeUnit, mode mode: RoundMode = .HalfExpand) -> ZonedDateTime {
        ZonedDateTime(instant: self.inst.rounded(to: unit, mode: mode), in: self.tz)
    }

    // --- Protocol conformances ---

    // Compared by underlying instant
    public func isEqual(to other: ZonedDateTime) -> Bool {
        self.inst.isEqual(to: other.inst)
    }

    public func compare(other: ZonedDateTime) -> Ordering {
        self.inst.compare(other.inst)
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.inst.hash(into: hasher);
    }

    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        // RFC 9557: "2024-07-04T15:30:05-04:00[America/New_York]"
        let dt = self.dateTime;
        dt.dateVal.format(into: writer);
        writer.append("T");
        dt.timeVal.format(into: writer);
        // Offset
        let off = self.offsetSeconds;
        if off == 0 {
            writer.append("Z");
        } else {
            let sign = if off >= 0 { "+" } else { "-" };
            let absOff = off.abs();
            let h = absOff / 3600;
            let m = (absOff % 3600) / 60;
            writer.append(sign);
            appendPadded(into: writer, h, 2);
            writer.append(":");
            appendPadded(into: writer, m, 2);
        }
        writer.append("[");
        writer.append(self.tz.name);
        writer.append("]");
    }

    public func clone() -> ZonedDateTime {
        ZonedDateTime(instant: self.inst.clone(), in: self.tz)
    }
}
