module crypto.key

import crypto.bytes.(ByteValue)
import crypto.random.(RandomByteValue)

/// A symmetric cryptographic key.
///
/// Wraps raw key material for use with HMAC, HKDF, and ciphers. `==` is
/// constant-time and `hexString` / `init(hex:)?` exist for loading and
/// storing keys deliberately, but a key is not `Hashable` and not
/// formattable, so it can't end up in a dictionary or a log line by
/// accident.
public struct SymmetricKey: ByteValue, RandomByteValue, Equatable {
    private var storage: Array[UInt8];

    public init(bytes bytes: Array[UInt8]) {
        self.storage = bytes;
    }

    /// `SymmetricKey.random()` makes 32 bytes, which suits
    /// ChaCha20-Poly1305 and HMAC-SHA256.
    public static var randomCount: Int64 { 32 }

    /// The raw key bytes.
    public var bytes: Array[UInt8] { self.storage }

    /// The key length in bytes.
    public var count: Int64 { self.storage.count }

}
