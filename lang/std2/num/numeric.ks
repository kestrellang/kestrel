// Numeric protocols

module std.num

// Steppable - for types that can be incremented/decremented (used in ranges)
public protocol Steppable {
    func successor() -> Self
    func predecessor() -> Self
}

// Signed integer marker protocol
public protocol SignedInteger {
    func abs() -> Self
}

// Unsigned integer marker protocol
public protocol UnsignedInteger {}
