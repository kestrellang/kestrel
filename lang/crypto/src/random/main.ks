module crypto.random

import std.memory.(RawPointer, Pointer)
import std.numeric.(UInt64, UInt8, Int64, RandomNumberGenerator)

@extern(.C, mangleName: "arc4random_buf")
func arc4random_buf(consuming buf: RawPointer, consuming nbytes: Int64)

/// Cryptographically secure random number generator.
///
/// Each call to `nextUInt64` asks the OS for 8 fresh random bytes via
/// `arc4random_buf`. The struct is zero-sized — there is no internal
/// state to seed or maintain.
///
/// # Examples
///
/// ```
/// var rng = SecureRandom();
/// let a = rng.nextUInt64();
/// ```
public struct SecureRandom: RandomNumberGenerator {
    public init() {}

    /// Returns 8 cryptographically secure random bytes as a `UInt64`.
    public mutating func nextUInt64() -> UInt64 {
        var value: UInt64 = 0;
        arc4random_buf(Pointer(to: value).asRaw(), 8);
        value
    }
}

/// Fills an array with cryptographically secure random bytes.
public func randomBytes(count count: Int64) -> Array[UInt8] {
    var buf = Array[UInt8](repeating: 0, count: count);
    arc4random_buf(buf.asSlice().pointer.asRaw(), count);
    return buf;
}
