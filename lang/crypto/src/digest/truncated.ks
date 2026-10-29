module crypto.digest

/// SHA-224 (FIPS 180-4): SHA-256 with its own initial hash value,
/// truncated to 28 bytes.
public struct SHA224: Digest {
    var core: SHA256;

    public static var digestSize: Int64 { 28 }
    public static var blockSize: Int64 { 64 }

    public init() {
        self.core = SHA256(initialState: [
            0xc1059ed8, 0x367cd507, 0x3070dd17, 0xf70e5939,
            0xffc00b31, 0x68581511, 0x64f98fa7, 0xbefa4fa4
        ]);
    }

    public mutating func update[S](bytes: S) where S: Slice[UInt8] {
        self.core.update(bytes);
    }

    public func finalize() -> DigestOutput {
        truncated(self.core.finalize(), SHA224.digestSize)
    }
}

/// SHA-384 (FIPS 180-4): SHA-512 with its own initial hash value,
/// truncated to 48 bytes.
public struct SHA384: Digest {
    var core: SHA512;

    public static var digestSize: Int64 { 48 }
    public static var blockSize: Int64 { 128 }

    public init() {
        self.core = SHA512(initialState: [
            0xcbbb9d5dc1059ed8, 0x629a292a367cd507,
            0x9159015a3070dd17, 0x152fecd8f70e5939,
            0x67332667ffc00b31, 0x8eb44a8768581511,
            0xdb0c2e0d64f98fa7, 0x47b5481dbefa4fa4
        ]);
    }

    public mutating func update[S](bytes: S) where S: Slice[UInt8] {
        self.core.update(bytes);
    }

    public func finalize() -> DigestOutput {
        truncated(self.core.finalize(), SHA384.digestSize)
    }
}

/// SHA-512/256 (FIPS 180-4): SHA-512 with its own initial hash value,
/// truncated to 32 bytes. SHA-256-sized output, but faster than SHA-256
/// on 64-bit machines and not open to length extension.
public struct SHA512_256: Digest {
    var core: SHA512;

    public static var digestSize: Int64 { 32 }
    public static var blockSize: Int64 { 128 }

    public init() {
        self.core = SHA512(initialState: [
            0x22312194fc2bf72c, 0x9f555fa3c84c64c2,
            0x2393b86b6f53b151, 0x963877195940eabd,
            0x96283ee2a88effe3, 0xbe5e1e2553863992,
            0x2b0199fc2c85b8aa, 0x0eb72ddc81c52ca2
        ]);
    }

    public mutating func update[S](bytes: S) where S: Slice[UInt8] {
        self.core.update(bytes);
    }

    public func finalize() -> DigestOutput {
        truncated(self.core.finalize(), SHA512_256.digestSize)
    }
}

/// The first `count` bytes of a full-width digest.
func truncated(full: DigestOutput, count: Int64) -> DigestOutput {
    var result: [UInt8] = [];
    result.append(contentsOf: full.bytes.asSlice().prefix(count));
    DigestOutput(bytes: result)
}
