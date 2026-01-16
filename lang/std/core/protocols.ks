// Core protocols

module std.core

import std.ops.(Equal, NotEqual, Less, LessOrEqual, Greater, GreaterOrEqual)
import std.memory.(Slice)

// Equatable - types that can be compared for equality
public protocol Equatable {
    func equals(other: Self) -> Bool
}

// TODO: Protocol extensions not yet supported
// Default operator implementations for Equatable
// extend Equatable: Equal[Self], NotEqual[Self] {
//     type Output = Bool
//
//     func eq(other: Self) -> Bool {
//         self.equals(other)
//     }
//
//     func ne(other: Self) -> Bool {
//         not self.equals(other)
//     }
// }

// Comparable - types that have a total ordering
public protocol Comparable: Equatable {
    func compare(other: Self) -> Ordering
}

// Default operator implementations for Comparable
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
public protocol Hashable: Equatable {
    func hash[H](mutating into hasher: H) where H: Hasher
}

// Hasher - types that can compute hash values
public protocol Hasher {
    mutating func write(bytes: Slice[UInt8])
    mutating func finish() -> UInt64
}

// DefaultHasher (SipHash-1-3)
public struct DefaultHasher: Hasher {
    private var k0: UInt64
    private var k1: UInt64
    private var length: UInt64
    private var v0: UInt64
    private var v1: UInt64
    private var v2: UInt64
    private var v3: UInt64
    private var tail: UInt64
    private var tailLen: Int

    public init() {
        self.k0 = 0;
        self.k1 = 0;
        self.length = 0;
        self.v0 = 0x736f6d6570736575;
        self.v1 = 0x646f72616e646f6d;
        self.v2 = 0x6c7967656e657261;
        self.v3 = 0x7465646279746573;
        self.tail = 0;
        self.tailLen = 0;
    }

    public init(seed: (UInt64, UInt64)) {
        self.k0 = seed.0;
        self.k1 = seed.1;
        self.length = 0;
        self.v0 = self.k0 ^ 0x736f6d6570736575;
        self.v1 = self.k1 ^ 0x646f72616e646f6d;
        self.v2 = self.k0 ^ 0x6c7967656e657261;
        self.v3 = self.k1 ^ 0x7465646279746573;
        self.tail = 0;
        self.tailLen = 0;
    }

    public mutating func write(bytes: Slice[UInt8]) {
        // Implementation details - writes bytes into hasher state
        self.length = self.length + UInt64(bytes.count);
        // ... SipHash implementation
    }

    public mutating func finish() -> UInt64 {
        // Finalize and return hash
        // ... SipHash finalization
        0 // placeholder
    }
}

// Cloneable - types that can be implicitly copied via clone()
// Unlike simple Copyable (bitwise copy), Cloneable types have custom copy behavior.
// When a Cloneable value is copied, clone() is called automatically.
@builtin(.Cloneable)
public protocol Cloneable: Copyable {
    @builtin(.Clone)
    func clone() -> Self
}

// Copyable - marker protocol for types that can be implicitly copied
// Types implicitly conform to Copyable unless opted out with `not Copyable`
@builtin(.Copyable)
public protocol Copyable {}

// Defaultable - types with a default value
public protocol Defaultable {
    init()
}

// Convertible - types that can be converted from another type
public protocol Convertible[From] {
    init(from value: From)
}
