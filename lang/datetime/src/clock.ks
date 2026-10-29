module datetime

import std.memory.(Pointer, RcBox)

public protocol Clock {
    func now() -> Instant
}

public struct SystemClock: Clock {
    public static var shared: SystemClock { SystemClock() }

    public init() {}

    public func now() -> Instant {
        Instant.now()
    }
}

// For testing — manually control the clock. Clones share one time source,
// so handing `clock.clone()` to the code under test and then calling
// `clock.advance(by:)` moves the time that code sees too.
public struct FakeClock: Clock, Cloneable {
    var current: RcBox[Instant]

    public init(at instant: Instant) {
        self.current = RcBox(instant);
    }

    init(sharing current: RcBox[Instant]) {
        self.current = current;
    }

    public func advance(by duration: Duration) {
        self.current.setValue(self.current.getValue().advanced(by: duration));
    }

    public func setTo(instant: Instant) {
        self.current.setValue(instant);
    }

    public func now() -> Instant {
        self.current.getValue()
    }

    // A handle on the same time source (not a snapshot).
    public func clone() -> FakeClock {
        FakeClock(sharing: self.current.clone())
    }
}

// Clock-parameterized now() for Instant
extend Instant {
    public static func now(from clock: some Clock) -> Instant {
        clock.now()
    }
}

// Clock-parameterized today() for Date
extend Date {
    public static func today(from clock: some Clock) -> Date {
        clock.now().toDate(in: TimeZone.system())
    }

    public static func today(from clock: some Clock, in zone: TimeZone) -> Date {
        let inst = clock.now();
        inst.toDate(in: zone)
    }
}

// Clock-parameterized now() for ZonedDateTime
extend ZonedDateTime {
    public static func now(from clock: some Clock) -> ZonedDateTime {
        clock.now().toZoned(in: TimeZone.system())
    }

    public static func now(from clock: some Clock, in zone: TimeZone) -> ZonedDateTime {
        clock.now().toZoned(in: zone)
    }
}
