// Default hashing implementation using FNV-1a
// FNV-1a is a non-cryptographic hash function created by Glenn Fowler, Landon Curt Noll, and Phong Vo.

module std.collections

import std.core.(Hasher, Hash, Defaultable)
import std.num.(UInt8, UInt64, Int64)
import std.memory.(Slice)

public struct DefaultHasher: Hasher, Defaultable {
    private var state: UInt64

    public init() {
        // FNV offset basis for 64-bit
        self.state = UInt64(intLiteral: 14695981039346656037);
    }

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

    public mutating func finish() -> UInt64 {
        self.state
    }
}
