// Ordering enum for comparison results

public enum Ordering: Equatable {
    case Less
    case Equal
    case Greater

    public func equals(other: Ordering) -> Bool {
        match (self, other) {
            (.Less, .Less) => true,
            (.Equal, .Equal) => true,
            (.Greater, .Greater) => true,
            _ => false
        }
    }

    public func reverse() -> Ordering {
        match self {
            .Less => .Greater,
            .Equal => .Equal,
            .Greater => .Less,

            _ => .Equal, // todo: remove this case
        }
    }

    public func then(other: Ordering) -> Ordering {
        match self {
            .Equal => other,
            _ => self
        }
    }

    public func thenWith(compare: () -> Ordering) -> Ordering {
        match self {
            .Equal => compare(),
            _ => self
        }
    }
}
