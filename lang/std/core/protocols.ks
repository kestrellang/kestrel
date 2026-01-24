// Core protocols

module std.core

import std.core.(Less, LessOrEqual, Greater, GreaterOrEqual, NotEqual, Equal)
import std.text.(String)
import std.memory.(Slice, Pointer)
import std.num.(UInt64)

// Equatable - types that can be compared for equality
public protocol Equatable {
    func equals(other: Self) -> Bool
}

// Matchable - types that can be matched in match expressions
@builtin(.Matchable)
public protocol Matchable {
    func matches(other: Self) -> Bool
}

// Default operator implementation for Equatable (provides == and !=)
extend Equatable: Equal[Self], NotEqual[Self] {
    type Equal.Output = Bool
    type NotEqual.Output = Bool

    public func notEquals(other: Self) -> Bool {
        if self.equals(other) { false } else { true }
    }
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

// Hash - types that can be hashed
public protocol Hash: Equatable {
    func hash[H](mutating into hasher: H) where H: Hasher
}

// Hasher - types that can compute hash values
public protocol Hasher {
    mutating func write(bytes: Slice[UInt8])
    mutating func finish() -> UInt64
}


// Defaultable - types with a default value
public protocol Defaultable {
    init()
}

// Formattable - types that can be formatted as a string
public protocol Formattable {
    func format() -> String
}
