module crypto.key

/// A symmetric cryptographic key.
///
/// Wraps raw key material for use with HMAC, HKDF, and ciphers.
/// No `hexString` or `Hashable` — key material should not be
/// casually printed or used as dictionary keys.
public struct SymmetricKey {
    private var storage: Array[UInt8];

    public init(bytes bytes: Array[UInt8]) {
        self.storage = bytes;
    }

    /// The raw key bytes.
    public var bytes: Array[UInt8] { self.storage }

    /// The key length in bytes.
    public var count: Int64 { self.storage.count }
}
