// Equatable, Hashable and Comparable for tuples of 2 to 4 elements

module std.core

import std.core.(Bool, Equatable, Hashable, Hasher, Comparable, Ordering)

// Each conformance holds when every element type has it. Equality and
// hashing go element by element; comparison is lexicographic, so tuples
// work as dictionary keys and sort keys (`[(Int64, Int64): V]`).

extend (A, B): Equatable where A: Equatable, B: Equatable {
    public func isEqual(to other: (A, B)) -> Bool {
        self.0.isEqual(to: other.0) and self.1.isEqual(to: other.1)
    }
}

extend (A, B): Hashable where A: Hashable, B: Hashable {
    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.0.hash(into: hasher);
        self.1.hash(into: hasher);
    }
}

extend (A, B): Comparable where A: Comparable, B: Comparable {
    public func compare(other: (A, B)) -> Ordering {
        let c0 = self.0.compare(other.0);
        if c0 != .Equal { return c0 }
        self.1.compare(other.1)
    }
}

extend (A, B, C): Equatable where A: Equatable, B: Equatable, C: Equatable {
    public func isEqual(to other: (A, B, C)) -> Bool {
        self.0.isEqual(to: other.0) and self.1.isEqual(to: other.1) and self.2.isEqual(to: other.2)
    }
}

extend (A, B, C): Hashable where A: Hashable, B: Hashable, C: Hashable {
    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.0.hash(into: hasher);
        self.1.hash(into: hasher);
        self.2.hash(into: hasher);
    }
}

extend (A, B, C): Comparable where A: Comparable, B: Comparable, C: Comparable {
    public func compare(other: (A, B, C)) -> Ordering {
        let c0 = self.0.compare(other.0);
        if c0 != .Equal { return c0 }
        let c1 = self.1.compare(other.1);
        if c1 != .Equal { return c1 }
        self.2.compare(other.2)
    }
}

extend (A, B, C, D): Equatable where A: Equatable, B: Equatable, C: Equatable, D: Equatable {
    public func isEqual(to other: (A, B, C, D)) -> Bool {
        self.0.isEqual(to: other.0) and self.1.isEqual(to: other.1) and self.2.isEqual(to: other.2) and self.3.isEqual(to: other.3)
    }
}

extend (A, B, C, D): Hashable where A: Hashable, B: Hashable, C: Hashable, D: Hashable {
    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.0.hash(into: hasher);
        self.1.hash(into: hasher);
        self.2.hash(into: hasher);
        self.3.hash(into: hasher);
    }
}

extend (A, B, C, D): Comparable where A: Comparable, B: Comparable, C: Comparable, D: Comparable {
    public func compare(other: (A, B, C, D)) -> Ordering {
        let c0 = self.0.compare(other.0);
        if c0 != .Equal { return c0 }
        let c1 = self.1.compare(other.1);
        if c1 != .Equal { return c1 }
        let c2 = self.2.compare(other.2);
        if c2 != .Equal { return c2 }
        self.3.compare(other.3)
    }
}
