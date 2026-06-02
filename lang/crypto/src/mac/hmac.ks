module crypto.mac

import crypto.digest.(Digest)
import crypto.key.(SymmetricKey)

/// The output of a message authentication code.
///
/// Constant-time equality comparison prevents timing side-channel attacks.
public struct AuthenticationCode: Equatable, Hashable {
    private var storage: Array[UInt8];

    public init(bytes bytes: Array[UInt8]) {
        self.storage = bytes;
    }

    /// The raw tag bytes.
    public var bytes: Array[UInt8] { self.storage }

    /// The tag as a lowercase hexadecimal string.
    public var hexString: String {
        var result = String();
        for b in self.storage {
            result.append(macHexNibble(Int64(from: b) >> 4));
            result.append(macHexNibble(Int64(from: b) & 0x0f));
        }
        return result;
    }

    /// Constant-time comparison to prevent timing side-channel attacks.
    public func equals(other: AuthenticationCode) -> Bool {
        if self.storage.count != other.storage.count {
            return false;
        }
        var diff: UInt8 = 0;
        for i in 0..<self.storage.count {
            diff = diff | (self.storage(i) ^ other.storage(i));
        }
        return diff == 0;
    }

    /// `Equatable` conformance. Delegates to `equals` so `==` stays
    /// constant-time and timing-safe.
    public func isEqual(to other: AuthenticationCode) -> Bool {
        self.equals(other)
    }

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
/// let tag = HMAC[SHA256].authenticate(key: key, message: data);
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
    public static func authenticate[S](key key: SymmetricKey, message message: S) -> AuthenticationCode where S: Slice[UInt8] {
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

        var normKey = Array[UInt8]();
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

        var ipad = Array[UInt8]();
        opadKey = Array[UInt8]();
        for i in 0..<bs {
            ipad.append(normKey(i) ^ 0x36);
            opadKey.append(normKey(i) ^ 0x5c);
        }

        inner = hasher;
        inner.update(ipad);
        base = hasher;
    }
}

func macHexNibble(n: Int64) -> String {
    match n {
        0 => "0",
        1 => "1",
        2 => "2",
        3 => "3",
        4 => "4",
        5 => "5",
        6 => "6",
        7 => "7",
        8 => "8",
        9 => "9",
        10 => "a",
        11 => "b",
        12 => "c",
        13 => "d",
        14 => "e",
        _ => "f"
    }
}
