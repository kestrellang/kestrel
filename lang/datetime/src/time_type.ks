module datetime

// Wall-clock time of day. Nanosecond precision.
// Range: [00:00:00.000000000, 23:59:59.999999999].
// Arithmetic wraps at day boundaries.
// Default Formattable output: ISO 8601 ("15:30:05").
public struct Time: Equatable, Comparable, Hashable, Formattable, Cloneable {
    var nanosSinceMidnight: Int64

    static var NANOS_PER_SECOND: Int64 { 1_000_000_000 }
    static var NANOS_PER_MINUTE: Int64 { 60_000_000_000 }
    static var NANOS_PER_HOUR: Int64 { 3_600_000_000_000 }
    static var NANOS_PER_DAY: Int64 { 86_400_000_000_000 }

    // --- Construction ---

    public init(hour hour: Int64, minute minute: Int64 = 0, second second: Int64 = 0,
                nanosecond nanosecond: Int64 = 0) throws DateError {
        guard hour >= 0 and hour < 24 and
              minute >= 0 and minute < 60 and
              second >= 0 and second < 60 and
              nanosecond >= 0 and nanosecond < 1_000_000_000 else {
            throw DateError.InvalidTime(hour: hour, minute: minute, second: second);
        }
        self.nanosSinceMidnight = hour * Time.NANOS_PER_HOUR +
                                  minute * Time.NANOS_PER_MINUTE +
                                  second * Time.NANOS_PER_SECOND +
                                  nanosecond;
    }

    // Direct field construction — the primitive every other non-throwing
    // constructor builds on. No validation, no recursion.
    init(rawNanos n: Int64) {
        self.nanosSinceMidnight = n;
    }

    // Raw init from total nanoseconds (already validated)
    static func fromNanos(n: Int64) -> Time {
        Time(rawNanos: n)
    }

    public static var midnight: Time {
        Time.fromNanos(0)
    }

    public static var noon: Time {
        Time.fromNanos(12 * 3_600_000_000_000)
    }

    // --- Properties ---

    public var hour: Int64 { self.nanosSinceMidnight / Time.NANOS_PER_HOUR }
    public var minute: Int64 { (self.nanosSinceMidnight / Time.NANOS_PER_MINUTE) % 60 }
    public var second: Int64 { (self.nanosSinceMidnight / Time.NANOS_PER_SECOND) % 60 }
    public var nanosecond: Int64 { self.nanosSinceMidnight % Time.NANOS_PER_SECOND }

    // --- Arithmetic ---

    // Wraps at day boundaries. 23:59:59 + 1s = 00:00:00.
    public func advanced(by duration: Duration) -> Time {
        let deltaNanos = duration.totalNanoseconds;
        let total = self.nanosSinceMidnight + deltaNanos;
        let wrapped = floorMod(total, Time.NANOS_PER_DAY);
        Time.fromNanos(wrapped)
    }

    // Wraps and reports how many days overflowed.
    public func advancedWithOverflow(by duration: Duration) -> (Time, Int64) {
        let deltaNanos = duration.totalNanoseconds;
        let total = self.nanosSinceMidnight + deltaNanos;
        let days = floorDiv(total, Time.NANOS_PER_DAY);
        let wrapped = floorMod(total, Time.NANOS_PER_DAY);
        (Time.fromNanos(wrapped), days)
    }

    // --- Difference ---

    public func duration(to other: Time) -> Duration {
        let diff = other.nanosSinceMidnight - self.nanosSinceMidnight;
        Duration.nanoseconds(diff)
    }

    // --- Rounding ---

    public func rounded(to unit: TimeUnit, mode mode: RoundMode = .HalfExpand) -> Time {
        let divisor = unit.nanoseconds();
        if divisor <= 1 { return self }
        let rounded = roundValue(self.nanosSinceMidnight, divisor, mode);
        let wrapped = floorMod(rounded, Time.NANOS_PER_DAY);
        Time.fromNanos(wrapped)
    }

    // --- Conversion ---

    public func toDateTime(on date: Date) -> DateTime {
        DateTime(date: date, time: self)
    }

    // --- Parsing ---

    public static func parse(from input: String) -> Time throws ParseError {
        // Parse ISO 8601: HH:MM:SS or HH:MM:SS.nnnnnnnnn
        let bytes: Array[UInt8] = Array(from: input.bytes);
        guard bytes.count >= 8 else { throw ParseError.UnexpectedEnd; }
        let hour = try parseDigits(bytes, 0, 2);
        guard bytes(2) == 58 else { throw ParseError.InvalidFormat("expected ':' at position 2"); }
        let minute = try parseDigits(bytes, 3, 2);
        guard bytes(5) == 58 else { throw ParseError.InvalidFormat("expected ':' at position 5"); }
        let second = try parseDigits(bytes, 6, 2);

        var nanos: Int64 = 0;
        if bytes.count > 8 and bytes(8) == 46 {
            // Parse fractional seconds
            var i: Int64 = 9;
            var scale: Int64 = 100_000_000;
            while i < bytes.count and scale > 0 {
                let b = Int64(from: bytes(i));
                if b < 48 or b > 57 { break; }
                nanos = nanos + (b - 48) * scale;
                scale = scale / 10;
                i = i + 1;
            }
        }

        Time(hour: hour, minute: minute, second: second, nanosecond: nanos).mapErr { ParseError.InvalidValue("invalid time") }
    }

    // --- Protocol conformances ---

    public func isEqual(to other: Time) -> Bool {
        self.nanosSinceMidnight == other.nanosSinceMidnight
    }

    public func compare(other: Time) -> Ordering {
        self.nanosSinceMidnight.compare(other.nanosSinceMidnight)
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.nanosSinceMidnight.hash(into: hasher);
    }

    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        appendPadded(into: writer, self.hour, 2);
        writer.append(":");
        appendPadded(into: writer, self.minute, 2);
        writer.append(":");
        appendPadded(into: writer, self.second, 2);
    }

    public func clone() -> Time {
        Time.fromNanos(self.nanosSinceMidnight)
    }
}
