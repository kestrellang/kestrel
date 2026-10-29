module datetime_regressions

// Self-checking regression program for the datetime package. Build and run
// with `lang/datetime/tests/run.sh`; exits non-zero if any check fails.

import datetime.(Date, Time, DateTime, ZonedDateTime, Instant, Duration, Period, TimeZone, Format, FakeClock, Weekday, TimeUnit)
import std.os.(exit)

struct Checker {
    var failures: Int64
    var total: Int64

    init() {
        self.failures = 0;
        self.total = 0;
    }

    mutating func check(name: String, got got: String, want want: String) {
        self.total = self.total + 1;
        if got == want { return; }
        self.failures = self.failures + 1;
        println("FAIL \(name)\n  got:  \(got)\n  want: \(want)");
    }

    mutating func checkTrue(name: String, cond: Bool) {
        self.check(name, got: "\(cond)", want: "true");
    }
}

func ny() -> TimeZone { TimeZone("America/New_York")! }

func zoned(y: Int64, mo: Int64, d: Int64, h: Int64, mi: Int64) -> ZonedDateTime {
    ZonedDateTime(year: y, month: mo, day: d, in: ny(), hour: h, minute: mi).unwrap()
}

// DST: the offset-guessing resolution could not see folds, so .Later was ignored.
func dstDisambiguation(mutating c: Checker) {
    let fold = DateTime(year: 2024, month: 11, day: 3, hour: 1, minute: 30).unwrap();
    c.checkTrue("fold is ambiguous", fold.isAmbiguous(in: ny()));
    c.check("fold .Earlier", got: "\(fold.toZoned(in: ny(), disambiguation: .Earlier))",
            want: "2024-11-03T01:30:00-04:00[America/New_York]");
    c.check("fold .Later", got: "\(fold.toZoned(in: ny(), disambiguation: .Later))",
            want: "2024-11-03T01:30:00-05:00[America/New_York]");
    c.check("fold .Compatible", got: "\(fold.toZoned(in: ny()))",
            want: "2024-11-03T01:30:00-04:00[America/New_York]");

    let gap = DateTime(year: 2024, month: 3, day: 10, hour: 2, minute: 30).unwrap();
    c.checkTrue("gap is nonexistent", gap.isNonexistent(in: ny()));
    c.check("gap .Earlier", got: "\(gap.toZoned(in: ny(), disambiguation: .Earlier))",
            want: "2024-03-10T01:30:00-05:00[America/New_York]");
    c.check("gap .Later", got: "\(gap.toZoned(in: ny(), disambiguation: .Later))",
            want: "2024-03-10T03:30:00-04:00[America/New_York]");
    c.check("gap .Compatible", got: "\(gap.toZoned(in: ny()))",
            want: "2024-03-10T03:30:00-04:00[America/New_York]");

    c.check("plain summer time", got: "\(zoned(2024, 7, 4, 9, 0))",
            want: "2024-07-04T09:00:00-04:00[America/New_York]");
    c.check("plain winter time", got: "\(zoned(2024, 1, 15, 9, 0))",
            want: "2024-01-15T09:00:00-05:00[America/New_York]");
}

// Instant must have one representation: nanos in [0, 1e9), seconds floored.
func instantRepresentation(mutating c: Checker) {
    let a = Instant(secondsSinceEpoch: 0, nanoseconds: -500_000_000);
    let b = Instant.parse(from: "1969-12-31T23:59:59.5Z").unwrap();
    c.check("negative fraction formats", got: "\(a)", want: "1969-12-31T23:59:59.5Z");
    c.checkTrue("parse == init", a == b);
    c.check("secondsSinceEpoch floors", got: "\(a.secondsSinceEpoch)", want: "-1");
    c.check("subsecond in range", got: "\(a.subsecondNanoseconds)", want: "500000000");
    c.check("toTime nanosecond", got: "\(a.toTime(in: TimeZone.utc).nanosecond)", want: "500000000");
    c.check("millis -1", got: "\(Instant(millisecondsSinceEpoch: -1))", want: "1969-12-31T23:59:59.999Z");
    c.check("millisecondsSinceEpoch", got: "\(a.millisecondsSinceEpoch)", want: "-500");
    let clock = FakeClock(at: Instant(secondsSinceEpoch: -1));
    c.check("today() floors before epoch", got: "\(Date.today(from: clock, in: TimeZone.utc))", want: "1969-12-31");
    let back = Instant(secondsSinceEpoch: 1).advanced(by: Duration.milliseconds(-1500));
    c.check("advance below epoch", got: "\(back)", want: "1969-12-31T23:59:59.5Z");
}

// Presets must round-trip; zone IDs and abbreviations are distinct components.
func formatPresets(mutating c: Checker) {
    let z = zoned(2024, 7, 4, 9, 0);
    let s = z.formatted(as: Format.rfc9557);
    c.check("rfc9557 preset", got: s, want: "2024-07-04T09:00:00-04:00[America/New_York]");
    let p = ZonedDateTime.parse(from: s, as: Format.rfc9557).unwrap();
    c.check("rfc9557 round-trip", got: "\(p)", want: "2024-07-04T09:00:00-04:00[America/New_York]");
    let abbr: Format = "\(.Hour12):\(.Minute) \(.AmPm) \(.TimeZoneName)";
    c.check("abbreviation component", got: z.formatted(as: abbr), want: "9:00 AM EDT");
}

// Components a value doesn't carry are skipped rather than invented.
func formatSkipsMissingFields(mutating c: Checker) {
    let dt = DateTime(year: 2024, month: 7, day: 4, hour: 9).unwrap();
    c.check("naive rfc3339 has no offset", got: dt.formatted(as: Format.rfc3339), want: "2024-07-04T09:00:00");
    let d = Date(year: 2024, month: 7, day: 4).unwrap();
    let dateAndHour: Format = "\(.Year)-\(.Month)-\(.Day) \(.Hour)";
    c.check("date skips hour", got: d.formatted(as: dateAndHour), want: "2024-07-04 ");
    let t = Time(hour: 13, minute: 5).unwrap();
    let timeAndYear: Format = "\(.Year)/\(.Hour):\(.Minute)";
    c.check("time skips year", got: t.formatted(as: timeAndYear), want: "/13:05");
}

// Default formatting keeps fractional seconds, so parse(format(x)) == x.
func defaultFormatRoundTrips(mutating c: Checker) {
    let t = Time(hour: 1, minute: 2, second: 3, nanosecond: 500_000_000).unwrap();
    c.check("time fraction", got: "\(t)", want: "01:02:03.5");
    c.checkTrue("time round-trip", Time.parse(from: "\(t)").unwrap() == t);
    let dt = DateTime(year: 2024, month: 7, day: 4, hour: 1, minute: 2, second: 3, nanosecond: 120).unwrap();
    c.check("datetime fraction", got: "\(dt)", want: "2024-07-04T01:02:03.00000012");
    c.checkTrue("datetime round-trip", DateTime.parse(from: "\(dt)").unwrap() == dt);
    let z = dt.toZoned(in: ny());
    c.check("zoned fraction", got: "\(z)", want: "2024-07-04T01:02:03.00000012-04:00[America/New_York]");
    c.checkTrue("zoned round-trip", ZonedDateTime.parse(from: "\(z)").unwrap() == z);
}

// startOf/endOf Month/Year are real boundaries on every type that has a time.
func periodBoundaries(mutating c: Checker) {
    let dt = DateTime(year: 2024, month: 7, day: 4, hour: 9).unwrap();
    c.check("DateTime.startOfMonth", got: "\(dt.startOfMonth())", want: "2024-07-01T00:00:00");
    c.check("DateTime.endOfMonth", got: "\(dt.endOfMonth())", want: "2024-07-31T23:59:59.999999999");
    c.check("DateTime.startOfYear", got: "\(dt.startOfYear())", want: "2024-01-01T00:00:00");
    c.check("DateTime.endOfYear", got: "\(dt.endOfYear())", want: "2024-12-31T23:59:59.999999999");
    let z = zoned(2024, 7, 4, 9, 0);
    c.check("Zoned.startOfMonth", got: "\(z.startOfMonth())", want: "2024-07-01T00:00:00-04:00[America/New_York]");
    c.check("Zoned.endOfMonth", got: "\(z.endOfMonth())",
            want: "2024-07-31T23:59:59.999999999-04:00[America/New_York]");
    c.check("Zoned.endOfYear", got: "\(z.endOfYear())",
            want: "2024-12-31T23:59:59.999999999-05:00[America/New_York]");
}

// Equality includes the zone; ordering and isSameInstant(as:) do not.
func zonedEquality(mutating c: Checker) {
    let a = zoned(2024, 7, 4, 9, 0);
    let b = a.toZoned(in: TimeZone("Europe/London")!);
    c.checkTrue("different zones are not ==", not (a == b));
    c.checkTrue("same instant", a.isSameInstant(as: b));
    c.checkTrue("ordering by instant", not (a < b) and not (b < a));
}

// Offset-only input parses to a fixed-offset zone; zone + offset picks the fold side.
func zonedParsing(mutating c: Checker) {
    let fixed = ZonedDateTime.parse(from: "2024-07-04T09:00:00+05:30", as: Format.rfc3339).unwrap();
    c.check("offset-only keeps offset", got: "\(fixed)", want: "2024-07-04T09:00:00+05:30[+05:30]");
    let later = ZonedDateTime.parse(from: "2024-11-03T01:30:00-05:00[America/New_York]").unwrap();
    c.check("zone + offset picks fold side", got: "\(later)", want: "2024-11-03T01:30:00-05:00[America/New_York]");
    c.check("TimeZone(offsetSeconds:)", got: "\(TimeZone(offsetSeconds: -12600)!)", want: "-03:30");
    c.checkTrue("TimeZone(offsetSeconds: 0) is utc", TimeZone(offsetSeconds: 0)! == TimeZone.utc);
    c.checkTrue("sub-minute offset rejected", TimeZone(offsetSeconds: 90).isNone());
    c.checkTrue("day-long offset rejected", TimeZone(offsetSeconds: 86400).isNone());
    c.checkTrue("offset must match zone",
                ZonedDateTime.parse(from: "2024-07-04T09:00:00-04:00[America/Los_Angeles]").isErr);
}

// Strict ISO parsing: no trailing input, no empty durations.
func strictParsing(mutating c: Checker) {
    c.checkTrue("Date rejects trailing", Date.parse(from: "2024-07-04xyz").isErr);
    c.checkTrue("Time rejects trailing", Time.parse(from: "01:02:03 junk").isErr);
    c.checkTrue("Instant rejects trailing", Instant.parse(from: "2024-07-04T00:00:00Zjunk").isErr);
    c.checkTrue("Instant requires an offset", Instant.parse(from: "2024-07-04T00:00:00").isErr);
    c.checkTrue("Duration rejects bare PT", Duration.parse(from: "PT").isErr);
    c.check("Duration accepts days", got: "\(Duration.parse(from: "P1DT2H").unwrap())", want: "PT26H");
    let yy: Format = "\(.ShortYear)-\(.Month)-\(.Day)";
    c.check("ShortYear pivot 99", got: "\(Date.parse(from: "99-01-02", as: yy).unwrap())", want: "1999-01-02");
    c.check("ShortYear pivot 24", got: "\(Date.parse(from: "24-01-02", as: yy).unwrap())", want: "2024-01-02");
}

// toZoned(in:) is the one spelling of "view in another zone".
func zoneConversionNaming(mutating c: Checker) {
    let z = zoned(2024, 7, 4, 9, 0);
    c.check("ZonedDateTime.toZoned(in:)", got: "\(z.toZoned(in: TimeZone("Europe/London")!))",
            want: "2024-07-04T14:00:00+01:00[Europe/London]");
}

// Calendar (Period) and exact (Duration) arithmetic through operators.
func operators(mutating c: Checker) {
    let d = Date(year: 2024, month: 1, day: 31).unwrap();
    c.check("Date + Period clips", got: "\(d + Period(months: 1))", want: "2024-02-29");
    c.check("Date - Period", got: "\(d - Period(days: 31))", want: "2023-12-31");
    let dt = DateTime(year: 2024, month: 7, day: 4, hour: 9).unwrap();
    c.check("DateTime + Duration", got: "\(dt + Duration.hours(20))", want: "2024-07-05T05:00:00");
    c.check("DateTime - Duration", got: "\(dt - Duration.minutes(30))", want: "2024-07-04T08:30:00");
    c.check("DateTime + Period", got: "\(dt + Period(weeks: 1))", want: "2024-07-11T09:00:00");
    c.check("DateTime - Period", got: "\(dt - Period(years: 1))", want: "2023-07-04T09:00:00");
    let next = DateTime(year: 2024, month: 7, day: 5).unwrap();
    c.check("DateTime - DateTime", got: "\(next - dt)", want: "PT15H");
    let z = zoned(2024, 7, 4, 9, 0);
    c.check("Zoned + Duration", got: "\(z + Duration.hours(1))", want: "2024-07-04T10:00:00-04:00[America/New_York]");
    c.check("Zoned - Duration", got: "\(z - Duration.hours(10))", want: "2024-07-03T23:00:00-04:00[America/New_York]");
    c.check("Zoned + Period keeps wall time", got: "\(z + Period(months: 6))",
            want: "2025-01-04T09:00:00-05:00[America/New_York]");
    c.check("Zoned - Period", got: "\(z - Period(days: 4))", want: "2024-06-30T09:00:00-04:00[America/New_York]");
    c.check("Period + Period", got: "\(Period(months: 1) + Period(days: 3))", want: "P1M3D");
    c.check("Period - Period", got: "\(Period(months: 1) - Period(days: 3))", want: "P1M-3D");
    c.check("-Period", got: "\(-Period(days: 2))", want: "-P2D");
    c.check("Duration + Duration", got: "\(Duration.seconds(1) + Duration.milliseconds(500))", want: "PT1.5S");
    c.check("Duration * Int64", got: "\(Duration.seconds(2) * 3)", want: "PT6S");
    c.check("Duration / Int64", got: "\(Duration.seconds(10) / 4)", want: "PT2.5S");
    c.check("Duration / Duration", got: "\(Duration.minutes(5) / Duration.seconds(30))", want: "10");
    c.check("TimeUnit.nanoseconds", got: "\(TimeUnit.Hour.nanoseconds)", want: "3600000000000");
}

// Duration accessors truncate toward zero (a signed magnitude, unlike Instant).
func durationAccessors(mutating c: Checker) {
    let neg = Duration.milliseconds(-1500);
    c.check("totalSeconds truncates", got: "\(neg.totalSeconds)", want: "-1");
    c.check("subsecond keeps sign", got: "\(neg.subsecondNanoseconds)", want: "-500000000");
    c.check("Period.weeks truncates", got: "\(Period(days: -10).weeks)", want: "-1");
    c.check("Period.remainingDays keeps sign", got: "\(Period(days: -10).remainingDays)", want: "-3");
    c.check("Duration.days", got: "\(Duration.days(2))", want: "PT48H");
    c.check("totalMicroseconds", got: "\(Duration.milliseconds(3).totalMicroseconds)", want: "3000");
    c.check("fractionalSeconds", got: "\(Duration.milliseconds(1500).fractionalSeconds)", want: "1.5");
    c.check("init(fractionalSeconds:)", got: "\(Duration(fractionalSeconds: 1.25)!)", want: "PT1.25S");
    c.check("humanString days + fraction", got: (Duration.hours(26) + Duration.milliseconds(1500)).humanString(),
            want: "1d 2h 1.5s");
    c.check("humanString negative fraction", got: neg.humanString(), want: "-1.5s");
}

// Public zone queries.
func zoneQueries(mutating c: Checker) {
    let summer = Instant.parse(from: "2024-07-04T13:00:00Z").unwrap();
    let winter = Instant.parse(from: "2024-01-15T13:00:00Z").unwrap();
    c.check("offset(at:)", got: "\(ny().offset(at: summer))", want: "-14400");
    c.check("abbreviation(at:)", got: ny().abbreviation(at: winter), want: "EST");
    c.checkTrue("isDaylightSavingTime summer", ny().isDaylightSavingTime(at: summer));
    c.checkTrue("isDaylightSavingTime winter", not ny().isDaylightSavingTime(at: winter));
    let ids = TimeZone.availableIdentifiers();
    c.checkTrue("available ids include New York", ids.contains("America/New_York"));
    c.checkTrue("available ids are many", ids.count > 300);
    c.check("TimeZone.parse", got: "\(TimeZone.parse(from: "Asia/Tokyo").unwrap())", want: "Asia/Tokyo");
    c.checkTrue("TimeZone.parse unknown", TimeZone.parse(from: "Nowhere/Zone").isErr);
}

// with(...) replaces fields and re-validates.
func withFields(mutating c: Checker) {
    let d = Date(year: 2024, month: 1, day: 31).unwrap();
    c.check("Date.with(day:)", got: "\(d.with(day: 1).unwrap())", want: "2024-01-01");
    c.checkTrue("Date.with invalid", d.with(month: 2).isErr);
    let t = Time(hour: 9, minute: 45).unwrap();
    c.check("Time.with(minute:)", got: "\(t.with(minute: 0).unwrap())", want: "09:00:00");
    let dt = DateTime(year: 2024, month: 7, day: 4, hour: 9).unwrap();
    c.check("DateTime.with", got: "\(dt.with(year: 2025, hour: 17).unwrap())", want: "2025-07-04T17:00:00");
    let z = zoned(2024, 7, 4, 9, 0);
    c.check("Zoned.with(month:)", got: "\(z.with(month: 12).unwrap())", want: "2024-12-04T09:00:00-05:00[America/New_York]");
}

// Epoch days, ISO weeks, quarters, weekday navigation.
func calendarQueries(mutating c: Checker) {
    let d = Date(year: 2024, month: 7, day: 4).unwrap();
    c.check("epochDay", got: "\(d.epochDay)", want: "19908");
    c.check("init(epochDay:)", got: "\(Date(epochDay: 19908))", want: "2024-07-04");
    c.check("init(epochDay: -1)", got: "\(Date(epochDay: -1))", want: "1969-12-31");
    c.check("quarter", got: "\(d.quarter)", want: "3");
    let endOf2024 = Date(year: 2024, month: 12, day: 30).unwrap();
    c.check("isoWeek rolls forward", got: "\(endOf2024.isoWeekYear)-W\(endOf2024.isoWeek)", want: "2025-W1");
    let startOf2021 = Date(year: 2021, month: 1, day: 3).unwrap();
    c.check("isoWeek rolls back", got: "\(startOf2021.isoWeekYear)-W\(startOf2021.isoWeek)", want: "2020-W53");
    c.check("Weekday.number Monday", got: "\(Weekday.Monday.number)", want: "1");
    c.check("Weekday.number Sunday", got: "\(Weekday.Sunday.number)", want: "7");
    c.check("Weekday(number:)", got: "\(Weekday(number: 3)!)", want: "Wednesday");
    c.checkTrue("Weekday(number: 8) is nil", Weekday(number: 8).isNone());
    c.check("next same weekday", got: "\(d.next(.Thursday))", want: "2024-07-11");
    c.check("next(.Friday)", got: "\(d.next(.Friday))", want: "2024-07-05");
    c.check("previous(.Monday)", got: "\(d.previous(.Monday))", want: "2024-07-01");
    let z = zoned(2024, 7, 4, 9, 0);
    c.check("Zoned.period(to:)", got: "\(z.period(to: z.adding(months: 1, days: 3)))", want: "P1M3D");
}

// Date ranges step one day at a time.
func dateRanges(mutating c: Checker) {
    let a = Date(year: 2024, month: 2, day: 27).unwrap();
    let b = Date(year: 2024, month: 3, day: 1).unwrap();
    var closed = StringBuilder();
    for day in a..=b { closed.append("\(day) "); }
    c.check("closed range", got: closed.build(), want: "2024-02-27 2024-02-28 2024-02-29 2024-03-01 ");
    var count: Int64 = 0;
    for _ in a..<b { count = count + 1; }
    c.check("half-open count", got: "\(count)", want: "3");
    c.checkTrue("range contains", (a..<b).contains(Date(year: 2024, month: 2, day: 29).unwrap()));
}

// Parsers for the remaining types.
func moreParsers(mutating c: Checker) {
    c.check("Period.parse", got: "\(Period.parse(from: "P1Y2M3W4D").unwrap())", want: "P1Y2M25D");
    c.check("Period.parse negative", got: "\(Period.parse(from: "-P1M").unwrap())", want: "-P1M");
    c.check("Period.parse mixed signs", got: "\(Period.parse(from: "P1M-3D").unwrap())", want: "P1M-3D");
    c.check("Period mixed humanString", got: Period(months: 1, days: -3).humanString(), want: "1mo -3d");
    c.checkTrue("Period.parse rejects time part", Period.parse(from: "P1DT2H").isErr);
    c.checkTrue("Period.parse rejects empty", Period.parse(from: "P").isErr);
    let fmt: Format = "\(.Month)/\(.Day)/\(.Year) \(.Hour):\(.Minute) \(.Offset)";
    c.check("Instant.parse(as:)", got: "\(Instant.parse(from: "07/04/2024 09:00 -04:00", as: fmt).unwrap())",
            want: "2024-07-04T13:00:00Z");
    let noZone: Format = "\(.Year)-\(.Month)-\(.Day) \(.Hour):\(.Minute)";
    c.checkTrue("Instant.parse(as:) requires an offset", Instant.parse(from: "2024-07-04 09:00", as: noZone).isErr);
}

// Period.isNegative: uniformly negative only; mixed signs are neither.
func periodSign(mutating c: Checker) {
    c.checkTrue("negative period", Period(months: -1, days: -3).isNegative);
    c.checkTrue("mixed signs not negative", not Period(months: 1, days: -3).isNegative);
    c.checkTrue("mixed signs not negative (leading negative)", not Period(months: -1, days: 3).isNegative);
    c.checkTrue("zero not negative", not Period.zero.isNegative);
}

// Clones of a FakeClock share one time source.
func sharedFakeClock(mutating c: Checker) {
    let clock = FakeClock(at: Instant(secondsSinceEpoch: 0));
    let view = clock.clone();
    clock.advance(by: Duration.hours(1));
    c.check("clone sees advance", got: "\(view.now())", want: "1970-01-01T01:00:00Z");
    view.setTo(Instant(secondsSinceEpoch: 86400));
    c.check("original sees setTo", got: "\(Instant.now(from: clock))", want: "1970-01-02T00:00:00Z");
}

// The registry grows past its old 256-entry cap.
func registryGrowth(mutating c: Checker) {
    var registered: Int64 = 0;
    var minutes: Int64 = 1;
    while minutes <= 400 {
        if TimeZone(offsetSeconds: minutes * 60).isSome() { registered = registered + 1; }
        minutes = minutes + 1;
    }
    c.check("400 fixed-offset zones", got: "\(registered)", want: "400");
    c.checkTrue("IANA zone after many fixed ones", TimeZone("America/Chicago").isSome());
}

// TZif footer rules apply after the table's last explicit transition (2037).
func footerRules(mutating c: Checker) {
    c.check("2100 summer", got: "\(zoned(2100, 7, 4, 9, 0))", want: "2100-07-04T09:00:00-04:00[America/New_York]");
    c.check("2100 winter", got: "\(zoned(2100, 1, 4, 9, 0))", want: "2100-01-04T09:00:00-05:00[America/New_York]");
    // First Sunday of November 2100 is the 7th; 01:30 occurs twice
    let fold = DateTime(year: 2100, month: 11, day: 7, hour: 1, minute: 30).unwrap();
    c.checkTrue("2100 fold is ambiguous", fold.isAmbiguous(in: ny()));
    let gap = DateTime(year: 2100, month: 3, day: 14, hour: 2, minute: 30).unwrap();
    c.checkTrue("2100 gap is nonexistent", gap.isNonexistent(in: ny()));
    c.check("2100 abbreviation", got: "\(zoned(2100, 7, 4, 9, 0).formatted(as: "\(.TimeZoneName)"))", want: "EDT");
    let sydney = TimeZone("Australia/Sydney")!;
    let s = ZonedDateTime(year: 2100, month: 1, day: 15, in: sydney, hour: 12).unwrap();
    c.check("2100 southern-hemisphere DST", got: "\(s)", want: "2100-01-15T12:00:00+11:00[Australia/Sydney]");
}

// Before the first transition, time type 0 (local mean time) applies.
func beforeFirstTransition(mutating c: Checker) {
    let old = Instant.parse(from: "1800-01-01T00:00:00Z").unwrap();
    c.check("NY local mean time", got: "\(ny().offset(at: old))", want: "-17762");
    c.check("LMT abbreviation", got: ny().abbreviation(at: old), want: "LMT");
}

// Zone names cannot escape the zoneinfo root.
func zoneNameSafety(mutating c: Checker) {
    c.checkTrue("dot-dot segment rejected", TimeZone("America/../America/New_York").isNone());
    c.checkTrue("absolute path rejected", TimeZone("/usr/share/zoneinfo/UTC").isNone());
    c.checkTrue("traversal rejected", TimeZone("../../../etc/passwd").isNone());
    c.checkTrue("ordinary name still loads", TimeZone("America/Argentina/Buenos_Aires").isSome());
}

@main
func main() {
    var c = Checker();
    dstDisambiguation(c);
    instantRepresentation(c);
    formatPresets(c);
    formatSkipsMissingFields(c);
    defaultFormatRoundTrips(c);
    periodBoundaries(c);
    zonedEquality(c);
    zonedParsing(c);
    strictParsing(c);
    zoneConversionNaming(c);
    operators(c);
    durationAccessors(c);
    zoneQueries(c);
    withFields(c);
    calendarQueries(c);
    dateRanges(c);
    moreParsers(c);
    periodSign(c);
    sharedFakeClock(c);
    registryGrowth(c);
    footerRules(c);
    beforeFirstTransition(c);
    zoneNameSafety(c);
    println("\(c.total - c.failures)/\(c.total) passed");
    if c.failures > 0 { exit(1); }
}
