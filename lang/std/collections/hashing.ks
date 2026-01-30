// Default hashing implementation using FNV-1a

module std.collections

import std.core.(Hasher, Hash, Defaultable)
import std.num.(UInt8, UInt64, Int64)
import std.memory.(Slice)

// ============================================================================
// DEFAULT HASHER
// ============================================================================

/// Default hasher using the FNV-1a algorithm.
///
/// FNV-1a is a non-cryptographic hash function created by Glenn Fowler,
/// Landon Curt Noll, and Phong Vo. It's fast and has good distribution
/// properties for hash table use.
public struct DefaultHasher: Hasher, Defaultable {
    private var state: UInt64

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// Creates a new hasher with the FNV-1a offset basis.
    public init() {
        // FNV offset basis for 64-bit
        self.state = UInt64(intLiteral: 14695981039346656037);
    }

    // ========================================================================
    // HASHER PROTOCOL
    // ========================================================================

    /// Writes bytes into the hasher state.
    public mutating func write(bytes: Slice[UInt8]) {
        let count = bytes.count;
        let ptr = bytes.pointer;
        var i: Int64 = Int64(intLiteral: 0);
        let prime = UInt64(intLiteral: 1099511628211);

        while i < count {
            let byte = ptr.offset(by: i).read();
            // XOR state with byte
            self.state = self.state.bitwiseXor(UInt64(from: byte));
            // Multiply by prime
            self.state = self.state.multiply(prime);
            i = i + Int64(intLiteral: 1)
        }
    }

    /// Finishes hashing and returns the final hash value.
    public mutating func finish() -> UInt64 {
        self.state
    }
}
