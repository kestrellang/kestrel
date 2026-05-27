// Cryptographically secure random number generator backed by the OS.

module uuid.secure_random

import std.memory.(RawPointer, Pointer)
import std.numeric.(UInt64, UInt8, Int64, RandomNumberGenerator)

// arc4random_buf fills a buffer with cryptographically secure random bytes.
// Available on macOS (libc) and Linux (libbsd or glibc 2.36+).
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
/// let b = rng.nextUInt64();  // independent of a
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
