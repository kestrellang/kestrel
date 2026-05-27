module datetime

import std.memory.(Pointer)

// Absolute point in time. Nanosecond precision. No calendar, no timezone.
// Internal: epoch seconds + nanosecond component since 1970-01-01T00:00:00Z.
// Default Formattable output: RFC 3339 ("2024-07-04T15:30:05Z").
public struct Instant: Equatable, Comparable, Hashable, Formattable, Cloneable {
    var secs: Int64
    var nanos: Int64

    // --- Construction ---

    public static func now() -> Instant {
        var s: Int64 = 0;
        var n: Int64 = 0;
        kestrel_clock_gettime(Pointer(to: s), Pointer(to: n));
        Instant.raw(secs: s, nanos: n)
    }

    public init(secondsSinceEpoch seconds: Int64, nanoseconds nanoseconds: Int64 = 0) {
        let d = Duration.normalized(secs: seconds, nanos: nanoseconds);
        self.secs = d.secs;
        self.nanos = d.nanos;
    }

    public init(millisecondsSinceEpoch ms: Int64) {
        self.secs = ms / 1000;
        self.nanos = (ms % 1000) * 1_000_000;
    }

    static func raw(secs secs: Int64, nanos nanos: Int64) -> Instant {
        var i = Instant(secondsSinceEpoch: 0);
        i.secs = secs;
        i.nanos = nanos;
        i
    }

    // --- Properties ---

    public var secondsSinceEpoch: Int64 { self.secs }
    public var millisecondsSinceEpoch: Int64 { self.secs * 1000 + self.nanos / 1_000_000 }
    public var nanosecondsSinceEpoch: Int64 { self.secs * 1_000_000_000 + self.nanos }
    public var subsecondNanoseconds: Int64 { self.nanos }

    // --- Arithmetic ---

    public func advanced(by duration: Duration) -> Instant {
        let d = Duration.normalized(secs: self.secs + duration.secs, nanos: self.nanos + duration.nanos);
        Instant.raw(secs: d.secs, nanos: d.nanos)
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
        ZonedDateTime.fromInstant(self, in: zone)
    }

    // --- Rounding ---

    public func rounded(to unit: TimeUnit, mode mode: RoundMode = .HalfExpand) -> Instant {
        let divisor = unit.nanoseconds();
        if divisor <= 1 { return self }
        let total = self.nanosecondsSinceEpoch;
        let rounded = roundValue(total, divisor, mode);
        Instant(secondsSinceEpoch: 0, nanoseconds: rounded)
    }

    // --- Parsing ---

    public static func parse(from input: String) -> Instant throws ParseError {
        // Parse RFC 3339: YYYY-MM-DDTHH:MM:SSZ or YYYY-MM-DDTHH:MM:SS±HH:MM
        let bytes = input.utf8;
        guard bytes.count >= 20 else { throw ParseError.UnexpectedEnd; }

        let year = try parseDigits(bytes, 0, 4);
        guard bytes(4) == 45 else { throw ParseError.InvalidFormat("expected '-'"); }
        let month = try parseDigits(bytes, 5, 2);
        guard bytes(7) == 45 else { throw ParseError.InvalidFormat("expected '-'"); }
        let day = try parseDigits(bytes, 8, 2);
        let sep = bytes(10);
        guard sep == 84 or sep == 32 else { throw ParseError.InvalidFormat("expected 'T'"); }
        let hour = try parseDigits(bytes, 11, 2);
        guard bytes(13) == 58 else { throw ParseError.InvalidFormat("expected ':'"); }
        let minute = try parseDigits(bytes, 14, 2);
        guard bytes(16) == 58 else { throw ParseError.InvalidFormat("expected ':'"); }
        let second = try parseDigits(bytes, 17, 2);

        // Parse fractional seconds if present
        var nanos: Int64 = 0;
        var pos: Int64 = 19;
        if pos < bytes.count and bytes(pos) == 46 {
            pos = pos + 1;
            var scale: Int64 = 100_000_000;
            while pos < bytes.count and scale > 0 {
                let b = Int64(from: bytes(pos));
                if b < 48 or b > 57 { break; }
                nanos = nanos + (b - 48) * scale;
                scale = scale / 10;
                pos = pos + 1;
            }
        }

        // Parse timezone offset
        var offsetSecs: Int64 = 0;
        if pos < bytes.count {
            let zc = bytes(pos);
            if zc == 90 or zc == 122 {
                // 'Z' or 'z'
                offsetSecs = 0;
            } else if zc == 43 or zc == 45 {
                // '+' or '-'
                guard pos + 5 < bytes.count else { throw ParseError.UnexpectedEnd; }
                let offH = try parseDigits(bytes, pos + 1, 2);
                // Skip optional colon
                var offMPos = pos + 3;
                if offMPos < bytes.count and bytes(offMPos) == 58 {
                    offMPos = offMPos + 1;
                }
                let offM = try parseDigits(bytes, offMPos, 2);
                offsetSecs = offH * 3600 + offM * 60;
                if zc == 45 { offsetSecs = 0 - offsetSecs; }
            }
        }

        guard isValidDate(year, month, day) else {
            throw ParseError.InvalidValue("invalid date");
        }

        let epochDay = daysToCivil(year, month, day);
        let epochSec = epochDay * 86400 + hour * 3600 + minute * 60 + second - offsetSecs;
        Instant.raw(secs: epochSec, nanos: nanos)
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

    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        // RFC 3339 UTC: "2024-07-04T15:30:05Z"
        let dt = self.toDateTime(in: TimeZone.utc);
        dt.dateVal.format(into: writer);
        writer.append("T");
        dt.timeVal.format(into: writer);
        if self.nanos > 0 {
            writer.append(".");
            appendFractional(into: writer, self.nanos);
        }
        writer.append("Z");
    }

    public func clone() -> Instant {
        Instant.raw(secs: self.secs, nanos: self.nanos)
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
