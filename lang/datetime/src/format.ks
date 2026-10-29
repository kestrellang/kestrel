module datetime

// String interpolation-based datetime formatting.
// All text output is English (no locale support in v1).
// A component the formatted value doesn't carry (`.Hour` on a Date, `.Offset`
// on a naive DateTime) produces no output rather than an invented value.

public enum FormatComponent: Equatable, Cloneable {
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
    case TimeZoneID         // "America/New_York" (IANA identifier; parses back to the zone)
    case TimeZoneName       // "EDT" (abbreviation in effect; not parseable back to a zone)
    case Offset             // "-04:00", or "Z" at UTC

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
            (.TimeZoneID, .TimeZoneID) => true,
            (.TimeZoneName, .TimeZoneName) => true,
            (.Offset, .Offset) => true,
            (.Literal(a), .Literal(b)) => a == b,
            _ => false
        }
    }

    public func matches(other: FormatComponent) -> Bool {
        self.isEqual(to: other)
    }

    public func clone() -> FormatComponent {
        // Only `.Literal` owns a heap payload (the String). A bare `self` here
        // bit-copies (aliases) that String pointer and double-frees on drop, so
        // clone it explicitly. Payload-less cases bit-copy safely.
        match self {
            .Literal(s) => .Literal(s.clone()),
            _ => self
        }
    }
}

// Accumulator for building Format values via string interpolation.
public struct FormatAccumulator: Interpolatable, Cloneable {
    var components: Array[FormatComponent]

    public init(literalCapacity literalCapacity: Int64, interpolationCount interpolationCount: Int64) {
        self.components = [];
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

    // Finalizer invoked by the string-interpolation desugar (`let f: Format = "..."`).
    // Mirrors DefaultStringInterpolation.build(): reads the accumulated value out.
    public func build() -> Format {
        Format(components: self.components.clone())
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
            .Offset, .Literal("["), .TimeZoneID, .Literal("]")
        ])
    }
}

// ============================================================================
// Formatting engine
// ============================================================================

// Month name lookup tables
let MONTH_NAMES: Array[String] = ["January", "February", "March", "April", "May", "June",
                   "July", "August", "September", "October", "November", "December"];
let SHORT_MONTH_NAMES: Array[String] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
                         "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
let NARROW_MONTH_NAMES: Array[String] = ["J", "F", "M", "A", "M", "J", "J", "A", "S", "O", "N", "D"];

// Weekday name lookup tables
let WEEKDAY_NAMES: Array[String] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
let SHORT_WEEKDAY_NAMES: Array[String] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
let NARROW_WEEKDAY_NAMES: Array[String] = ["M", "T", "W", "T", "F", "S", "S"];

// The civil fields a value can supply to a Format. Each `has*` flag gates the
// components that read the corresponding fields; ungated components emit nothing.
struct FormatFields {
    var hasDate: Bool
    var year: Int64
    var month: Int64
    var day: Int64
    var weekday: Weekday
    var hasTime: Bool
    var hour: Int64
    var minute: Int64
    var second: Int64
    var nanosecond: Int64
    var hasZone: Bool
    var offsetSeconds: Int64
    var tzId: String
    var tzAbbr: String

    init() {
        self.hasDate = false; self.year = 0; self.month = 1; self.day = 1; self.weekday = .Monday;
        self.hasTime = false; self.hour = 0; self.minute = 0; self.second = 0; self.nanosecond = 0;
        self.hasZone = false; self.offsetSeconds = 0; self.tzId = ""; self.tzAbbr = "";
    }

    mutating func setDate(d: Date) {
        self.hasDate = true;
        self.year = d.y; self.month = d.m; self.day = d.d;
        self.weekday = d.weekday;
    }

    mutating func setTime(t: Time) {
        self.hasTime = true;
        self.hour = t.hour; self.minute = t.minute;
        self.second = t.second; self.nanosecond = t.nanosecond;
    }

    // Render `fmt` against these fields.
    func render(fmt: Format) -> String {
        var b = StringBuilder();
        for comp in fmt.components {
            self.formatComponent(into: b, comp);
        }
        b.build()
    }

    func formatComponent(mutating into writer: some Formatter, component: FormatComponent) {
        match component {
            .Literal(s) => writer.append(s),
            .Year => if self.hasDate { appendPadded(into: writer, self.year, 4); },
            .ShortYear => if self.hasDate { appendPadded(into: writer, floorMod(self.year, 100), 2); },
            .Month => if self.hasDate { appendPadded(into: writer, self.month, 2); },
            .MonthUnpadded => if self.hasDate { writer.append("\(self.month)"); },
            .MonthName => if self.hasDate { writer.append(MONTH_NAMES(self.month - 1)); },
            .ShortMonthName => if self.hasDate { writer.append(SHORT_MONTH_NAMES(self.month - 1)); },
            .NarrowMonthName => if self.hasDate { writer.append(NARROW_MONTH_NAMES(self.month - 1)); },
            .Day => if self.hasDate { appendPadded(into: writer, self.day, 2); },
            .DayUnpadded => if self.hasDate { writer.append("\(self.day)"); },
            .Weekday => if self.hasDate { writer.append(WEEKDAY_NAMES(self.weekday.ordinal())); },
            .ShortWeekday => if self.hasDate { writer.append(SHORT_WEEKDAY_NAMES(self.weekday.ordinal())); },
            .NarrowWeekday => if self.hasDate { writer.append(NARROW_WEEKDAY_NAMES(self.weekday.ordinal())); },
            .Hour => if self.hasTime { appendPadded(into: writer, self.hour, 2); },
            .Hour12 => if self.hasTime { writer.append("\(hour12(self.hour))"); },
            .Hour12Padded => if self.hasTime { appendPadded(into: writer, hour12(self.hour), 2); },
            .Minute => if self.hasTime { appendPadded(into: writer, self.minute, 2); },
            .Second => if self.hasTime { appendPadded(into: writer, self.second, 2); },
            .Millisecond => if self.hasTime { appendPadded(into: writer, self.nanosecond / 1_000_000, 3); },
            .Microsecond => if self.hasTime { appendPadded(into: writer, self.nanosecond / 1000, 6); },
            .Nanosecond => if self.hasTime { appendPadded(into: writer, self.nanosecond, 9); },
            .AmPm => if self.hasTime { writer.append(if self.hour < 12 { "AM" } else { "PM" }); },
            .TimeZoneID => if self.hasZone { writer.append(self.tzId); },
            .TimeZoneName => if self.hasZone { writer.append(self.tzAbbr); },
            .Offset => if self.hasZone { appendOffset(into: writer, self.offsetSeconds); }
        };
    }
}

func hour12(hour: Int64) -> Int64 {
    if hour == 0 { 12 } else if hour > 12 { hour - 12 } else { hour }
}

// RFC 3339 offset: "Z" at zero, otherwise "+HH:MM" / "-HH:MM".
func appendOffset(mutating into writer: some Formatter, offsetSeconds: Int64) {
    if offsetSeconds == 0 {
        writer.append("Z");
        return;
    }
    let absOff = offsetSeconds.abs();
    writer.append(if offsetSeconds > 0 { "+" } else { "-" });
    appendPadded(into: writer, absOff / 3600, 2);
    writer.append(":");
    appendPadded(into: writer, (absOff % 3600) / 60, 2);
}

// ============================================================================
// formatted(as:) extensions for each type
// ============================================================================

extend Date {
    public func formatted(as fmt: Format) -> String {
        var f = FormatFields();
        f.setDate(self);
        f.render(fmt)
    }
}

extend Time {
    public func formatted(as fmt: Format) -> String {
        var f = FormatFields();
        f.setTime(self);
        f.render(fmt)
    }
}

extend DateTime {
    public func formatted(as fmt: Format) -> String {
        var f = FormatFields();
        f.setDate(self.dateVal);
        f.setTime(self.timeVal);
        f.render(fmt)
    }
}

extend ZonedDateTime {
    public func formatted(as fmt: Format) -> String {
        let dt = self.dateTime;
        var f = FormatFields();
        f.setDate(dt.dateVal);
        f.setTime(dt.timeVal);
        f.hasZone = true;
        f.offsetSeconds = self.offsetSeconds;
        f.tzId = self.tz.name;
        f.tzAbbr = self.tz.abbreviationAt(self.inst.secs);
        f.render(fmt)
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
        var f = ParsedFields();
        f.year = self.year; f.month = self.month; f.day = self.day;
        f.hour = self.hour; f.minute = self.minute; f.second = self.second;
        f.nanosecond = self.nanosecond; f.offsetSeconds = self.offsetSeconds;
        f.hasOffset = self.hasOffset; f.tzName = self.tzName.clone();
        f.hasTz = self.hasTz; f.isPm = self.isPm;
        f.isAmPm = self.isAmPm; f.is12Hour = self.is12Hour;
        f
    }

    init() {
        self.year = 0; self.month = 1; self.day = 1;
        self.hour = 0; self.minute = 0; self.second = 0;
        self.nanosecond = 0; self.offsetSeconds = 0;
        self.hasOffset = false; self.tzName = ""; self.hasTz = false;
        self.isPm = false; self.isAmPm = false; self.is12Hour = false;
    }
}

// Parse a single component from input, advancing the cursor
func parseComponent(bytes: Array[UInt8], mutating pos pos: Int64, component component: FormatComponent,
                    mutating fields fields: ParsedFields) -> () throws ParseError {
    match component {
        .Year => { fields.year = try parseDigits(bytes, pos, 4); pos = pos + 4; },
        .ShortYear => {
            // `try` hoisted out of operand position (MIR-3 OSSA ICE workaround).
            let yy = try parseDigits(bytes, pos, 2);
            // POSIX strptime pivot: 69-99 -> 1969-1999, 00-68 -> 2000-2068.
            fields.year = if yy >= 69 { 1900 + yy } else { 2000 + yy };
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
        .TimeZoneID => {
            // IANA identifiers ("America/New_York", "+05:30") run to the next ']' or the end.
            var end = pos;
            while end < bytes.count and bytes(end) != 93 { end = end + 1; }
            guard end > pos else { throw ParseError.InvalidFormat("expected time zone identifier"); }
            fields.tzName = stringFromBytes(bytes, pos, end - pos);
            fields.hasTz = true;
            pos = end;
        },
        .TimeZoneName => {
            // Abbreviations ("EDT") are ambiguous across zones, so they are
            // consumed but never resolved to a zone.
            var end = pos;
            while end < bytes.count and isAsciiLetter(bytes(end)) { end = end + 1; }
            guard end > pos else { throw ParseError.InvalidFormat("expected time zone abbreviation"); }
            pos = end;
        },
        .Offset => {
            let (off, len) = try parseOffset(bytes, pos);
            fields.offsetSeconds = off;
            fields.hasOffset = true;
            pos = pos + len;
        },
        .Literal(expected) => {
            let expBytes: Array[UInt8] = Array(from: expected.bytes);
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
    ()
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
        let nameBytes: Array[UInt8] = Array(from: name.bytes);
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
    String(fromUtf8Unchecked: bytes(start..<(start + length)))
}

func isAsciiLetter(b: UInt8) -> Bool {
    (b >= 65 and b <= 90) or (b >= 97 and b <= 122)
}

// Parse an RFC 3339 offset at `pos`: "Z"/"z", "+HH:MM", "-HH:MM" or "+HHMM".
// Returns (offset seconds east of UTC, bytes consumed).
func parseOffset(bytes: Array[UInt8], pos: Int64) -> (Int64, Int64) throws ParseError {
    guard pos < bytes.count else { throw ParseError.UnexpectedEnd; }
    let c = bytes(pos);
    if c == 90 or c == 122 { return (0, 1); }
    guard c == 43 or c == 45 else { throw ParseError.InvalidFormat("expected offset at position \(pos)"); }
    let h = try parseDigits(bytes, pos + 1, 2);
    var mPos = pos + 3;
    if mPos < bytes.count and bytes(mPos) == 58 { mPos = mPos + 1; }
    let m = try parseDigits(bytes, mPos, 2);
    guard h < 24 and m < 60 else { throw ParseError.InvalidValue("offset out of range"); }
    let magnitude = h * 3600 + m * 60;
    let off = if c == 45 { 0 - magnitude } else { magnitude };
    (off, mPos + 2 - pos)
}

// Index of the offset ("Z", "+", "-") that follows an ISO date-time in
// bytes[0..<end], or `end` if there is none. Scanning starts past the date's
// own '-' separators (position 19 = after "YYYY-MM-DDTHH:MM:SS").
func findOffsetStart(bytes: Array[UInt8], end: Int64) -> Int64 {
    var i: Int64 = 19;
    while i < end {
        let c = bytes(i);
        if c == 90 or c == 122 or c == 43 or c == 45 { return i; }
        i = i + 1;
    }
    end
}

// Run every component of `fmt` over `input`, rejecting unconsumed trailing
// input, and fold a 12-hour clock with AM/PM into a 24-hour `hour`.
func parseFields(input: String, fmt: Format) -> ParsedFields throws ParseError {
    let bytes: Array[UInt8] = Array(from: input.bytes);
    var pos: Int64 = 0;
    var fields = ParsedFields();
    for comp in fmt.components {
        try parseComponent(bytes, pos: pos, component: comp, fields: fields);
    }
    guard pos == bytes.count else {
        throw ParseError.InvalidFormat("unexpected trailing input at position \(pos)");
    }
    if fields.isAmPm {
        if fields.isPm and fields.hour < 12 { fields.hour = fields.hour + 12; }
        if not fields.isPm and fields.hour == 12 { fields.hour = 0; }
    }
    fields
}

// ============================================================================
// parse(from:, as:) extensions
// ============================================================================

extend Date {
    public static func parse(from input: String, as fmt: Format) -> Date throws ParseError {
        let f = try parseFields(input, fmt);
        guard isValidDate(f.year, f.month, f.day) else {
            throw ParseError.InvalidValue("invalid date");
        }
        .Ok(Date(rawYear: f.year, rawMonth: f.month, rawDay: f.day))
    }
}

extend Time {
    public static func parse(from input: String, as fmt: Format) -> Time throws ParseError {
        let f = try parseFields(input, fmt);
        let t = Time(hour: f.hour, minute: f.minute, second: f.second, nanosecond: f.nanosecond);
        t.mapErr { ParseError.InvalidValue("invalid time") }
    }
}

extend DateTime {
    public static func parse(from input: String, as fmt: Format) -> DateTime throws ParseError {
        let f = try parseFields(input, fmt);
        DateTime(year: f.year, month: f.month, day: f.day,
                 hour: f.hour, minute: f.minute, second: f.second,
                 nanosecond: f.nanosecond).mapErr { ParseError.InvalidValue("invalid datetime") }
    }
}

extend ZonedDateTime {
    // Zone resolution: a parsed zone ID wins, using a parsed offset (if any) to
    // pick the side of a DST fold; an offset alone yields a fixed-offset zone;
    // neither yields UTC.
    public static func parse(from input: String, as fmt: Format) -> ZonedDateTime throws ParseError {
        let f = try parseFields(input, fmt);
        let dt = try DateTime(year: f.year, month: f.month, day: f.day,
                              hour: f.hour, minute: f.minute, second: f.second,
                              nanosecond: f.nanosecond).mapErr { ParseError.InvalidValue("invalid datetime") };
        let zoneName: String? = if f.hasTz { .Some(f.tzName.clone()) } else { .None };
        let offset: Int64? = if f.hasOffset { .Some(f.offsetSeconds) } else { .None };
        ZonedDateTime.resolveParsed(dt, zoneName: zoneName, offset: offset)
    }
}

extend Instant {
    // Parse with a custom format. Like Instant.parse(from:), the input must
    // carry an offset or zone ID — without one the text names no instant.
    public static func parse(from input: String, as fmt: Format) -> Instant throws ParseError {
        let f = try parseFields(input, fmt);
        guard f.hasOffset or f.hasTz else {
            throw ParseError.InvalidFormat("format has no offset or time zone");
        }
        let dt = try DateTime(year: f.year, month: f.month, day: f.day,
                              hour: f.hour, minute: f.minute, second: f.second,
                              nanosecond: f.nanosecond).mapErr { ParseError.InvalidValue("invalid datetime") };
        let zoneName: String? = if f.hasTz { .Some(f.tzName.clone()) } else { .None };
        let offset: Int64? = if f.hasOffset { .Some(f.offsetSeconds) } else { .None };
        let z = try ZonedDateTime.resolveParsed(dt, zoneName: zoneName, offset: offset);
        .Ok(z.instant)
    }
}
