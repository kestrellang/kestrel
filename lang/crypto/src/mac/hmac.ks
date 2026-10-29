module crypto.mac

import crypto.bytes.(ByteValue)
import crypto.digest.(Digest)
import crypto.key.(SymmetricKey)

/// The output of a message authentication code.
///
/// `hexString`, `init(hex:)?` and a constant-time `==` come from `ByteValue`,
/// so `tag == expected` is the right way to verify.
public struct AuthenticationCode: ByteValue, Equatable, Hashable {
    private var storage: Array[UInt8];

    public init(bytes bytes: Array[UInt8]) {
        self.storage = bytes;
    }

    /// The raw tag bytes.
    public var bytes: Array[UInt8] { self.storage }


    public func hash[H](mutating into hasher: H) where H: Hasher {
        hasher.write(self.storage.asSlice());
    }
}

/// HMAC: Keyed-hash message authentication code (RFC 2104).
///
/// Generic over any `Digest` implementation.
///
/// # Examples
///
/// ```
/// // One-shot
/// let tag = HMAC[SHA256].authenticate(data, using: key);
///
/// // Incremental
/// var mac = HMAC[SHA256](key: key);
/// mac.update(chunk1);
/// mac.update(chunk2);
/// let tag = mac.finalize();
///
/// // Verify (constant-time)
/// if tag == expected { ... }
/// ```
public struct HMAC[H] where H: Digest {
    var inner: H;
    var base: H;
    var opadKey: Array[UInt8];

    /// Creates an HMAC using a fresh hasher for the given digest type.
    public init(key key: SymmetricKey) {
        let hasher = H();
        self.opadKey = Array[UInt8]();
        self.inner = hasher;
        self.base = hasher;
        HMAC.setup(hasher, key.bytes, self.inner, self.base, self.opadKey);
    }

    /// Creates an HMAC using the provided hasher instance.
    public init(hasher: H, key key: SymmetricKey) {
        self.opadKey = Array[UInt8]();
        self.inner = hasher;
        self.base = hasher;
        HMAC.setup(hasher, key.bytes, self.inner, self.base, self.opadKey);
    }

    public mutating func update[S](bytes: S) where S: Slice[UInt8] {
        self.inner.update(bytes);
    }

    public func finalize() -> AuthenticationCode {
        let innerDigest = self.inner.finalize();
        var outer = self.base;
        outer.update(self.opadKey);
        outer.update(innerDigest.bytes);
        let result = outer.finalize();
        return AuthenticationCode(bytes: result.bytes);
    }

    /// One-shot authentication.
    public static func authenticate(message: some Slice[UInt8], using key: SymmetricKey) -> AuthenticationCode {
        var mac = HMAC[H](key: key);
        mac.update(message);
        return mac.finalize();
    }

    // --- internals ---

    static func setup(
        hasher: H,
        keyBytes: Array[UInt8],
        mutating inner: H,
        mutating base: H,
        mutating opadKey: Array[UInt8]
    ) {
        let bs = H.blockSize;

        var normKey: [UInt8] = [];
        if keyBytes.count > bs {
            var h = hasher;
            h.update(keyBytes);
            normKey = h.finalize().bytes;
        } else {
            normKey.append(contentsOf: keyBytes.asSlice());
        }

        while normKey.count < bs {
            normKey.append(0);
        }

        var ipad: [UInt8] = [];
        opadKey = [];
        for i in 0..<bs {
            ipad.append(normKey(i) ^ 0x36);
            opadKey.append(normKey(i) ^ 0x5c);
        }

        inner = hasher;
        inner.update(ipad);
        base = hasher;
    }
}
