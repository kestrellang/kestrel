module crypto.digest

import crypto.bytes.(ByteValue)

/// The output of a cryptographic hash function.
///
/// `hexString`, `init(hex:)?` and a constant-time `==` come from `ByteValue`.
public struct DigestOutput: ByteValue, Equatable, Hashable {
    private var storage: Array[UInt8];

    public init(bytes bytes: Array[UInt8]) {
        self.storage = bytes;
    }

    /// The raw digest bytes.
    public var bytes: Array[UInt8] { self.storage }


    public func hash[H](mutating into hasher: H) where H: Hasher {
        hasher.write(self.storage.asSlice());
    }
}

/// A cryptographic hash function that maps arbitrary data to a fixed-size digest.
public protocol Digest {
    /// The size of the digest output in bytes.
    static var digestSize: Int64 { get }

    /// The internal block size in bytes. Used by HMAC and other
    /// constructions built on top of hash functions.
    static var blockSize: Int64 { get }

    /// Creates a fresh hasher with initial state.
    init()

    /// Feeds data into the hash function.
    mutating func update[S](bytes: S) where S: Slice[UInt8]

    /// Computes the final digest. The hasher state is unchanged, so
    /// you can continue calling `update` and `finalize` again to get
    /// the digest of a longer message.
    func finalize() -> DigestOutput
}

extend Digest {
    /// One-shot hash: a fresh hasher, one `update`, `finalize`. Defined
    /// once here, so it also works in generic code (`H.hash(data)`).
    public static func hash(bytes: some Slice[UInt8]) -> DigestOutput {
        var hasher = Self();
        hasher.update(bytes);
        hasher.finalize()
    }
}
