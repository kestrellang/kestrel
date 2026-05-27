module crypto.aead

import crypto.key.(SymmetricKey)
import crypto.random.(randomBytes)

/// A 12-byte nonce (number used once) for AEAD ciphers.
public struct Nonce {
    private var storage: Array[UInt8];

    /// Creates a random nonce using the OS cryptographic random source.
    public init() {
        self.storage = randomBytes(count: 12);
    }

    /// Creates a nonce from exactly 12 bytes. Returns null if wrong length.
    public init(from bytes: Array[UInt8])? {
        if bytes.count != 12 { return null; }
        self.storage = bytes;
    }

    /// The raw nonce bytes.
    public var bytes: Array[UInt8] { self.storage }
}

/// The output of an AEAD seal operation.
///
/// Contains the nonce, ciphertext, and authentication tag.
public struct SealedBox {
    public var nonce: Nonce;
    public var ciphertext: Array[UInt8];
    public var tag: Array[UInt8];

    public init(nonce nonce: Nonce, ciphertext ciphertext: Array[UInt8], tag tag: Array[UInt8]) {
        self.nonce = nonce;
        self.ciphertext = ciphertext;
        self.tag = tag;
    }

    /// Serialized form: nonce (12) || ciphertext || tag (16).
    public var combined: Array[UInt8] {
        var result = Array[UInt8]();
        result.append(contentsOf: self.nonce.bytes.asSlice());
        result.append(contentsOf: self.ciphertext.asSlice());
        result.append(contentsOf: self.tag.asSlice());
        return result;
    }

    /// Reconstructs from combined bytes (nonce || ciphertext || tag).
    /// Returns null if too short (minimum 28 bytes).
    public init(combined combined: Array[UInt8])? {
        if combined.count < 28 { return null; }

        var nonceBytes = Array[UInt8]();
        for i in 0..<12 {
            nonceBytes.append(combined(i));
        }
        guard let .Some(n) = Nonce(from: nonceBytes) else { return null; }
        self.nonce = n;

        self.ciphertext = Array[UInt8]();
        let ctEnd = combined.count - 16;
        for i in 12..<ctEnd {
            self.ciphertext.append(combined(i));
        }

        self.tag = Array[UInt8]();
        for i in ctEnd..<combined.count {
            self.tag.append(combined(i));
        }
    }
}

/// Errors from cryptographic operations.
public enum CryptoError {
    case AuthenticationFailure
}

/// ChaCha20-Poly1305 authenticated encryption (RFC 8439).
///
/// # Examples
///
/// ```
/// let sealed = ChaCha20Poly1305.seal(plaintext, using: key);
/// let plaintext = try ChaCha20Poly1305.open(sealed, using: key);
/// ```
public struct ChaCha20Poly1305 {

    /// Encrypts with a random nonce.
    public static func seal(message: some Slice[UInt8], using key: SymmetricKey) -> SealedBox {
        let nonce = Nonce();
        let empty = Array[UInt8]();
        return ChaCha20Poly1305.sealWith(message.asSlice(), key, nonce, empty.asSlice());
    }

    /// Encrypts with an explicit nonce.
    public static func seal(message: some Slice[UInt8], using key: SymmetricKey, nonce nonce: Nonce) -> SealedBox {
        let empty = Array[UInt8]();
        return ChaCha20Poly1305.sealWith(message.asSlice(), key, nonce, empty.asSlice());
    }

    /// Encrypts with AAD and a random nonce.
    public static func seal(message: some Slice[UInt8], using key: SymmetricKey, authenticating aad: some Slice[UInt8]) -> SealedBox {
        let nonce = Nonce();
        return ChaCha20Poly1305.sealWith(message.asSlice(), key, nonce, aad.asSlice());
    }

    /// Encrypts with AAD and an explicit nonce.
    public static func seal(message: some Slice[UInt8], using key: SymmetricKey, nonce nonce: Nonce, authenticating aad: some Slice[UInt8]) -> SealedBox {
        return ChaCha20Poly1305.sealWith(message.asSlice(), key, nonce, aad.asSlice());
    }

    /// Decrypts and verifies. Throws on authentication failure.
    public static func open(box: SealedBox, using key: SymmetricKey) -> Array[UInt8] throws CryptoError {
        let empty = Array[UInt8]();
        return ChaCha20Poly1305.openWith(box, key, empty.asSlice());
    }

    /// Decrypts and verifies with AAD. Throws on authentication failure.
    public static func open(box: SealedBox, using key: SymmetricKey, authenticating aad: some Slice[UInt8]) -> Array[UInt8] throws CryptoError {
        return ChaCha20Poly1305.openWith(box, key, aad.asSlice());
    }

    // --- internals ---

    static func sealWith(
        plaintext: ArraySlice[UInt8],
        key: SymmetricKey,
        nonce: Nonce,
        aad: ArraySlice[UInt8]
    ) -> SealedBox {
        let keyBytes = key.bytes;
        let nonceBytes = nonce.bytes;

        // Poly1305 one-time key from block 0
        var polyKeyBlock = Array[UInt8](repeating: 0, count: 64);
        ChaCha20Poly1305.chachaBlock(keyBytes, nonceBytes, 0, polyKeyBlock);
        var polyKey = Array[UInt8]();
        for i in 0..<32 {
            polyKey.append(polyKeyBlock(i));
        }

        // Encrypt starting at counter 1
        let ct = ChaCha20Poly1305.chachaEncrypt(keyBytes, nonceBytes, 1, plaintext);

        // Compute tag
        let tag = ChaCha20Poly1305.computeTag(polyKey, aad, ct.asSlice());

        return SealedBox(nonce: nonce, ciphertext: ct, tag: tag);
    }

    static func openWith(
        box: SealedBox,
        key: SymmetricKey,
        aad: ArraySlice[UInt8]
    ) -> Array[UInt8] throws CryptoError {
        let keyBytes = key.bytes;
        let nonceBytes = box.nonce.bytes;

        // Poly1305 one-time key from block 0
        var polyKeyBlock = Array[UInt8](repeating: 0, count: 64);
        ChaCha20Poly1305.chachaBlock(keyBytes, nonceBytes, 0, polyKeyBlock);
        var polyKey = Array[UInt8]();
        for i in 0..<32 {
            polyKey.append(polyKeyBlock(i));
        }

        // Verify tag (constant-time)
        let expectedTag = ChaCha20Poly1305.computeTag(polyKey, aad, box.ciphertext.asSlice());
        var diff: UInt8 = 0;
        for i in 0..<16 {
            diff = diff | (box.tag(i) ^ expectedTag(i));
        }
        if diff != 0 {
            throw CryptoError.AuthenticationFailure;
        }

        // Decrypt
        let plaintext = ChaCha20Poly1305.chachaEncrypt(keyBytes, nonceBytes, 1, box.ciphertext.asSlice());
        return .Ok(plaintext);
    }

    // ========================================================================
    // ChaCha20 core
    // ========================================================================

    static func quarterRound(
        mutating state: Array[UInt32],
        a: Int64, b: Int64, c: Int64, d: Int64
    ) {
        state(a) = state(a) + state(b); state(d) = (state(d) ^ state(a)).rotateLeft(by: 16);
        state(c) = state(c) + state(d); state(b) = (state(b) ^ state(c)).rotateLeft(by: 12);
        state(a) = state(a) + state(b); state(d) = (state(d) ^ state(a)).rotateLeft(by: 8);
        state(c) = state(c) + state(d); state(b) = (state(b) ^ state(c)).rotateLeft(by: 7);
    }

    static func chachaBlock(
        key: Array[UInt8],
        nonce: Array[UInt8],
        counter: UInt32,
        mutating out: Array[UInt8]
    ) {
        var state: [UInt32] = [
            0x61707865, 0x3320646e, 0x79622d32, 0x6b206574,
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0
        ];

        for i in 0..<8 {
            state(4 + i) = loadLE32(key, i * 4);
        }
        state(12) = counter;
        for i in 0..<3 {
            state(13 + i) = loadLE32(nonce, i * 4);
        }

        var w = Array[UInt32](repeating: 0, count: 16);
        for i in 0..<16 {
            w(i) = state(i);
        }

        for round in 0..<10 {
            ChaCha20Poly1305.quarterRound(w, 0, 4,  8, 12);
            ChaCha20Poly1305.quarterRound(w, 1, 5,  9, 13);
            ChaCha20Poly1305.quarterRound(w, 2, 6, 10, 14);
            ChaCha20Poly1305.quarterRound(w, 3, 7, 11, 15);
            ChaCha20Poly1305.quarterRound(w, 0, 5, 10, 15);
            ChaCha20Poly1305.quarterRound(w, 1, 6, 11, 12);
            ChaCha20Poly1305.quarterRound(w, 2, 7,  8, 13);
            ChaCha20Poly1305.quarterRound(w, 3, 4,  9, 14);
        }

        for i in 0..<16 {
            let val = w(i) + state(i);
            let off = i * 4;
            out(off) = UInt8(from: val);
            out(off + 1) = UInt8(from: val >> 8);
            out(off + 2) = UInt8(from: val >> 16);
            out(off + 3) = UInt8(from: val >> 24);
        }
    }

    static func chachaEncrypt(
        key: Array[UInt8],
        nonce: Array[UInt8],
        startCounter: UInt32,
        data: ArraySlice[UInt8]
    ) -> Array[UInt8] {
        var result = Array[UInt8]();
        var block = Array[UInt8](repeating: 0, count: 64);
        var counter = startCounter;
        var offset: Int64 = 0;

        while offset < data.count {
            ChaCha20Poly1305.chachaBlock(key, nonce, counter, block);
            counter = counter + 1;
            var j: Int64 = 0;
            while j < 64 and offset < data.count {
                result.append(data(offset) ^ block(j));
                offset = offset + 1;
                j = j + 1;
            }
        }
        return result;
    }

    // ========================================================================
    // Poly1305 MAC
    // ========================================================================

    static func computeTag(
        key: Array[UInt8],
        aad: ArraySlice[UInt8],
        ciphertext: ArraySlice[UInt8]
    ) -> Array[UInt8] {
        // r (clamped) and s from the 32-byte key
        var r0 = loadLE32(key, 0);
        var r1 = loadLE32(key, 4);
        var r2 = loadLE32(key, 8);
        var r3 = loadLE32(key, 12);
        r0 = r0 & 0x0fffffff;
        r1 = r1 & 0x0ffffffc;
        r2 = r2 & 0x0ffffffc;
        r3 = r3 & 0x0ffffffc;
        let s0 = loadLE32(key, 16);
        let s1 = loadLE32(key, 20);
        let s2 = loadLE32(key, 24);
        let s3 = loadLE32(key, 28);

        // Accumulator (5 x UInt64 limbs for 130-bit arithmetic)
        var a0: UInt64 = 0;
        var a1: UInt64 = 0;
        var a2: UInt64 = 0;
        var a3: UInt64 = 0;
        var a4: UInt64 = 0;

        let rr0 = UInt64(from: r0);
        let rr1 = UInt64(from: r1);
        let rr2 = UInt64(from: r2);
        let rr3 = UInt64(from: r3);
        let ss1 = rr1 * 5;
        let ss2 = rr2 * 5;
        let ss3 = rr3 * 5;

        // Process AAD
        ChaCha20Poly1305.poly1305Process(aad, a0, a1, a2, a3, a4, rr0, rr1, rr2, rr3, ss1, ss2, ss3);

        // Pad AAD
        let aadPad = (16 - (aad.count % 16)) % 16;
        if aadPad > 0 {
            let zeros = Array[UInt8](repeating: 0, count: aadPad);
            ChaCha20Poly1305.poly1305Process(zeros.asSlice(), a0, a1, a2, a3, a4, rr0, rr1, rr2, rr3, ss1, ss2, ss3);
        }

        // Process ciphertext
        ChaCha20Poly1305.poly1305Process(ciphertext, a0, a1, a2, a3, a4, rr0, rr1, rr2, rr3, ss1, ss2, ss3);

        // Pad ciphertext
        let ctPad = (16 - (ciphertext.count % 16)) % 16;
        if ctPad > 0 {
            let zeros = Array[UInt8](repeating: 0, count: ctPad);
            ChaCha20Poly1305.poly1305Process(zeros.asSlice(), a0, a1, a2, a3, a4, rr0, rr1, rr2, rr3, ss1, ss2, ss3);
        }

        // Append lengths
        var lengths = Array[UInt8]();
        lengths.append(contentsOf: UInt64(from: aad.count).toBytesLittleEndian().asSlice());
        lengths.append(contentsOf: UInt64(from: ciphertext.count).toBytesLittleEndian().asSlice());
        ChaCha20Poly1305.poly1305Process(lengths.asSlice(), a0, a1, a2, a3, a4, rr0, rr1, rr2, rr3, ss1, ss2, ss3);

        // Finalize: acc + s mod 2^128
        var f: UInt64 = a0 + UInt64(from: s0);
        let t0 = UInt32(from: f & 0xffffffff);
        f = (f >> 32) + a1 + UInt64(from: s1);
        let t1 = UInt32(from: f & 0xffffffff);
        f = (f >> 32) + a2 + UInt64(from: s2);
        let t2 = UInt32(from: f & 0xffffffff);
        f = (f >> 32) + a3 + UInt64(from: s3);
        let t3 = UInt32(from: f & 0xffffffff);

        var tag = Array[UInt8]();
        tag.append(contentsOf: t0.toBytesLittleEndian().asSlice());
        tag.append(contentsOf: t1.toBytesLittleEndian().asSlice());
        tag.append(contentsOf: t2.toBytesLittleEndian().asSlice());
        tag.append(contentsOf: t3.toBytesLittleEndian().asSlice());
        return tag;
    }

    static func poly1305Process(
        data: ArraySlice[UInt8],
        mutating a0: UInt64, mutating a1: UInt64,
        mutating a2: UInt64, mutating a3: UInt64,
        mutating a4: UInt64,
        r0: UInt64, r1: UInt64, r2: UInt64, r3: UInt64,
        s1: UInt64, s2: UInt64, s3: UInt64
    ) {
        var offset: Int64 = 0;
        while offset + 16 <= data.count {
            let n0 = UInt64(from: loadLE32Slice(data, offset));
            let n1 = UInt64(from: loadLE32Slice(data, offset + 4));
            let n2 = UInt64(from: loadLE32Slice(data, offset + 8));
            let n3 = UInt64(from: loadLE32Slice(data, offset + 12));

            a0 = a0 + n0;
            a1 = a1 + n1;
            a2 = a2 + n2;
            a3 = a3 + n3;
            a4 = a4 + 1;

            ChaCha20Poly1305.poly1305Mul(a0, a1, a2, a3, a4, r0, r1, r2, r3, s1, s2, s3);
            offset = offset + 16;
        }

        if offset < data.count {
            var block = Array[UInt8](repeating: 0, count: 17);
            var i: Int64 = 0;
            while offset + i < data.count {
                block(i) = data(offset + i);
                i = i + 1;
            }
            block(i) = 1;

            let n0 = UInt64(from: loadLE32(block, 0));
            let n1 = UInt64(from: loadLE32(block, 4));
            let n2 = UInt64(from: loadLE32(block, 8));
            let n3 = UInt64(from: loadLE32(block, 12));

            a0 = a0 + n0;
            a1 = a1 + n1;
            a2 = a2 + n2;
            a3 = a3 + n3;

            ChaCha20Poly1305.poly1305Mul(a0, a1, a2, a3, a4, r0, r1, r2, r3, s1, s2, s3);
        }
    }

    static func poly1305Mul(
        mutating a0: UInt64, mutating a1: UInt64,
        mutating a2: UInt64, mutating a3: UInt64,
        mutating a4: UInt64,
        r0: UInt64, r1: UInt64, r2: UInt64, r3: UInt64,
        s1: UInt64, s2: UInt64, s3: UInt64
    ) {
        var d0 = a0 * r0 + a1 * s3 + a2 * s2 + a3 * s1;
        var d1 = a0 * r1 + a1 * r0 + a2 * s3 + a3 * s2;
        var d2 = a0 * r2 + a1 * r1 + a2 * r0 + a3 * s3;
        var d3 = a0 * r3 + a1 * r2 + a2 * r1 + a3 * r0;
        var d4 = a4 * r0;

        var c: UInt64 = d0 >> 32;
        a0 = d0 & 0xffffffff;
        d1 = d1 + c;
        c = d1 >> 32;
        a1 = d1 & 0xffffffff;
        d2 = d2 + c;
        c = d2 >> 32;
        a2 = d2 & 0xffffffff;
        d3 = d3 + c;
        c = d3 >> 32;
        a3 = d3 & 0xffffffff;
        d4 = d4 + c;
        a4 = d4 & 3;
        let carry = (d4 >> 2) * 5;
        a0 = a0 + carry;
        c = a0 >> 32;
        a0 = a0 & 0xffffffff;
        a1 = a1 + c;
    }

    // ========================================================================
    // Helpers
    // ========================================================================

    static func loadLE32(bytes: Array[UInt8], offset: Int64) -> UInt32 {
        let b0 = UInt32(from: bytes(offset));
        let b1 = UInt32(from: bytes(offset + 1));
        let b2 = UInt32(from: bytes(offset + 2));
        let b3 = UInt32(from: bytes(offset + 3));
        return b0 | (b1 << 8) | (b2 << 16) | (b3 << 24);
    }

    static func loadLE32Slice(data: ArraySlice[UInt8], offset: Int64) -> UInt32 {
        let b0 = UInt32(from: data(offset));
        let b1 = UInt32(from: data(offset + 1));
        let b2 = UInt32(from: data(offset + 2));
        let b3 = UInt32(from: data(offset + 3));
        return b0 | (b1 << 8) | (b2 << 16) | (b3 << 24);
    }
}
