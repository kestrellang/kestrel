# Datetime

Date, time, and timezone library for Kestrel. Distinct types for absolute time, civil (wall-clock) time, and zoned time, with exact and calendar arithmetic, IANA timezone support, and interpolation-based formatting.

## Installation

```toml
[dependencies]
datetime = { path = "../../lang/datetime" }
```

The package bundles a small C shim (`datetime_shims.c`, compiled automatically via the manifest's `c-sources`). Timezone data is read from the system's `/usr/share/zoneinfo/`: each zone's explicit transition table is extended with its TZif footer rule (DST in years past the table, typically 2037) through the year 2500, and instants before the first transition use the zone's first time type (usually local mean time). Zone names must be relative paths inside that directory (no `..`). Zones are interned in a growable, lock-protected registry.

## Core Types

- **Instant** - absolute point in time, nanosecond precision, no calendar or zone
- **Date** - calendar date (`2024-07-04`), no time or zone
- **Time** - wall-clock time of day, nanosecond precision, wraps at day boundaries
- **DateTime** - `Date` + `Time`, no zone ("what the wall clock shows")
- **ZonedDateTime** - `DateTime` + `TimeZone`; the "full" type. `==` compares instant *and* zone; `<` orders by instant; `isSameInstant(as:)` ignores the zone
- **TimeZone** - interned IANA or fixed-offset zone (`TimeZone("America/New_York")`, `TimeZone(offsetSeconds: 19800)` → `+05:30`, `TimeZone.utc`, `TimeZone.system()`)
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
println(launch.toZoned(in: TimeZone("Asia/Tokyo")!).formatted(as: clock)); // same instant, Tokyo wall clock

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
- `Instant(secondsSinceEpoch:nanoseconds:)`, `Instant(millisecondsSinceEpoch:)`, `Date(epochDay:)`
- `Duration.days(n)` (exact 24h) / `.hours(n)` / `.minutes(n)` / `.seconds(n)` / `.milliseconds(n)` / `.microseconds(n)` / `.nanoseconds(n)`, `Duration(fractionalSeconds: 1.5)` (failable)
- `Period(years:months:weeks:days:)` (all default 0)
- `with(...)` on `Date`, `Time`, `DateTime`, `ZonedDateTime` replaces fields and re-validates: `date.with(day: 1)`, `zdt.with(hour: 9, minute: 0)`

## Queries

- `Date`: `year`, `month`, `day`, `weekday`, `dayOfYear`, `epochDay`, `quarter`, `isoWeek`, `isoWeekYear`, `isLeapYear`, `daysInMonth`, `daysInYear`
- `Weekday`: `number` (ISO, Monday = 1 … Sunday = 7), `Weekday(number:)` (failable); `date.next(.Friday)` / `date.previous(.Monday)` (strictly after / before)
- `TimeZone`: `offset(at:)`, `abbreviation(at:)`, `isDaylightSavingTime(at:)`, `TimeZone.availableIdentifiers()`; `ZonedDateTime.offsetSeconds`
- `Duration` accessors treat it as a signed magnitude: `totalSeconds`, `totalMilliseconds`, `totalMicroseconds`, `totalNanoseconds` truncate toward zero and `subsecondNanoseconds` keeps the sign (-1.5s = -1s, -500ms); `fractionalSeconds` is a `Float64`. `Period.weeks` / `remainingDays` truncate the same way

## Arithmetic

- **Exact**: `advanced(by: Duration)` on `Instant`, `DateTime`, `ZonedDateTime`; `Time.advancedWrapping/advancedChecked/advancedWithOverflow(by:)`
- **Calendar**: `adding(years:months:days:overflow:)` and `adding(period:overflow:)` on `Date`, `DateTime`, `ZonedDateTime` — `ZonedDateTime.adding` preserves wall-clock time across DST
- **Difference**: `duration(to:)`, `Date.days(to:)`, `period(to:)` (on `Date`, `DateTime`, `ZonedDateTime`; date-based, time of day ignored)
- **Zone conversion**: `toZoned(in:)` on `Instant`, `DateTime` and `ZonedDateTime` (same instant, different wall clock)
- **Ranges**: `Date` is `Steppable`, so `for day in start..=end { }` and `(start..<end).contains(d)` step one calendar day
- **Navigation**: `tomorrow()`, `yesterday()`, `startOfDay/Month/Year()`, `endOfDay/Month/Year()` — on types with a time, these are real boundaries (start = 00:00 on the first day, end = 23:59:59.999999999 on the last)
- **Rounding**: `rounded(to: TimeUnit, mode: RoundMode)` on `Instant`, `Time`, `Duration`, `ZonedDateTime`
- Operators:
  - exact: `Instant`/`DateTime`/`ZonedDateTime` `± Duration`, `Instant - Instant` and `DateTime - DateTime` → `Duration`
  - calendar: `Date`/`DateTime`/`ZonedDateTime` `± Period` (clips, like `adding(period:)`)
  - `Duration + -`, `Duration * Int64`, `Duration / Int64`, `Duration / Duration -> Int64`, unary `-`
  - `Period + -` and unary `-`; mixed-sign periods format per field (`P1M-3D`) and are not `isNegative` (only an all-non-positive, non-zero period is)
  - there is no `Int64 * Duration` (a second `Multipliable` conformance on `Int64` breaks inference of `Int64 * Int64`); write `d * n`

## Formatting and Parsing

Every core type is `Formattable` with an ISO 8601 / RFC 3339 / RFC 9557 default rendering (fractional seconds included when non-zero), and `Instant`, `Date`, `Time`, `DateTime`, `ZonedDateTime`, `Duration`, `Period` and `TimeZone` have a strict `parse(from: String)` that round-trips it and rejects trailing input. `Duration.parse` accepts days as exact 24-hour units (`P1DT2H`).

`formatted(as: Format)` and `parse(from:as:)` (on `Date`, `Time`, `DateTime`, `ZonedDateTime`, `Instant`) accept custom formats; presets include `Format.isoDate`, `.isoTime`, `.isoDateTime`, `.rfc3339`, `.rfc2822`, `.rfc9557`. Zone components: `.TimeZoneID` (`America/New_York`, parses back to the zone), `.TimeZoneName` (abbreviation such as `EDT`, display only), `.Offset` (`-04:00` / `Z`). A component the value doesn't carry — `.Hour` on a `Date`, `.Offset` on a naive `DateTime` — produces no output.

`ZonedDateTime` parsing resolves the zone as: a zone ID wins, and an offset alongside it must match that zone (it selects the side of a DST fold); an offset alone gives a fixed-offset zone; neither gives UTC.

## DST Handling

Ambiguous or nonexistent wall-clock times (DST folds/gaps) are resolved via the `Disambiguation` parameter on `DateTime.toZoned(in:disambiguation:)` and the `ZonedDateTime` initializers. Query hazards with `DateTime.isAmbiguous(in:)` / `isNonexistent(in:)`.

## Testable Time

```kestrel
let clock = FakeClock(at: Instant(secondsSinceEpoch: 0));
let service = Service(clock: clock.clone());   // clones share one time source
clock.advance(by: Duration.hours(1));          // the service sees the new time too
let t = Instant.now(from: clock);   // also Date.today(from:) and ZonedDateTime.now(from:)
```

## Tests

`tests/run.sh` builds `tests/regressions.ks` with the in-repo compiler and exits non-zero on any failed check.
