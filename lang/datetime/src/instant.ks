module datetime

import std.memory.(Pointer)

// Absolute point in time. Nanosecond precision. No calendar, no timezone.
// Internal: epoch seconds + nanosecond component since 1970-01-01T00:00:00Z.
// Invariant: `secs` is floored and `nanos` is in [0, 1e9), so every instant
// has exactly one representation (-0.5s is secs -1, nanos 500_000_000).
// Default Formattable output: RFC 3339 ("2024-07-04T15:30:05Z").
public struct Instant: Equatable, Comparable, Hashable, Formattable, Cloneable {
    var secs: Int64
    var nanos: Int64

    // --- Construction ---

    public static func now() -> Instant {
        var s: Int64 = 0;
        var n: Int64 = 0;
        kestrel_clock_gettime(Pointer(to: s), Pointer(to: n));
        Instant(secondsSinceEpoch: s, nanoseconds: n)
    }

    // The single constructor that establishes the invariant: any `nanoseconds`
    // (negative or >= 1e9) is folded into the floored seconds.
    public init(secondsSinceEpoch seconds: Int64, nanoseconds nanoseconds: Int64 = 0) {
        self.secs = seconds + floorDiv(nanoseconds, Duration.NANOS_PER_SEC);
        self.nanos = floorMod(nanoseconds, Duration.NANOS_PER_SEC);
    }

    public init(millisecondsSinceEpoch ms: Int64) {
        self.secs = floorDiv(ms, 1000);
        self.nanos = floorMod(ms, 1000) * 1_000_000;
    }

    // --- Properties ---

    // Whole seconds, floored: -0.5s reports -1.
    public var secondsSinceEpoch: Int64 { self.secs }
    public var millisecondsSinceEpoch: Int64 { self.secs * 1000 + self.nanos / 1_000_000 }
    public var nanosecondsSinceEpoch: Int64 { self.secs * 1_000_000_000 + self.nanos }
    public var subsecondNanoseconds: Int64 { self.nanos }

    // --- Arithmetic ---

    public func advanced(by duration: Duration) -> Instant {
        Instant(secondsSinceEpoch: self.secs + duration.secs, nanoseconds: self.nanos + duration.nanos)
    }

    // --- Difference ---

    public func duration(to other: Instant) -> Duration {
        Duration(seconds: other.secs - self.secs, nanoseconds: other.nanos - self.nanos)
    }

    // --- Conversion ---

    public func toDate(in zone: TimeZone) -> Date {
        let offset = Int64(from: kestrel_tz_offset(zone.id, self.secs));
        let localSecs = self.secs + offset;
        dateFromEpochDay(floorDiv(localSecs, 86400))
    }

    public func toTime(in zone: TimeZone) -> Time {
        let offset = Int64(from: kestrel_tz_offset(zone.id, self.secs));
        let localSecs = self.secs + offset;
        let secOfDay = floorMod(localSecs, 86400);
        Time.fromNanos(secOfDay * Time.NANOS_PER_SECOND + self.nanos)
    }

    public func toDateTime(in zone: TimeZone) -> DateTime {
        let offset = Int64(from: kestrel_tz_offset(zone.id, self.secs));
        let localSecs = self.secs + offset;
        DateTime.fromEpochSecs(localSecs, self.nanos)
    }

    public func toZoned(in zone: TimeZone) -> ZonedDateTime {
        ZonedDateTime(instant: self, in: zone)
    }

    // --- Rounding ---

    public func rounded(to unit: TimeUnit, mode mode: RoundMode = .HalfExpand) -> Instant {
        let divisor = unit.nanoseconds;
        if divisor <= 1 { return self }
        let total = self.nanosecondsSinceEpoch;
        let rounded = roundValue(total, divisor, mode);
        Instant(secondsSinceEpoch: 0, nanoseconds: rounded)
    }

    // --- Parsing ---

    // Parse RFC 3339: "2024-07-04T15:30:05Z", "2024-07-04T15:30:05.25-04:00".
    // The offset is required — without one the text names no instant.
    public static func parse(from input: String) -> Instant throws ParseError {
        let bytes: Array[UInt8] = Array(from: input.bytes);
        let offStart = findOffsetStart(bytes, bytes.count);
        guard offStart < bytes.count else { throw ParseError.InvalidFormat("expected offset"); }
        let dt = try DateTime.parse(from: stringFromBytes(bytes, 0, offStart));
        let (off, len) = try parseOffset(bytes, offStart);
        guard offStart + len == bytes.count else {
            throw ParseError.InvalidFormat("unexpected trailing input at position \(offStart + len)");
        }
        let (secs, nanos) = dt.toEpochSecs();
        .Ok(Instant(secondsSinceEpoch: secs - off, nanoseconds: nanos))
    }

    // --- Protocol conformances ---

    public func isEqual(to other: Instant) -> Bool {
        self.secs == other.secs and self.nanos == other.nanos
    }

    public func compare(other: Instant) -> Ordering {
        self.secs.compare(other.secs).then(self.nanos.compare(other.nanos))
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.secs.hash(into: hasher);
        self.nanos.hash(into: hasher);
    }

    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        // RFC 3339 UTC: "2024-07-04T15:30:05Z"
        self.toDateTime(in: TimeZone.utc).format(into: writer);
        writer.append("Z");
    }

    public func clone() -> Instant {
        Instant(secondsSinceEpoch: self.secs, nanoseconds: self.nanos)
    }
}

// Instant operators
extend Instant: Addable[Duration] {
    type Output = Instant
    public static var zero: Instant { Instant(secondsSinceEpoch: 0) }

    public consuming func add(consuming other: Duration) -> Instant {
        self.advanced(by: other)
    }
}

extend Instant: Subtractable[Duration] {
    type Output = Instant

    public consuming func subtract(consuming other: Duration) -> Instant {
        self.advanced(by: other.negated())
    }
}

// Instant - Instant -> Duration
extend Instant: Subtractable[Instant] {
    type Output = Duration

    public consuming func subtract(consuming other: Instant) -> Duration {
        other.duration(to: self)
    }
}
