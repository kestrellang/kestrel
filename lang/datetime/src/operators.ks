module datetime

// --- Period ---

extend Period: Addable[Period] {
    type Output = Period

    public consuming func add(consuming other: Period) -> Period {
        self.adding(other)
    }
}

extend Period: Subtractable[Period] {
    type Output = Period

    public consuming func subtract(consuming other: Period) -> Period {
        self.subtracting(other)
    }
}

extend Period: Negatable {
    type Output = Period

    public consuming func negate() -> Period {
        self.negated()
    }
}

// --- Calendar arithmetic: T ± Period (clipping overflow, like adding(period:)) ---
//
// `Addable` demands a `zero` even for a heterogeneous `T + Period`; the epoch
// stands in, since no date is an additive identity.

extend Date: Addable[Period] {
    type Output = Date
    public static var zero: Date { Date(epochDay: 0) }

    public consuming func add(consuming other: Period) -> Date {
        self.adding(period: other)
    }
}

extend Date: Subtractable[Period] {
    type Output = Date

    public consuming func subtract(consuming other: Period) -> Date {
        self.adding(period: other.negated())
    }
}

extend DateTime: Addable[Period] {
    type Output = DateTime
    public static var zero: DateTime { DateTime(date: Date(epochDay: 0), time: Time.midnight) }

    public consuming func add(consuming other: Period) -> DateTime {
        self.adding(period: other)
    }
}

extend DateTime: Subtractable[Period] {
    type Output = DateTime

    public consuming func subtract(consuming other: Period) -> DateTime {
        self.adding(period: other.negated())
    }
}

extend ZonedDateTime: Addable[Period] {
    type Output = ZonedDateTime
    public static var zero: ZonedDateTime { ZonedDateTime(instant: Instant(secondsSinceEpoch: 0), in: TimeZone.utc) }

    public consuming func add(consuming other: Period) -> ZonedDateTime {
        self.adding(period: other)
    }
}

extend ZonedDateTime: Subtractable[Period] {
    type Output = ZonedDateTime

    public consuming func subtract(consuming other: Period) -> ZonedDateTime {
        self.adding(period: other.negated())
    }
}

// --- Exact arithmetic: T ± Duration ---

extend DateTime: Addable[Duration] {
    type Output = DateTime

    public consuming func add(consuming other: Duration) -> DateTime {
        self.advanced(by: other)
    }
}

extend DateTime: Subtractable[Duration] {
    type Output = DateTime

    public consuming func subtract(consuming other: Duration) -> DateTime {
        self.advanced(by: other.negated())
    }
}

// DateTime - DateTime -> Duration (wall-clock difference, no zone)
extend DateTime: Subtractable[DateTime] {
    type Output = Duration

    public consuming func subtract(consuming other: DateTime) -> Duration {
        other.duration(to: self)
    }
}

extend ZonedDateTime: Addable[Duration] {
    type Output = ZonedDateTime

    public consuming func add(consuming other: Duration) -> ZonedDateTime {
        self.advanced(by: other)
    }
}

extend ZonedDateTime: Subtractable[Duration] {
    type Output = ZonedDateTime

    public consuming func subtract(consuming other: Duration) -> ZonedDateTime {
        self.advanced(by: other.negated())
    }
}
