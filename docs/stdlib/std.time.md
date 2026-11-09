# std.time

Exact spans of time, sleeping, and measuring elapsed time. Visible
everywhere without an import.

## struct `Duration`

```kestrel
public struct Duration: Equatable, Comparable, Hashable, Formattable, Cloneable
```

An exact span of time: signed, with nanosecond precision. Every std timeout
takes one (`Command.timeout`, `Child.wait(for:)`, `TcpStream.connect(...,
timeout:)`, `setReadTimeout` / `setWriteTimeout`).

- Build: `Duration.nanoseconds(n)`, `.microseconds(n)`, `.milliseconds(n)`,
  `.seconds(n)`, `.minutes(n)`, `.hours(n)`, `.days(n)` (24 hours),
  `Duration(seconds:nanoseconds:)`, `Duration(fractionalSeconds:)?`,
  `Duration.zero`.
- Read: `totalSeconds`, `totalMilliseconds`, `totalMicroseconds`,
  `totalNanoseconds`, `subsecondNanoseconds`, `fractionalSeconds`,
  `isNegative`, `isZero`. Whole units truncate toward zero.
- Arithmetic: `+`, `-`, unary `-`, `* Int64`, `/ Int64`, `/ Duration`
  (whole count), `negated()`, `abs()`, `multiplied(by:)`, `divided(by:)`.
- Print: ISO 8601 (`PT1.5S`) by default; `humanString()` gives `1.5s`,
  `isoString()` the ISO form.

The datetime package adds `rounded(to:mode:)` and `Duration.parse(from:)`.

_Defined in `lang/std/time/duration.ks`._

## func `sleep`

```kestrel
public func sleep(for: Duration)
```

Pauses the current thread for `duration`. A zero or negative duration
returns at once.

_Defined in `lang/std/time/clock.ks`._

## struct `Stopwatch`

```kestrel
public struct Stopwatch: Cloneable
```

Measures elapsed time on the monotonic clock, so a system clock change
mid-measurement can't make it wrong or negative.

- `Stopwatch.start()`: a stopwatch running from now.
- `elapsed: Duration`: time since the start (or last restart).
- `restart() -> Duration` (mutating): returns the elapsed time and starts
  counting again, for laps.

_Defined in `lang/std/time/clock.ks`._
