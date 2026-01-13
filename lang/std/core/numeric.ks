// Numeric protocols

module std.core

import std.ops.(ExpressibleByIntLiteral)

// Base numeric protocol
public protocol Numeric: Equatable, ExpressibleByIntLiteral {
    static var zero: Self { get }
    static var one: Self { get }
}

// Integer protocol
public protocol Integer: Numeric, Comparable, Hashable {
    static var minValue: Self { get }
    static var maxValue: Self { get }
    static var bitWidth: Int { get }
}

// Signed integer protocol
public protocol SignedInteger: Integer {
    func abs() -> Self
}

// Unsigned integer protocol
public protocol UnsignedInteger: Integer {}

// Floating-point protocol
public protocol FloatingPoint: Numeric, Comparable {
    static var infinity: Self { get }
    static var nan: Self { get }
    static var bitWidth: Int { get }

    func isNaN() -> Bool
    func isInfinite() -> Bool
    func isFinite() -> Bool
}

// Steppable - for types that can be incremented/decremented (used in ranges)
public protocol Steppable {
    func successor() -> Self
    func predecessor() -> Self
}

// TODO: Protocol extensions not yet supported
// Default implementations for integers
// extend Integer: Steppable where Self: Addable[Self], Self: Subtractable[Self] {
//     func successor() -> Self {
//         self + Self.one
//     }
//
//     func predecessor() -> Self {
//         self - Self.one
//     }
// }
