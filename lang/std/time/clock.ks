// Sleeping and measuring elapsed time on the monotonic clock: one that
// never jumps when the system time is adjusted, unlike the wall clock.

module std.time

import std.numeric.(Int64)
import std.core.(Bool, Cloneable)

@extern(.C, mangleName: "kestrel_monotonic_ns")
internal func native_monotonic_ns() -> Int64

@extern(.C, mangleName: "kestrel_sleep_ns")
internal func native_sleep_ns(secs: Int64, nanos: Int64)

/// Pauses the current thread for `duration`. A zero or negative duration
/// returns at once.
///
/// # Examples
///
/// ```
/// sleep(for: .milliseconds(250));
/// ```
public func sleep(for duration: Duration) {
    if duration.isNegative or duration.isZero {
        return
    }
    unsafe native_sleep_ns(duration.totalSeconds, duration.subsecondNanoseconds)
}

/// Measures elapsed time on the monotonic clock, so a system clock change
/// mid-measurement can't make it wrong or negative.
///
/// # Examples
///
/// ```
/// let watch = Stopwatch.start();
/// doWork();
/// println("took \(watch.elapsed.humanString())");
/// ```
public struct Stopwatch: Cloneable {
    // Monotonic nanoseconds at the last start.
    var startedAt: Int64

    init(startedAt startedAt: Int64) {
        self.startedAt = startedAt;
    }

    /// A stopwatch running from now.
    public static func start() -> Stopwatch {
        Stopwatch(startedAt: unsafe native_monotonic_ns())
    }

    /// Time since the stopwatch started (or last restarted).
    public var elapsed: Duration {
        Duration.nanoseconds(unsafe native_monotonic_ns() - self.startedAt)
    }

    /// Returns the elapsed time and starts counting again from now, for
    /// timing laps.
    public mutating func restart() -> Duration {
        let now = unsafe native_monotonic_ns();
        let lap = Duration.nanoseconds(now - self.startedAt);
        self.startedAt = now;
        lap
    }

    public func clone() -> Stopwatch { Stopwatch(startedAt: self.startedAt) }
}

extend Duration {
    /// Whole milliseconds for a std timeout, rounded up so a sub-millisecond
    /// wait isn't zero; negative durations count as zero.
    internal var timeoutMilliseconds: Int64 {
        if self.isNegative { return 0 }
        (self.totalNanoseconds + 999_999) / 1_000_000
    }
}
