module crypto.random

import crypto.bytes.(ByteValue)

/// An unguessable random token: session tokens, API keys, reset links.
///
/// `hexString`, `init(hex:)?` and a constant-time `==` come from `ByteValue`.
/// `Hashable`, so tokens can key a session table.
public struct Token: ByteValue, RandomByteValue, Equatable, Hashable {
    private var storage: Array[UInt8];

    public init(bytes bytes: Array[UInt8]) {
        self.storage = bytes;
    }

    /// `Token.random()` makes 32 bytes: 256 bits of entropy.
    public static var randomCount: Int64 { 32 }

    /// The raw token bytes.
    public var bytes: Array[UInt8] { self.storage }


    public func hash[H](mutating into hasher: H) where H: Hasher {
        hasher.write(self.storage.asSlice());
    }
}
