module crypto.kdf

import crypto.bytes.(ByteValue)
import crypto.random.(RandomByteValue)

/// A random salt for password hashing and key derivation.
///
/// At least 8 bytes (RFC 8018 §4.1); `Salt.random()` makes 16, and
/// `Salt.random(count:)` returns null below 8.
/// `hexString`, `init(hex:)?` and a constant-time `==` come from `ByteValue`.
public struct Salt: ByteValue, RandomByteValue, Equatable, Hashable {
    private var storage: Array[UInt8];

    /// The shortest salt accepted, in bytes.
    public static var minimumCount: Int64 { 8 }

    /// Creates a salt from existing bytes. Returns null if shorter than
    /// `minimumCount`.
    public init(bytes bytes: Array[UInt8])? {
        if bytes.count < Salt.minimumCount { return null; }
        self.storage = bytes;
    }

    /// `Salt.random()` makes 16 bytes.
    public static var randomCount: Int64 { 16 }

    /// The raw salt bytes.
    public var bytes: Array[UInt8] { self.storage }


    public func hash[H](mutating into hasher: H) where H: Hasher {
        hasher.write(self.storage.asSlice());
    }
}
