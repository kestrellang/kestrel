# Datetime

Date, time, and timezone library for Kestrel. Distinct types for absolute time, civil (wall-clock) time, and zoned time, with exact and calendar arithmetic, IANA timezone support, and interpolation-based formatting.

## Installation

```toml
[dependencies]
datetime = { path = "../../lang/datetime" }
```

The package bundles a small C shim (`datetime_shims.c`, compiled automatically via the manifest's `c-sources`). Timezone data is read from the system's `/usr/share/zoneinfo/`.

## Core Types

- **Instant** - absolute point in time, nanosecond precision, no calendar or zone
- **Date** - calendar date (`2024-07-04`), no time or zone
- **Time** - wall-clock time of day, nanosecond precision, wraps at day boundaries
- **DateTime** - `Date` + `Time`, no zone ("what the wall clock shows")
- **ZonedDateTime** - `DateTime` + IANA `TimeZone`; the "full" type, compared by instant
- **TimeZone** - interned IANA timezone (`TimeZone("America/New_York")`, `TimeZone.utc`, `TimeZone.system()`)
- **Duration** - exact elapsed time (signed, nanosecond precision)
- **Period** - calendar duration (years, months, weeks, days)
- **Clock** / **SystemClock** / **FakeClock** - injectable time source for testing
- **Format** / **FormatComponent** - interpolation-based format strings
- Enums: **Weekday**, **Overflow** (`.Clip`/`.Rollover`), **Disambiguation** (`.Compatible`/`.Earlier`/`.Later`), **RoundMode**, **TimeUnit**
- Errors: **DateTimeError** (invalid date/time), **ParseError**

## Usage

Timezone-aware scheduling with custom formatting (from the datetime showcase):

```kestrel
import datetime.(ZonedDateTime, TimeZone, Format, DateTimeError)

let launch = try ZonedDateTime(year: 2026, month: 7, day: 4,
                               in: TimeZone("America/New_York")!,
                               hour: 9, minute: 30);

// Format strings are built from FormatComponent interpolations
let clock: Format = "\(.ShortWeekday) \(.Hour12):\(.Minute) \(.AmPm) \(.TimeZoneName)";
println(launch.formatted(as: clock));                                   // Sat 9:30 AM EDT
println(launch.inTimeZone(TimeZone("Asia/Tokyo")!).formatted(as: clock)); // same instant, Tokyo wall clock

let review = launch.adding(months: 1, days: 10);       // calendar arithmetic
println("Review: \(review.formatted(as: .isoDate))");
println("Elapsed: \(launch.duration(to: review).humanString())");
```

Instants and exact arithmetic:

```kestrel
import datetime.(Instant, Duration, TimeZone)

let now = Instant.now();
let later = now.advanced(by: Duration.minutes(5));   // or: now + Duration.minutes(5)
let elapsed = later - now;                           // Instant - Instant -> Duration
let local = now.toDateTime(in: TimeZone.system());
```

## Construction and Validation

- `Date(year:month:day:)`, `Time(hour:minute:second:nanosecond:)`, `DateTime(year:month:day:hour:...)`, `ZonedDateTime(year:...:in:...)` all validate and `throws DateTimeError`
- `Date.today()`, `Date.today(in: zone)`, `Instant.now()`, `ZonedDateTime.now()`, `ZonedDateTime.now(in: zone)`
- `Instant(secondsSinceEpoch:nanoseconds:)`, `Instant(millisecondsSinceEpoch:)`
- `Duration.seconds(n)` / `.minutes(n)` / `.hours(n)` / `.milliseconds(n)` / `.nanoseconds(n)`
- `Period(years:months:weeks:days:)` (all default 0)

## Arithmetic

- **Exact**: `advanced(by: Duration)` on `Instant`, `DateTime`, `ZonedDateTime`; `Time.advancedWrapping/advancedChecked/advancedWithOverflow(by:)`
- **Calendar**: `adding(years:months:days:overflow:)` and `adding(period:overflow:)` on `Date`, `DateTime`, `ZonedDateTime` — `ZonedDateTime.adding` preserves wall-clock time across DST
- **Difference**: `duration(to:)`, `Date.days(to:)`, `period(to:)`
- **Navigation**: `tomorrow()`, `yesterday()`, `startOfDay/Month/Year()`, `endOfDay/Month/Year()`
- **Rounding**: `rounded(to: TimeUnit, mode: RoundMode)` on `Instant`, `Time`, `Duration`, `ZonedDateTime`
- Operators: `Instant ± Duration`, `Instant - Instant -> Duration`, `Duration + - * /` and unary negation

## Formatting and Parsing

Every core type is `Formattable` with an ISO 8601 / RFC 3339 / RFC 9557 default rendering, and has `parse(from: String)`. `formatted(as: Format)` and `parse(from:as:)` accept custom formats; presets include `Format.isoDate`, `.isoTime`, `.isoDateTime`, `.rfc3339`, `.rfc2822`, `.rfc9557`.

## DST Handling

Ambiguous or nonexistent wall-clock times (DST folds/gaps) are resolved via the `Disambiguation` parameter on `DateTime.toZoned(in:disambiguation:)` and the `ZonedDateTime` initializers. Query hazards with `DateTime.isAmbiguous(in:)` / `isNonexistent(in:)`.

## Testable Time

```kestrel
var clock = FakeClock(at: Instant(secondsSinceEpoch: 0));
clock.advance(by: Duration.hours(1));
let t = Instant.now(from: clock);   // also Date.today(from:) and ZonedDateTime.now(from:)
```
