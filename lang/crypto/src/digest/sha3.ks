module crypto.digest

/// SHA3-224 (FIPS 202).
public struct SHA3_224: Digest {
    var core: Keccak;

    public static var digestSize: Int64 { 28 }
    public static var blockSize: Int64 { 144 }

    public init() { self.core = Keccak(rate: 144, domain: Keccak.sha3Domain); }

    public mutating func update[S](bytes: S) where S: Slice[UInt8] { self.core.update(bytes.asSlice()); }

    public func finalize() -> DigestOutput { DigestOutput(bytes: self.core.squeeze(SHA3_224.digestSize)) }
}

/// SHA3-256 (FIPS 202).
public struct SHA3_256: Digest {
    var core: Keccak;

    public static var digestSize: Int64 { 32 }
    public static var blockSize: Int64 { 136 }

    public init() { self.core = Keccak(rate: 136, domain: Keccak.sha3Domain); }

    public mutating func update[S](bytes: S) where S: Slice[UInt8] { self.core.update(bytes.asSlice()); }

    public func finalize() -> DigestOutput { DigestOutput(bytes: self.core.squeeze(SHA3_256.digestSize)) }
}

/// SHA3-384 (FIPS 202).
public struct SHA3_384: Digest {
    var core: Keccak;

    public static var digestSize: Int64 { 48 }
    public static var blockSize: Int64 { 104 }

    public init() { self.core = Keccak(rate: 104, domain: Keccak.sha3Domain); }

    public mutating func update[S](bytes: S) where S: Slice[UInt8] { self.core.update(bytes.asSlice()); }

    public func finalize() -> DigestOutput { DigestOutput(bytes: self.core.squeeze(SHA3_384.digestSize)) }
}

/// SHA3-512 (FIPS 202).
public struct SHA3_512: Digest {
    var core: Keccak;

    public static var digestSize: Int64 { 64 }
    public static var blockSize: Int64 { 72 }

    public init() { self.core = Keccak(rate: 72, domain: Keccak.sha3Domain); }

    public mutating func update[S](bytes: S) where S: Slice[UInt8] { self.core.update(bytes.asSlice()); }

    public func finalize() -> DigestOutput { DigestOutput(bytes: self.core.squeeze(SHA3_512.digestSize)) }
}

/// SHAKE128 (FIPS 202): an extendable-output function — as many output
/// bytes as you ask for. 128-bit security once the output is 32 bytes or
/// more.
///
/// Not a `Digest` (its output length is chosen per call), like `BLAKE2b`.
///
/// # Examples
///
/// ```
/// let mask = SHAKE128.hash(seed, outputLength: 200).expect("200 >= 1");
/// ```
public struct SHAKE128 {
    var core: Keccak;

    public init() { self.core = Keccak(rate: 168, domain: Keccak.shakeDomain); }

    public mutating func update[S](bytes: S) where S: Slice[UInt8] { self.core.update(bytes.asSlice()); }

    /// The first `outputLength` bytes of output, or null if it's below 1.
    /// Non-mutating: absorbing more and finalizing again is fine.
    public func finalize(outputLength outputLength: Int64) -> DigestOutput? {
        if outputLength < 1 { return null; }
        DigestOutput(bytes: self.core.squeeze(outputLength))
    }

    /// One-shot SHAKE128, or null if `outputLength` is below 1.
    public static func hash(bytes: some Slice[UInt8], outputLength outputLength: Int64) -> DigestOutput? {
        var shake = SHAKE128();
        shake.update(bytes);
        shake.finalize(outputLength: outputLength)
    }
}

/// SHAKE256 (FIPS 202): like `SHAKE128`, with 256-bit security once the
/// output is 64 bytes or more.
public struct SHAKE256 {
    var core: Keccak;

    public init() { self.core = Keccak(rate: 136, domain: Keccak.shakeDomain); }

    public mutating func update[S](bytes: S) where S: Slice[UInt8] { self.core.update(bytes.asSlice()); }

    /// The first `outputLength` bytes of output, or null if it's below 1.
    public func finalize(outputLength outputLength: Int64) -> DigestOutput? {
        if outputLength < 1 { return null; }
        DigestOutput(bytes: self.core.squeeze(outputLength))
    }

    /// One-shot SHAKE256, or null if `outputLength` is below 1.
    public static func hash(bytes: some Slice[UInt8], outputLength outputLength: Int64) -> DigestOutput? {
        var shake = SHAKE256();
        shake.update(bytes);
        shake.finalize(outputLength: outputLength)
    }
}

/// The Keccak-f[1600] sponge behind SHA-3 and SHAKE: absorb `rate`-byte
/// blocks into 25 little-endian lanes, pad with the domain bits, squeeze.
///
/// `blockSize` on the SHA-3 types is this rate, which is also the block
/// size HMAC-SHA3 uses.
struct Keccak {
    var blocks: BlockBuffer[KeccakState];
    var domain: UInt8;

    /// Domain-separation bits plus the first padding bit (FIPS 202):
    /// `01` for SHA-3, `1111` for SHAKE.
    static var sha3Domain: UInt8 { 0x06 }
    static var shakeDomain: UInt8 { 0x1f }

    init(rate rate: Int64, domain domain: UInt8) {
        self.blocks = BlockBuffer(function: KeccakState(), blockLength: rate);
        self.domain = domain;
    }

    mutating func update(bytes: ArraySlice[UInt8]) {
        self.blocks.update(bytes);
    }

    /// Pads, then squeezes `count` bytes, permuting between full blocks.
    /// Works on a copy, so the sponge can keep absorbing.
    func squeeze(count: Int64) -> Array[UInt8] {
        let rate = self.blocks.blockLength;
        var state = self.blocks.function;

        // pad10*1 after the domain bits: domain ... 0x80, or one byte
        // holding both when only one byte of the block is left.
        var block: [UInt8] = [];
        block.append(contentsOf: self.blocks.pending.asSlice());
        block.append(self.domain);
        while block.count < rate {
            block.append(0);
        }
        block(rate - 1) = block(rate - 1) | 0x80;
        state.compress(block.asSlice());

        var result: [UInt8] = [];
        loop {
            for i in 0..<rate {
                if result.count == count { break; }
                let lane = state.lanes(i / 8);
                result.append(UInt8(from: (lane >> (8 * (i % 8))) & 0xff));
            }
            if result.count == count { break; }
            Keccak.permute(state.lanes);
        }
        result
    }

    /// Keccak-f[1600]: 24 rounds of θ, ρ+π, χ, ι.
    static func permute(mutating a: Array[UInt64]) {
        let roundConstants: [UInt64] = [
            0x0000000000000001, 0x0000000000008082, 0x800000000000808a, 0x8000000080008000,
            0x000000000000808b, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009,
            0x000000000000008a, 0x0000000000000088, 0x0000000080008009, 0x000000008000000a,
            0x000000008000808b, 0x800000000000008b, 0x8000000000008089, 0x8000000000008003,
            0x8000000000008002, 0x8000000000000080, 0x000000000000800a, 0x800000008000000a,
            0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008
        ];
        // ρ rotation amounts and π destinations, walked as one 24-step cycle.
        let rotations: [Int64] = [1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14, 27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44];
        let destinations: [Int64] = [10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4, 15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1];

        var c = Array[UInt64](repeating: 0, count: 5);
        for round in 0..<24 {
            // θ: XOR each column's parity into its neighbours.
            for x in 0..<5 {
                c(x) = a(x) ^ a(x + 5) ^ a(x + 10) ^ a(x + 15) ^ a(x + 20);
            }
            for x in 0..<5 {
                let d = c((x + 4) % 5) ^ c((x + 1) % 5).rotateLeft(by: 1);
                for y in 0..<5 {
                    a(y * 5 + x) = a(y * 5 + x) ^ d;
                }
            }

            // ρ and π: rotate each lane and move it to its new position.
            var carried = a(1);
            for i in 0..<24 {
                let j = destinations(i);
                let next = a(j);
                a(j) = carried.rotateLeft(by: rotations(i));
                carried = next;
            }

            // χ: the only non-linear step, row by row.
            for y in 0..<5 {
                for x in 0..<5 {
                    c(x) = a(y * 5 + x);
                }
                for x in 0..<5 {
                    a(y * 5 + x) = c(x) ^ (!c((x + 1) % 5) & c((x + 2) % 5));
                }
            }

            // ι: break the symmetry between rounds.
            a(0) = a(0) ^ roundConstants(round);
        }
    }
}

/// The 25 Keccak lanes, absorbing one rate-sized block at a time through
/// `BlockBuffer`.
struct KeccakState: BlockFunction {
    var lanes: Array[UInt64];

    init() {
        self.lanes = Array[UInt64](repeating: 0, count: 25);
    }

    /// XORs the block into the leading lanes, then permutes.
    mutating func compress(block: ArraySlice[UInt8]) {
        for i in 0..<(block.count / 8) {
            var word: UInt64 = 0;
            for j in 0..<8 {
                word = word | (UInt64(from: block(i * 8 + j)) << (8 * j));
            }
            self.lanes(i) = self.lanes(i) ^ word;
        }
        Keccak.permute(self.lanes);
    }
}
