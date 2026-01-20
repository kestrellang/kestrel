// Core protocols

module std.core

import std.core.(Less, LessOrEqual, Greater, GreaterOrEqual, NotEqual, Equal)

// Equatable - types that can be compared for equality
public protocol Equatable {
    func equals(other: Self) -> Bool
}

// Default operator implementation for Equatable (provides ==)
extend Equatable: Equal[Self] {
    type Equal.Output = Bool
}

// Comparable - types that have a total ordering
public protocol Comparable: Equatable {
    func compare(other: Self) -> Ordering
}

// Default operator implementations for Comparable
// This extension provides <, <=, >, >=, != for any Comparable type
extend Comparable: Less[Self], LessOrEqual[Self], Greater[Self], GreaterOrEqual[Self], NotEqual[Self] {
    type Less.Output = Bool
    type LessOrEqual.Output = Bool
    type Greater.Output = Bool
    type GreaterOrEqual.Output = Bool
    type NotEqual.Output = Bool

    public func lessThan(other: Self) -> Bool {
        self.compare(other) == Ordering.Less
    }

    public func lessThanOrEqual(other: Self) -> Bool {
        self.compare(other) != Ordering.Greater
    }

    public func greaterThan(other: Self) -> Bool {
        self.compare(other) == Ordering.Greater
    }

    public func greaterThanOrEqual(other: Self) -> Bool {
        self.compare(other) != Ordering.Less
    }

    public func notEquals(other: Self) -> Bool {
        self.compare(other) != Ordering.Equal
    }
}

// Hashable - types that can be hashed
// Note: write(bytes:) requires Slice[UInt8] which comes later
public protocol Hashable: Equatable {
    func hash[H](mutating into hasher: H) where H: Hasher
}

// Hasher - types that can compute hash values
// Note: Full implementation requires Slice and UInt64
public protocol Hasher {
    //mutating func write(bytes: Slice[UInt8])
    //mutating func finish() -> UInt64
}

// DefaultHasher comes later when we have UInt64 and Slice

// Defaultable - types with a default value
public protocol Defaultable {
    init()
}
