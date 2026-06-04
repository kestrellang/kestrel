module datetime

import std.memory.(Pointer)

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

// For testing — manually control the clock.
public struct FakeClock: Clock {
    var current: Instant

    public init(at instant: Instant) {
        self.current = instant;
    }

    public mutating func advance(by duration: Duration) {
        self.current = self.current.advanced(by: duration);
    }

    public mutating func setTo(instant: Instant) {
        self.current = instant;
    }

    public func now() -> Instant {
        self.current
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
        let inst = clock.now();
        let offset = kestrel_localtime_gmtoff(inst.secs);
        let localSecs = inst.secs + offset;
        dateFromEpochDay(floorDiv(localSecs, 86400))
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
