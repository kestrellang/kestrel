# std.time

## struct `Duration`

```kestrel
public struct Duration { /* private fields */ }
```

An exact span of time: signed, with nanosecond precision.

Build one with a unit, `Duration.milliseconds(500)` or `.seconds(5)`
where a `Duration` is expected, and combine them with `+`, `-`, `*` and
`/`. Prints in ISO 8601 form (`PT1.5S`); `humanString()` gives `1.5s`.

Internal: secs + nanos with uniform sign (both positive or both negative).

_Defined in `lang/std/time/duration.ks`._

### Members

#### field `NANOS_PER_SEC`

```kestrel
internal static var NANOS_PER_SEC: Int64 { get }
```

_Defined in `lang/std/time/duration.ks`._

#### function `abs`

```kestrel
public func abs() -> Duration
```

_Defined in `lang/std/time/duration.ks`._

#### function `days`

```kestrel
public static func days(Int64) -> Duration
```

_Defined in `lang/std/time/duration.ks`._

#### function `divided`

```kestrel
public func divided(by: Int64) -> Duration
```

_Defined in `lang/std/time/duration.ks`._

#### field `fractionalSeconds`

```kestrel
public var fractionalSeconds: Float64 { get }
```

_Defined in `lang/std/time/duration.ks`._

#### function `hours`

```kestrel
public static func hours(Int64) -> Duration
```

_Defined in `lang/std/time/duration.ks`._

#### function `humanString`

```kestrel
public func humanString() -> String
```

_Defined in `lang/std/time/duration.ks`._

#### initializer `init`

```kestrel
internal init(raw: Int64, rawNanos: Int64)
```

_Defined in `lang/std/time/duration.ks`._

#### initializer `init`

```kestrel
public init(seconds: Int64, nanoseconds: Int64)
```

_Defined in `lang/std/time/duration.ks`._

#### initializer `init`

```kestrel
public init(fractionalSeconds: Float64)?
```

_Defined in `lang/std/time/duration.ks`._

#### field `isNegative`

```kestrel
public var isNegative: Bool { get }
```

_Defined in `lang/std/time/duration.ks`._

#### field `isZero`

```kestrel
public var isZero: Bool { get }
```

_Defined in `lang/std/time/duration.ks`._

#### function `isoString`

```kestrel
public func isoString() -> String
```

_Defined in `lang/std/time/duration.ks`._

#### function `microseconds`

```kestrel
public static func microseconds(Int64) -> Duration
```

_Defined in `lang/std/time/duration.ks`._

#### function `milliseconds`

```kestrel
public static func milliseconds(Int64) -> Duration
```

_Defined in `lang/std/time/duration.ks`._

#### function `minutes`

```kestrel
public static func minutes(Int64) -> Duration
```

_Defined in `lang/std/time/duration.ks`._

#### function `multiplied`

```kestrel
public func multiplied(by: Int64) -> Duration
```

_Defined in `lang/std/time/duration.ks`._

#### field `nanos`

```kestrel
var nanos: Int64
```

_Defined in `lang/std/time/duration.ks`._

#### function `nanoseconds`

```kestrel
public static func nanoseconds(Int64) -> Duration
```

_Defined in `lang/std/time/duration.ks`._

#### function `negated`

```kestrel
public func negated() -> Duration
```

_Defined in `lang/std/time/duration.ks`._

#### function `normalized`

```kestrel
internal static func normalized(secs: Int64, nanos: Int64) -> Duration
```

_Defined in `lang/std/time/duration.ks`._

#### function `seconds`

```kestrel
public static func seconds(Int64) -> Duration
```

_Defined in `lang/std/time/duration.ks`._

#### field `secs`

```kestrel
var secs: Int64
```

_Defined in `lang/std/time/duration.ks`._

#### field `subsecondNanoseconds`

```kestrel
public var subsecondNanoseconds: Int64 { get }
```

_Defined in `lang/std/time/duration.ks`._

#### field `timeoutMilliseconds`

```kestrel
internal var timeoutMilliseconds: Int64 { get }
```

Whole milliseconds for a std timeout, rounded up so a sub-millisecond
wait isn't zero; negative durations count as zero.

_Defined in `lang/std/time/clock.ks`._

#### field `totalMicroseconds`

```kestrel
public var totalMicroseconds: Int64 { get }
```

_Defined in `lang/std/time/duration.ks`._

#### field `totalMilliseconds`

```kestrel
public var totalMilliseconds: Int64 { get }
```

_Defined in `lang/std/time/duration.ks`._

#### field `totalNanoseconds`

```kestrel
public var totalNanoseconds: Int64 { get }
```

_Defined in `lang/std/time/duration.ks`._

#### field `totalSeconds`

```kestrel
public var totalSeconds: Int64 { get }
```

_Defined in `lang/std/time/duration.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Duration) -> Bool
```

_Defined in `lang/std/time/duration.ks`._

### Implements `Comparable`

#### function `compare`

```kestrel
public func compare(Duration) -> Ordering
```

_Defined in `lang/std/time/duration.ks`._

### Implements `Hashable`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

_Defined in `lang/std/time/duration.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating some Formatter, FormatOptions)
```

_Defined in `lang/std/time/duration.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> Duration
```

_Defined in `lang/std/time/duration.ks`._

### Implements `Addable`

#### typealias `Output`

```kestrel
type Output = Duration
```

_Defined in `lang/std/time/duration.ks`._

#### typealias `Output`

```kestrel
type Output = Duration
```

_Defined in `lang/std/time/duration.ks`._

#### typealias `Output`

```kestrel
type Output = Duration
```

_Defined in `lang/std/time/duration.ks`._

#### typealias `Output`

```kestrel
type Output = Duration
```

_Defined in `lang/std/time/duration.ks`._

#### typealias `Output`

```kestrel
type Output = Duration
```

_Defined in `lang/std/time/duration.ks`._

#### typealias `Output`

```kestrel
type Output = Int64
```

_Defined in `lang/std/time/duration.ks`._

#### function `add`

```kestrel
public consuming func add(consuming Duration) -> Duration
```

_Defined in `lang/std/time/duration.ks`._

#### field `zero`

```kestrel
public static var zero: Duration { get }
```

_Defined in `lang/std/time/duration.ks`._

#### field `zero`

```kestrel
public static var zero: Duration { get }
```

_Defined in `lang/std/time/duration.ks`._

### Implements `Subtractable`

#### function `subtract`

```kestrel
public consuming func subtract(consuming Duration) -> Duration
```

_Defined in `lang/std/time/duration.ks`._

### Implements `Multipliable`

#### function `multiply`

```kestrel
public consuming func multiply(consuming Int64) -> Duration
```

_Defined in `lang/std/time/duration.ks`._

#### field `one`

```kestrel
public static var one: Duration { get }
```

_Defined in `lang/std/time/duration.ks`._

### Implements `Divisible`

#### function `divide`

```kestrel
public consuming func divide(consuming Int64) -> Duration
```

_Defined in `lang/std/time/duration.ks`._

#### function `divide`

```kestrel
public consuming func divide(consuming Duration) -> Int64
```

_Defined in `lang/std/time/duration.ks`._

### Implements `Negatable`

#### function `negate`

```kestrel
public consuming func negate() -> Duration
```

_Defined in `lang/std/time/duration.ks`._

## struct `Stopwatch`

```kestrel
public struct Stopwatch { /* private fields */ }
```

Measures elapsed time on the monotonic clock, so a system clock change
mid-measurement can't make it wrong or negative.

### Examples

```
let watch = Stopwatch.start();
doWork();
println("took \(watch.elapsed.humanString())");
```

_Defined in `lang/std/time/clock.ks`._

### Members

#### field `elapsed`

```kestrel
public var elapsed: Duration { get }
```

Time since the stopwatch started (or last restarted).

_Defined in `lang/std/time/clock.ks`._

#### initializer `init`

```kestrel
init(startedAt: Int64)
```

_Defined in `lang/std/time/clock.ks`._

#### function `restart`

```kestrel
public mutating func restart() -> Duration
```

Returns the elapsed time and starts counting again from now, for
timing laps.

_Defined in `lang/std/time/clock.ks`._

#### function `start`

```kestrel
public static func start() -> Stopwatch
```

A stopwatch running from now.

_Defined in `lang/std/time/clock.ks`._

#### field `startedAt`

```kestrel
var startedAt: Int64
```

_Defined in `lang/std/time/clock.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> Stopwatch
```

_Defined in `lang/std/time/clock.ks`._

## function `sleep`

```kestrel
public func sleep(for: Duration)
```

Pauses the current thread for `duration`. A zero or negative duration
returns at once.

### Examples

```
sleep(for: .milliseconds(250));
```

_Defined in `lang/std/time/clock.ks`._

