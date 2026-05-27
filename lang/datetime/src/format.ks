module datetime

// String interpolation-based datetime formatting.
// All text output is English (no locale support in v1).

public enum FormatComponent: Equatable, Matchable, Cloneable {
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
    case Hour               // "15" (24-hour, zero-padded)
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

    // Literal (populated by appendLiteral)
    case Literal(String)

    public func isEqual(to other: FormatComponent) -> Bool {
        match (self, other) {
            (.Year, .Year) => true,
            (.ShortYear, .ShortYear) => true,
            (.Month, .Month) => true,
            (.MonthUnpadded, .MonthUnpadded) => true,
            (.MonthName, .MonthName) => true,
            (.ShortMonthName, .ShortMonthName) => true,
            (.NarrowMonthName, .NarrowMonthName) => true,
            (.Day, .Day) => true,
            (.DayUnpadded, .DayUnpadded) => true,
            (.Weekday, .Weekday) => true,
            (.ShortWeekday, .ShortWeekday) => true,
            (.NarrowWeekday, .NarrowWeekday) => true,
            (.Hour, .Hour) => true,
            (.Hour12, .Hour12) => true,
            (.Hour12Padded, .Hour12Padded) => true,
            (.Minute, .Minute) => true,
            (.Second, .Second) => true,
            (.Millisecond, .Millisecond) => true,
            (.Microsecond, .Microsecond) => true,
            (.Nanosecond, .Nanosecond) => true,
            (.AmPm, .AmPm) => true,
            (.TimeZoneName, .TimeZoneName) => true,
            (.FullTimeZoneName, .FullTimeZoneName) => true,
            (.Offset, .Offset) => true,
            (.Literal(a), .Literal(b)) => a == b,
            _ => false
        }
    }

    public func matches(other: FormatComponent) -> Bool {
        self.isEqual(to: other)
    }

    public func clone() -> FormatComponent { self }
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

    public mutating func appendInterpolation(value: FormatComponent) {
        self.components.append(value);
    }
}

// A reusable format specification for datetime formatting and parsing.
public struct Format: ExpressibleByStringInterpolation, Cloneable {
    public type Interpolation = FormatAccumulator

    public var components: Array[FormatComponent]

    public init(components components: Array[FormatComponent]) {
        self.components = components;
    }

    public init(stringLiteral ptr: lang.ptr[lang.i8], length: lang.i64) {
        self.components = [FormatComponent.Literal(String(stringLiteral: ptr, length))];
    }

    public init(interpolation: FormatAccumulator) {
        self.components = interpolation.components;
    }

    public func clone() -> Format {
        Format(components: self.components.clone())
    }

    // --- Presets ---

    public static var isoDate: Format {
        Format(components: [.Year, .Literal("-"), .Month, .Literal("-"), .Day])
    }

    public static var isoTime: Format {
        Format(components: [.Hour, .Literal(":"), .Minute, .Literal(":"), .Second])
    }

    public static var isoDateTime: Format {
        Format(components: [
            .Year, .Literal("-"), .Month, .Literal("-"), .Day,
            .Literal("T"),
            .Hour, .Literal(":"), .Minute, .Literal(":"), .Second
        ])
    }

    public static var rfc3339: Format {
        Format(components: [
            .Year, .Literal("-"), .Month, .Literal("-"), .Day,
            .Literal("T"),
            .Hour, .Literal(":"), .Minute, .Literal(":"), .Second,
            .Offset
        ])
    }

    public static var rfc2822: Format {
        Format(components: [
            .ShortWeekday, .Literal(", "), .Day, .Literal(" "),
            .ShortMonthName, .Literal(" "), .Year, .Literal(" "),
            .Hour, .Literal(":"), .Minute, .Literal(":"), .Second,
            .Literal(" "), .Offset
        ])
    }

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
// Formatting engine
// ============================================================================

// Month name lookup tables
let MONTH_NAMES = ["January", "February", "March", "April", "May", "June",
                   "July", "August", "September", "October", "November", "December"];
let SHORT_MONTH_NAMES = ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
                         "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
let NARROW_MONTH_NAMES = ["J", "F", "M", "A", "M", "J", "J", "A", "S", "O", "N", "D"];

// Weekday name lookup tables
let WEEKDAY_NAMES = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
let SHORT_WEEKDAY_NAMES = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
let NARROW_WEEKDAY_NAMES = ["M", "T", "W", "T", "F", "S", "S"];

// Format a component using date/time/zone values
func formatComponent(mutating into writer: StringBuilder,
                     component: FormatComponent,
                     year: Int64, month: Int64, day: Int64,
                     hour: Int64, minute: Int64, second: Int64, nanosecond: Int64,
                     weekday: Weekday,
                     offsetSeconds: Int64, tzName: String) {
    match component {
        .Year => appendPadded(into: writer, year, 4),
        .ShortYear => appendPadded(into: writer, year % 100, 2),
        .Month => appendPadded(into: writer, month, 2),
        .MonthUnpadded => writer.append("\(month)"),
        .MonthName => writer.append(MONTH_NAMES(month - 1)),
        .ShortMonthName => writer.append(SHORT_MONTH_NAMES(month - 1)),
        .NarrowMonthName => writer.append(NARROW_MONTH_NAMES(month - 1)),
        .Day => appendPadded(into: writer, day, 2),
        .DayUnpadded => writer.append("\(day)"),
        .Weekday => writer.append(WEEKDAY_NAMES(weekday.ordinal())),
        .ShortWeekday => writer.append(SHORT_WEEKDAY_NAMES(weekday.ordinal())),
        .NarrowWeekday => writer.append(NARROW_WEEKDAY_NAMES(weekday.ordinal())),
        .Hour => appendPadded(into: writer, hour, 2),
        .Hour12 => {
            let h12 = if hour == 0 { 12 } else if hour > 12 { hour - 12 } else { hour };
            writer.append("\(h12)");
        },
        .Hour12Padded => {
            let h12 = if hour == 0 { 12 } else if hour > 12 { hour - 12 } else { hour };
            appendPadded(into: writer, h12, 2);
        },
        .Minute => appendPadded(into: writer, minute, 2),
        .Second => appendPadded(into: writer, second, 2),
        .Millisecond => appendPadded(into: writer, nanosecond / 1_000_000, 3),
        .Microsecond => appendPadded(into: writer, nanosecond / 1000, 6),
        .Nanosecond => appendPadded(into: writer, nanosecond, 9),
        .AmPm => writer.append(if hour < 12 { "AM" } else { "PM" }),
        .TimeZoneName => writer.append(tzName),
        .FullTimeZoneName => writer.append(tzName),
        .Offset => {
            if offsetSeconds == 0 {
                writer.append("Z");
            } else {
                let sign = if offsetSeconds >= 0 { "+" } else { "-" };
                let absOff = offsetSeconds.abs();
                writer.append(sign);
                appendPadded(into: writer, absOff / 3600, 2);
                writer.append(":");
                appendPadded(into: writer, (absOff % 3600) / 60, 2);
            }
        },
        .Literal(s) => writer.append(s)
    };
}

// ============================================================================
// formatted(as:) extensions for each type
// ============================================================================

extend Date {
    public func formatted(as fmt: Format) -> String {
        var b = StringBuilder();
        for comp in fmt.components {
            formatComponent(into: b, component: comp,
                          year: self.y, month: self.m, day: self.d,
                          hour: 0, minute: 0, second: 0, nanosecond: 0,
                          weekday: self.weekday,
                          offsetSeconds: 0, tzName: "");
        }
        b.build()
    }
}

extend Time {
    public func formatted(as fmt: Format) -> String {
        var b = StringBuilder();
        for comp in fmt.components {
            formatComponent(into: b, component: comp,
                          year: 0, month: 0, day: 0,
                          hour: self.hour, minute: self.minute,
                          second: self.second, nanosecond: self.nanosecond,
                          weekday: .Monday,
                          offsetSeconds: 0, tzName: "");
        }
        b.build()
    }
}

extend DateTime {
    public func formatted(as fmt: Format) -> String {
        var b = StringBuilder();
        for comp in fmt.components {
            formatComponent(into: b, component: comp,
                          year: self.year, month: self.month, day: self.day,
                          hour: self.hour, minute: self.minute,
                          second: self.second, nanosecond: self.nanosecond,
                          weekday: self.weekday,
                          offsetSeconds: 0, tzName: "");
        }
        b.build()
    }
}

extend ZonedDateTime {
    public func formatted(as fmt: Format) -> String {
        let dt = self.dateTime;
        let off = self.offsetSeconds;
        let abbr = self.tz.abbreviationAt(self.inst.secs);
        var b = StringBuilder();
        for comp in fmt.components {
            formatComponent(into: b, component: comp,
                          year: dt.year, month: dt.month, day: dt.day,
                          hour: dt.hour, minute: dt.minute,
                          second: dt.second, nanosecond: dt.nanosecond,
                          weekday: dt.weekday,
                          offsetSeconds: off, tzName: abbr);
        }
        b.build()
    }
}

// ============================================================================
// Parsing engine
// ============================================================================

// Parsed fields accumulator
struct ParsedFields: Cloneable {
    var year: Int64
    var month: Int64
    var day: Int64
    var hour: Int64
    var minute: Int64
    var second: Int64
    var nanosecond: Int64
    var offsetSeconds: Int64
    var hasOffset: Bool
    var tzName: String
    var hasTz: Bool
    var isPm: Bool
    var isAmPm: Bool
    var is12Hour: Bool

    public func clone() -> ParsedFields {
        var f = ParsedFields.empty();
        f.year = self.year; f.month = self.month; f.day = self.day;
        f.hour = self.hour; f.minute = self.minute; f.second = self.second;
        f.nanosecond = self.nanosecond; f.offsetSeconds = self.offsetSeconds;
        f.hasOffset = self.hasOffset; f.tzName = self.tzName.clone();
        f.hasTz = self.hasTz; f.isPm = self.isPm;
        f.isAmPm = self.isAmPm; f.is12Hour = self.is12Hour;
        f
    }

    static func empty() -> ParsedFields {
        var f = ParsedFields(year: 0, month: 1, day: 1, hour: 0, minute: 0,
                             second: 0, nanosecond: 0, offsetSeconds: 0,
                             hasOffset: false, tzName: "", hasTz: false,
                             isPm: false, isAmPm: false, is12Hour: false);
        f
    }
}

// Parse a single component from input, advancing the cursor
func parseComponent(bytes: Array[UInt8], mutating pos: Int64, component: FormatComponent,
                    mutating fields: ParsedFields) throws ParseError {
    match component {
        .Year => { fields.year = try parseDigits(bytes, pos, 4); pos = pos + 4; },
        .ShortYear => {
            fields.year = 2000 + try parseDigits(bytes, pos, 2);
            pos = pos + 2;
        },
        .Month => { fields.month = try parseDigits(bytes, pos, 2); pos = pos + 2; },
        .MonthUnpadded => {
            let (val, len) = try parseDigitsVariable(bytes, pos, 1, 2);
            fields.month = val;
            pos = pos + len;
        },
        .MonthName => {
            let (idx, len) = try matchName(bytes, pos, MONTH_NAMES);
            fields.month = idx + 1;
            pos = pos + len;
        },
        .ShortMonthName => {
            let (idx, len) = try matchName(bytes, pos, SHORT_MONTH_NAMES);
            fields.month = idx + 1;
            pos = pos + len;
        },
        .NarrowMonthName => {
            let (idx, len) = try matchName(bytes, pos, NARROW_MONTH_NAMES);
            fields.month = idx + 1;
            pos = pos + len;
        },
        .Day => { fields.day = try parseDigits(bytes, pos, 2); pos = pos + 2; },
        .DayUnpadded => {
            let (val, len) = try parseDigitsVariable(bytes, pos, 1, 2);
            fields.day = val;
            pos = pos + len;
        },
        .Weekday => {
            let (_, len) = try matchName(bytes, pos, WEEKDAY_NAMES);
            pos = pos + len;
        },
        .ShortWeekday => {
            let (_, len) = try matchName(bytes, pos, SHORT_WEEKDAY_NAMES);
            pos = pos + len;
        },
        .NarrowWeekday => {
            let (_, len) = try matchName(bytes, pos, NARROW_WEEKDAY_NAMES);
            pos = pos + len;
        },
        .Hour => { fields.hour = try parseDigits(bytes, pos, 2); pos = pos + 2; },
        .Hour12 => {
            let (val, len) = try parseDigitsVariable(bytes, pos, 1, 2);
            fields.hour = val;
            fields.is12Hour = true;
            pos = pos + len;
        },
        .Hour12Padded => {
            fields.hour = try parseDigits(bytes, pos, 2);
            fields.is12Hour = true;
            pos = pos + 2;
        },
        .Minute => { fields.minute = try parseDigits(bytes, pos, 2); pos = pos + 2; },
        .Second => { fields.second = try parseDigits(bytes, pos, 2); pos = pos + 2; },
        .Millisecond => {
            let v = try parseDigits(bytes, pos, 3);
            fields.nanosecond = v * 1_000_000;
            pos = pos + 3;
        },
        .Microsecond => {
            let v = try parseDigits(bytes, pos, 6);
            fields.nanosecond = v * 1000;
            pos = pos + 6;
        },
        .Nanosecond => {
            fields.nanosecond = try parseDigits(bytes, pos, 9);
            pos = pos + 9;
        },
        .AmPm => {
            guard pos + 1 < bytes.count else { throw ParseError.UnexpectedEnd; }
            let c0 = bytes(pos);
            let c1 = bytes(pos + 1);
            if (c0 == 65 or c0 == 97) and (c1 == 77 or c1 == 109) {
                fields.isPm = false;
            } else if (c0 == 80 or c0 == 112) and (c1 == 77 or c1 == 109) {
                fields.isPm = true;
            } else {
                throw ParseError.InvalidFormat("expected AM/PM");
            }
            fields.isAmPm = true;
            pos = pos + 2;
        },
        .TimeZoneName => {
            // Read until next literal or end
            var end = pos;
            while end < bytes.count and bytes(end) != 91 and bytes(end) != 93 {
                end = end + 1;
            }
            fields.tzName = stringFromBytes(bytes, pos, end - pos);
            fields.hasTz = true;
            pos = end;
        },
        .FullTimeZoneName => {
            // Same as TimeZoneName for parsing
            var end = pos;
            while end < bytes.count and bytes(end) != 91 and bytes(end) != 93 {
                end = end + 1;
            }
            fields.tzName = stringFromBytes(bytes, pos, end - pos);
            fields.hasTz = true;
            pos = end;
        },
        .Offset => {
            guard pos < bytes.count else { throw ParseError.UnexpectedEnd; }
            let c = bytes(pos);
            if c == 90 or c == 122 {
                fields.offsetSeconds = 0;
                fields.hasOffset = true;
                pos = pos + 1;
            } else if c == 43 or c == 45 {
                let h = try parseDigits(bytes, pos + 1, 2);
                var mPos = pos + 3;
                if mPos < bytes.count and bytes(mPos) == 58 { mPos = mPos + 1; }
                let m = try parseDigits(bytes, mPos, 2);
                fields.offsetSeconds = h * 3600 + m * 60;
                if c == 45 { fields.offsetSeconds = 0 - fields.offsetSeconds; }
                fields.hasOffset = true;
                pos = mPos + 2;
            } else {
                throw ParseError.InvalidFormat("expected offset");
            }
        },
        .Literal(expected) => {
            let expBytes = expected.utf8;
            var i: Int64 = 0;
            while i < expBytes.count {
                guard pos + i < bytes.count else { throw ParseError.UnexpectedEnd; }
                guard bytes(pos + i) == expBytes(i) else {
                    throw ParseError.InvalidFormat("expected literal");
                }
                i = i + 1;
            }
            pos = pos + expBytes.count;
        }
    };
}

// Parse variable-width digits (1-2 digits for unpadded fields)
func parseDigitsVariable(bytes: Array[UInt8], offset: Int64, minLen: Int64, maxLen: Int64) -> (Int64, Int64) throws ParseError {
    var result: Int64 = 0;
    var len: Int64 = 0;
    while len < maxLen and offset + len < bytes.count {
        let b = Int64(from: bytes(offset + len));
        if b < 48 or b > 57 { break; }
        result = result * 10 + (b - 48);
        len = len + 1;
    }
    guard len >= minLen else {
        throw ParseError.InvalidFormat("expected at least \(minLen) digits at position \(offset)");
    }
    (result, len)
}

// Match a string against a list of names, return (index, length)
func matchName(bytes: Array[UInt8], offset: Int64, names: Array[String]) -> (Int64, Int64) throws ParseError {
    var i: Int64 = 0;
    while i < names.count {
        let name = names(i);
        let nameBytes = name.utf8;
        if offset + nameBytes.count <= bytes.count {
            var matches = true;
            var j: Int64 = 0;
            while j < nameBytes.count {
                let a = bytes(offset + j);
                let b = nameBytes(j);
                // Case-insensitive comparison
                let al = if a >= 65 and a <= 90 { a + 32 } else { a };
                let bl = if b >= 65 and b <= 90 { b + 32 } else { b };
                if al != bl { matches = false; break; }
                j = j + 1;
            }
            if matches {
                return (i, nameBytes.count);
            }
        }
        i = i + 1;
    }
    throw ParseError.InvalidValue("no matching name at position \(offset)")
}

// Extract a String from a byte slice
func stringFromBytes(bytes: Array[UInt8], start: Int64, length: Int64) -> String {
    var b = StringBuilder();
    var i: Int64 = 0;
    while i < length {
        b.appendByte(bytes(start + i));
        i = i + 1;
    }
    b.build()
}

// ============================================================================
// parse(from:, as:) extensions
// ============================================================================

extend Date {
    public static func parse(from input: String, as fmt: Format) -> Date throws ParseError {
        let bytes = input.utf8;
        var pos: Int64 = 0;
        var fields = ParsedFields.empty();
        for comp in fmt.components {
            try parseComponent(bytes, pos: pos, component: comp, fields: fields);
        }
        guard isValidDate(fields.year, fields.month, fields.day) else {
            throw ParseError.InvalidValue("invalid date");
        }
        Date.unchecked(year: fields.year, month: fields.month, day: fields.day)
    }
}

extend Time {
    public static func parse(from input: String, as fmt: Format) -> Time throws ParseError {
        let bytes = input.utf8;
        var pos: Int64 = 0;
        var fields = ParsedFields.empty();
        for comp in fmt.components {
            try parseComponent(bytes, pos: pos, component: comp, fields: fields);
        }
        var hour = fields.hour;
        if fields.isAmPm {
            if fields.isPm and hour < 12 { hour = hour + 12; }
            if not fields.isPm and hour == 12 { hour = 0; }
        }
        try Time(hour: hour, minute: fields.minute, second: fields.second, nanosecond: fields.nanosecond)
    }
}

extend DateTime {
    public static func parse(from input: String, as fmt: Format) -> DateTime throws ParseError {
        let bytes = input.utf8;
        var pos: Int64 = 0;
        var fields = ParsedFields.empty();
        for comp in fmt.components {
            try parseComponent(bytes, pos: pos, component: comp, fields: fields);
        }
        var hour = fields.hour;
        if fields.isAmPm {
            if fields.isPm and hour < 12 { hour = hour + 12; }
            if not fields.isPm and hour == 12 { hour = 0; }
        }
        try DateTime(year: fields.year, month: fields.month, day: fields.day,
                     hour: hour, minute: fields.minute, second: fields.second,
                     nanosecond: fields.nanosecond)
    }
}

extend ZonedDateTime {
    public static func parse(from input: String, as fmt: Format) -> ZonedDateTime throws ParseError {
        let bytes = input.utf8;
        var pos: Int64 = 0;
        var fields = ParsedFields.empty();
        for comp in fmt.components {
            try parseComponent(bytes, pos: pos, component: comp, fields: fields);
        }
        var hour = fields.hour;
        if fields.isAmPm {
            if fields.isPm and hour < 12 { hour = hour + 12; }
            if not fields.isPm and hour == 12 { hour = 0; }
        }
        let dt = try DateTime(year: fields.year, month: fields.month, day: fields.day,
                              hour: hour, minute: fields.minute, second: fields.second,
                              nanosecond: fields.nanosecond);
        if fields.hasTz {
            if let .Some(tz) = TimeZone.find(fields.tzName) {
                return dt.toZoned(in: tz);
            }
        }
        if fields.hasOffset {
            // Use UTC and adjust by offset
            let epochDay = daysToCivil(fields.year, fields.month, fields.day);
            let epochSec = epochDay * 86400 + hour * 3600 + fields.minute * 60 + fields.second - fields.offsetSeconds;
            let inst = Instant.raw(secs: epochSec, nanos: fields.nanosecond);
            return inst.toZoned(in: TimeZone.utc);
        }
        dt.toZoned(in: TimeZone.utc)
    }
}
