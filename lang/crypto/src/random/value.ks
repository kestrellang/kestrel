module crypto.random

import crypto.bytes.(ByteValue)

/// A `ByteValue` that can be generated fresh from the OS random source:
/// keys, nonces, salts, tokens.
///
/// A conforming type only declares `randomCount`; `random()` and
/// `random(count:)` are defined once here.
public protocol RandomByteValue: ByteValue {
    /// The length `random()` generates, in bytes.
    static var randomCount: Int64 { get }
}

extend RandomByteValue {
    /// A fresh value of `randomCount` bytes.
    public static func random() -> Self {
        Self.random(count: Self.randomCount).expect("randomCount is a valid length for the type")
    }

    /// A fresh value of `count` bytes, or null if `count` isn't a valid
    /// length for the type (e.g. anything but 12 for a `Nonce`).
    public static func random(count count: Int64) -> Self? {
        if count < 0 { return null; }
        Self(bytes: randomBytes(count: count))
    }
}
