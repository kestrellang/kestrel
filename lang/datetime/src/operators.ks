module datetime

// Duration + Duration
extend Duration: Addable[Duration] {
    type Output = Duration
    public static var zero: Duration { Duration.zero }

    public consuming func add(consuming other: Duration) -> Duration {
        self.adding(other)
    }
}

// Duration - Duration
extend Duration: Subtractable[Duration] {
    type Output = Duration

    public consuming func subtract(consuming other: Duration) -> Duration {
        self.subtracting(other)
    }
}

// Duration * Int64
extend Duration: Multipliable[Int64] {
    type Output = Duration
    public static var one: Duration { Duration.seconds(1) }

    public consuming func multiply(consuming other: Int64) -> Duration {
        Duration.normalized(secs: self.secs * other, nanos: self.nanos * other)
    }
}

// Duration / Int64
extend Duration: Divisible[Int64] {
    type Output = Duration

    public consuming func divide(consuming other: Int64) -> Duration {
        Duration.nanoseconds(self.totalNanoseconds / other)
    }
}

// -Duration
extend Duration: Negatable {
    type Output = Duration

    public consuming func negate() -> Duration {
        self.negated()
    }
}
