module datetime

public enum Weekday: Equatable, Comparable, Hashable, Formattable, Matchable {
    case Monday
    case Tuesday
    case Wednesday
    case Thursday
    case Friday
    case Saturday
    case Sunday

    func ordinal() -> Int64 {
        match self {
            .Monday => 0,
            .Tuesday => 1,
            .Wednesday => 2,
            .Thursday => 3,
            .Friday => 4,
            .Saturday => 5,
            .Sunday => 6
        }
    }

    public func isEqual(to other: Weekday) -> Bool {
        self.ordinal() == other.ordinal()
    }

    public func compare(other: Weekday) -> Ordering {
        self.ordinal().compare(other.ordinal())
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.ordinal().hash(into: hasher);
    }

    public func matches(other: Weekday) -> Bool {
        self.isEqual(to: other)
    }

    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        let name = match self {
            .Monday => "Monday",
            .Tuesday => "Tuesday",
            .Wednesday => "Wednesday",
            .Thursday => "Thursday",
            .Friday => "Friday",
            .Saturday => "Saturday",
            .Sunday => "Sunday"
        };
        writer.append(name);
    }
}

public enum Overflow: Equatable, Matchable {
    case Clip
    case Rollover

    public func isEqual(to other: Overflow) -> Bool {
        match (self, other) {
            (.Clip, .Clip) => true,
            (.Rollover, .Rollover) => true,
            _ => false
        }
    }

    public func matches(other: Overflow) -> Bool {
        self.isEqual(to: other)
    }
}

public enum Disambiguation: Equatable, Matchable {
    case Compatible
    case Earlier
    case Later

    public func isEqual(to other: Disambiguation) -> Bool {
        match (self, other) {
            (.Compatible, .Compatible) => true,
            (.Earlier, .Earlier) => true,
            (.Later, .Later) => true,
            _ => false
        }
    }

    public func matches(other: Disambiguation) -> Bool {
        self.isEqual(to: other)
    }
}

public enum RoundMode: Equatable, Matchable {
    case Ceil
    case Floor
    case Expand
    case Truncate
    case HalfExpand
    case HalfEven

    public func isEqual(to other: RoundMode) -> Bool {
        match (self, other) {
            (.Ceil, .Ceil) => true,
            (.Floor, .Floor) => true,
            (.Expand, .Expand) => true,
            (.Truncate, .Truncate) => true,
            (.HalfExpand, .HalfExpand) => true,
            (.HalfEven, .HalfEven) => true,
            _ => false
        }
    }

    public func matches(other: RoundMode) -> Bool {
        self.isEqual(to: other)
    }
}

public enum TimeUnit: Equatable, Matchable {
    case Nanosecond
    case Microsecond
    case Millisecond
    case Second
    case Minute
    case Hour
    case Day

    // Nanoseconds per unit
    public func nanoseconds() -> Int64 {
        match self {
            .Nanosecond => 1,
            .Microsecond => 1000,
            .Millisecond => 1_000_000,
            .Second => 1_000_000_000,
            .Minute => 60_000_000_000,
            .Hour => 3_600_000_000_000,
            .Day => 86_400_000_000_000
        }
    }

    public func isEqual(to other: TimeUnit) -> Bool {
        self.nanoseconds() == other.nanoseconds()
    }

    public func matches(other: TimeUnit) -> Bool {
        self.isEqual(to: other)
    }
}
