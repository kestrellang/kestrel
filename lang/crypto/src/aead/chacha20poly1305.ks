module crypto.aead

import crypto.bytes.(ByteValue)
import crypto.key.(SymmetricKey)
import crypto.error.(CryptoError)
import crypto.random.(RandomByteValue)

/// A 12-byte nonce (number used once) for AEAD ciphers.
///
/// `hexString`, `init(hex:)?` and a constant-time `==` come from `ByteValue`.
public struct Nonce: ByteValue, RandomByteValue, Equatable {
    private var storage: Array[UInt8];

    /// Creates a nonce from exactly 12 bytes. Returns null if wrong length.
    public init(bytes bytes: Array[UInt8])? {
        if bytes.count != 12 { return null; }
        self.storage = bytes;
    }

    /// `Nonce.random()` makes the required 12 bytes.
    public static var randomCount: Int64 { 12 }

    /// The raw nonce bytes.
    public var bytes: Array[UInt8] { self.storage }

}

/// The output of an AEAD seal operation.
///
/// Contains the nonce, ciphertext, and authentication tag. The tag is
/// always exactly 16 bytes: the fields are read-only and every
/// initializer checks the length, so `open` can rely on it.
public struct SealedBox {
    private var nonceStorage: Nonce;
    private var ciphertextStorage: Array[UInt8];
    private var tagStorage: Array[UInt8];

    /// Creates a box from its parts. Returns null if `tag` is not 16 bytes.
    public init(nonce nonce: Nonce, ciphertext ciphertext: Array[UInt8], tag tag: Array[UInt8])? {
        if tag.count != 16 { return null; }
        self.nonceStorage = nonce;
        self.ciphertextStorage = ciphertext;
        self.tagStorage = tag;
    }

    /// Reconstructs from combined bytes (nonce || ciphertext || tag).
    /// Returns null if too short (minimum 28 bytes).
    public init(combined combined: Array[UInt8])? {
        if combined.count < 28 { return null; }

        var nonceBytes: [UInt8] = [];
        for i in 0..<12 {
            nonceBytes.append(combined(i));
        }
        guard let some n = Nonce(bytes: nonceBytes) else { return null; }
        self.nonceStorage = n;

        self.ciphertextStorage = Array[UInt8]();
        let ctEnd = combined.count - 16;
        for i in 12..<ctEnd {
            self.ciphertextStorage.append(combined(i));
        }

        self.tagStorage = Array[UInt8]();
        for i in ctEnd..<combined.count {
            self.tagStorage.append(combined(i));
        }
    }

    /// Builds a box from a tag `seal` just computed, which is 16 bytes
    /// by construction.
    fileprivate init(sealed nonce: Nonce, ciphertext ciphertext: Array[UInt8], tag tag: Array[UInt8]) {
        self.nonceStorage = nonce;
        self.ciphertextStorage = ciphertext;
        self.tagStorage = tag;
    }

    /// The nonce the message was sealed with.
    public var nonce: Nonce { self.nonceStorage }

    /// The encrypted message.
    public var ciphertext: Array[UInt8] { self.ciphertextStorage }

    /// The 16-byte Poly1305 authentication tag.
    public var tag: Array[UInt8] { self.tagStorage }

    /// Serialized form: nonce (12) || ciphertext || tag (16).
    public var combined: Array[UInt8] {
        var result: [UInt8] = [];
        result.append(contentsOf: self.nonceStorage.bytes.asUnsafeSlice());
        result.append(contentsOf: self.ciphertextStorage.asUnsafeSlice());
        result.append(contentsOf: self.tagStorage.asUnsafeSlice());
        return result;
    }
}

/// ChaCha20-Poly1305 authenticated encryption (RFC 8439).
///
/// # Examples
///
/// ```
/// let sealed = try ChaCha20Poly1305.seal(plaintext, using: key);
/// let plaintext = try ChaCha20Poly1305.open(sealed, using: key);
/// ```
public struct ChaCha20Poly1305 {
    /// The required key length in bytes.
    public static var keySize: Int64 { 32 }

    /// Encrypts `message`, and authenticates (without encrypting) `aad`
    /// when given. `nonce` defaults to a fresh random one, which is what you
    /// want unless a protocol dictates the nonce. Throws `.InvalidKeySize`
    /// unless the key is 32 bytes.
    public static func seal(
        message: &some Sliceable[UInt8] and not Static and not Copyable and not Sized,
        using key: SymmetricKey,
        authenticating aad: &some Sliceable[UInt8] and not Static and not Copyable and not Sized = Array[UInt8](),
        nonce nonce: Nonce = Nonce.random()
    ) -> SealedBox throws CryptoError {
        return ChaCha20Poly1305.sealWith(message.asUnsafeSlice(), key, nonce, aad.asUnsafeSlice());
    }

    /// Decrypts and verifies `box` and `aad` (empty unless given). Throws
    /// `.InvalidKeySize` unless the key is 32 bytes, and
    /// `.AuthenticationFailure` if the tag doesn't match.
    public static func open(
        box: SealedBox,
        using key: SymmetricKey,
        authenticating aad: &some Sliceable[UInt8] and not Static and not Copyable and not Sized = Array[UInt8]()
    ) -> Array[UInt8] throws CryptoError {
        return ChaCha20Poly1305.openWith(box, key, aad.asUnsafeSlice());
    }

    // --- internals ---

    static func sealWith(
        plaintext: UnsafeSlice[UInt8],
        key: SymmetricKey,
        nonce: Nonce,
        aad: UnsafeSlice[UInt8]
    ) -> SealedBox throws CryptoError {
        if key.count != ChaCha20Poly1305.keySize { throw CryptoError.InvalidKeySize; }
        let keyBytes = key.bytes;
        let nonceBytes = nonce.bytes;

        // Poly1305 one-time key from block 0
        var polyKeyBlock = Array[UInt8](repeating: 0, count: 64);
        ChaCha20Poly1305.chachaBlock(keyBytes, nonceBytes, 0, polyKeyBlock);
        var polyKey: [UInt8] = [];
        for i in 0..<32 {
            polyKey.append(polyKeyBlock(i));
        }

        // Encrypt starting at counter 1
        let ct = ChaCha20Poly1305.chachaEncrypt(keyBytes, nonceBytes, 1, plaintext);

        // Compute tag
        let tag = ChaCha20Poly1305.computeTag(polyKey, aad, ct.asUnsafeSlice());

        return .Ok(SealedBox(sealed: nonce, ciphertext: ct, tag: tag));
    }

    static func openWith(
        box: SealedBox,
        key: SymmetricKey,
        aad: UnsafeSlice[UInt8]
    ) -> Array[UInt8] throws CryptoError {
        if key.count != ChaCha20Poly1305.keySize { throw CryptoError.InvalidKeySize; }
        let keyBytes = key.bytes;
        let nonceBytes = box.nonce.bytes;

        // Poly1305 one-time key from block 0
        var polyKeyBlock = Array[UInt8](repeating: 0, count: 64);
        ChaCha20Poly1305.chachaBlock(keyBytes, nonceBytes, 0, polyKeyBlock);
        var polyKey: [UInt8] = [];
        for i in 0..<32 {
            polyKey.append(polyKeyBlock(i));
        }

        // Verify tag (constant-time)
        let ciphertext = box.ciphertext;
        let expectedTag = ChaCha20Poly1305.computeTag(polyKey, aad, ciphertext.asUnsafeSlice());
        let tag = box.tag;  // 16 bytes: SealedBox enforces it
        var diff: UInt8 = 0;
        for i in 0..<16 {
            diff = diff | (tag(i) ^ expectedTag(i));
        }
        if diff != 0 {
            throw CryptoError.AuthenticationFailure;
        }

        // Decrypt
        let plaintext = ChaCha20Poly1305.chachaEncrypt(keyBytes, nonceBytes, 1, ciphertext.asUnsafeSlice());
        return .Ok(plaintext);
    }

    // ========================================================================
    // ChaCha20 core
    // ========================================================================

    static func quarterRound(
        mutating state: Array[UInt32],
        a: Int64, b: Int64, c: Int64, d: Int64
    ) {
        state(a) = state(a).addWrapping(state(b)); state(d) = (state(d) ^ state(a)).rotateLeft(by: 16);
        state(c) = state(c).addWrapping(state(d)); state(b) = (state(b) ^ state(c)).rotateLeft(by: 12);
        state(a) = state(a).addWrapping(state(b)); state(d) = (state(d) ^ state(a)).rotateLeft(by: 8);
        state(c) = state(c).addWrapping(state(d)); state(b) = (state(b) ^ state(c)).rotateLeft(by: 7);
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
            let val = w(i).addWrapping(state(i));
            let off = i * 4;
            out(off) = UInt8(truncating: val);
            out(off + 1) = UInt8(truncating: val >> 8);
            out(off + 2) = UInt8(truncating: val >> 16);
            out(off + 3) = UInt8(truncating: val >> 24);
        }
    }

    static func chachaEncrypt(
        key: Array[UInt8],
        nonce: Array[UInt8],
        startCounter: UInt32,
        data: UnsafeSlice[UInt8]
    ) -> Array[UInt8] {
        var result: [UInt8] = [];
        var block = Array[UInt8](repeating: 0, count: 64);
        var counter = startCounter;
        var offset: Int64 = 0;

        while offset < data.count {
            ChaCha20Poly1305.chachaBlock(key, nonce, counter, block);
            counter = counter.addWrapping(1);
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

    /// RFC 8439 §2.8 tag: Poly1305 over `aad ‖ pad16 ‖ ciphertext ‖ pad16 ‖
    /// len(aad) ‖ len(ciphertext)`, as ONE message — the pads make every block
    /// full, so only the length-free spec construction is valid here.
    static func computeTag(
        key: Array[UInt8],
        aad: UnsafeSlice[UInt8],
        ciphertext: UnsafeSlice[UInt8]
    ) -> Array[UInt8] {
        var mac: [UInt8] = [];
        mac.append(contentsOf: aad);
        while mac.count % 16 != 0 { mac.append(0); }
        mac.append(contentsOf: ciphertext);
        while mac.count % 16 != 0 { mac.append(0); }
        mac.append(contentsOf: UInt64(truncating: aad.count).toBytesLittleEndian().asUnsafeSlice());
        mac.append(contentsOf: UInt64(truncating: ciphertext.count).toBytesLittleEndian().asUnsafeSlice());
        ChaCha20Poly1305.poly1305(key, mac.asUnsafeSlice())
    }

    /// Poly1305 (RFC 8439 §2.5) with five 26-bit limbs, after poly1305-donna-32.
    /// 26-bit limbs keep every product below 2^55 and each five-term sum below
    /// 2^58, so the 130-bit arithmetic never overflows a `UInt64`.
    static func poly1305(key: Array[UInt8], message: UnsafeSlice[UInt8]) -> Array[UInt8] {
        let mask: UInt64 = 0x3ffffff;
        // r, clamped, split into 26-bit limbs; s_i = 5 r_i folds 2^130 ≡ 5.
        let r0 = UInt64(from: loadLE32(key, 0)) & 0x3ffffff;
        let r1 = (UInt64(from: loadLE32(key, 3)) >> 2) & 0x3ffff03;
        let r2 = (UInt64(from: loadLE32(key, 6)) >> 4) & 0x3ffc0ff;
        let r3 = (UInt64(from: loadLE32(key, 9)) >> 6) & 0x3f03fff;
        let r4 = (UInt64(from: loadLE32(key, 12)) >> 8) & 0x00fffff;
        let s1 = r1.multiplyWrapping(5);
        let s2 = r2.multiplyWrapping(5);
        let s3 = r3.multiplyWrapping(5);
        let s4 = r4.multiplyWrapping(5);

        var h0: UInt64 = 0;
        var h1: UInt64 = 0;
        var h2: UInt64 = 0;
        var h3: UInt64 = 0;
        var h4: UInt64 = 0;

        var offset: Int64 = 0;
        while offset < message.count {
            // A full block gets the 2^128 bit (1 << 24 in limb 4); the final
            // partial block is padded with 0x01 then zeros instead.
            var block = Array[UInt8](repeating: 0, count: 16);
            var hibit: UInt64 = 1 << 24;
            let len = message.count - offset;
            if len >= 16 {
                for i in 0..<16 { block(i) = message(offset + i); }
            } else {
                for i in 0..<len { block(i) = message(offset + i); }
                block(len) = 1;
                hibit = 0;
            }

            h0 = h0.addWrapping((UInt64(from: loadLE32(block, 0)) & mask));
            h1 = h1.addWrapping(((UInt64(from: loadLE32(block, 3)) >> 2) & mask));
            h2 = h2.addWrapping(((UInt64(from: loadLE32(block, 6)) >> 4) & mask));
            h3 = h3.addWrapping(((UInt64(from: loadLE32(block, 9)) >> 6) & mask));
            h4 = h4.addWrapping(((UInt64(from: loadLE32(block, 12)) >> 8) | hibit));

            // h *= r (mod 2^130 - 5)
            let d0 = h0.multiplyWrapping(r0).addWrapping(h1.multiplyWrapping(s4)).addWrapping(h2.multiplyWrapping(s3)).addWrapping(h3.multiplyWrapping(s2)).addWrapping(h4.multiplyWrapping(s1));
            var d1 = h0.multiplyWrapping(r1).addWrapping(h1.multiplyWrapping(r0)).addWrapping(h2.multiplyWrapping(s4)).addWrapping(h3.multiplyWrapping(s3)).addWrapping(h4.multiplyWrapping(s2));
            var d2 = h0.multiplyWrapping(r2).addWrapping(h1.multiplyWrapping(r1)).addWrapping(h2.multiplyWrapping(r0)).addWrapping(h3.multiplyWrapping(s4)).addWrapping(h4.multiplyWrapping(s3));
            var d3 = h0.multiplyWrapping(r3).addWrapping(h1.multiplyWrapping(r2)).addWrapping(h2.multiplyWrapping(r1)).addWrapping(h3.multiplyWrapping(r0)).addWrapping(h4.multiplyWrapping(s4));
            var d4 = h0.multiplyWrapping(r4).addWrapping(h1.multiplyWrapping(r3)).addWrapping(h2.multiplyWrapping(r2)).addWrapping(h3.multiplyWrapping(r1)).addWrapping(h4.multiplyWrapping(r0));

            var c = d0 >> 26;
            h0 = d0 & mask;
            d1 = d1.addWrapping(c); c = d1 >> 26; h1 = d1 & mask;
            d2 = d2.addWrapping(c); c = d2 >> 26; h2 = d2 & mask;
            d3 = d3.addWrapping(c); c = d3 >> 26; h3 = d3 & mask;
            d4 = d4.addWrapping(c); c = d4 >> 26; h4 = d4 & mask;
            h0 = h0.addWrapping(c.multiplyWrapping(5)); c = h0 >> 26; h0 = h0 & mask;
            h1 = h1.addWrapping(c);

            offset = offset + 16;
        }

        // Fully carry h.
        var c = h1 >> 26; h1 = h1 & mask;
        h2 = h2.addWrapping(c); c = h2 >> 26; h2 = h2 & mask;
        h3 = h3.addWrapping(c); c = h3 >> 26; h3 = h3 & mask;
        h4 = h4.addWrapping(c); c = h4 >> 26; h4 = h4 & mask;
        h0 = h0.addWrapping(c.multiplyWrapping(5)); c = h0 >> 26; h0 = h0 & mask;
        h1 = h1.addWrapping(c);

        // g = h + 5 - 2^130; if it doesn't go negative, h >= p and g is h mod p.
        var g0 = h0.addWrapping(5); c = g0 >> 26; g0 = g0 & mask;
        var g1 = h1.addWrapping(c); c = g1 >> 26; g1 = g1 & mask;
        var g2 = h2.addWrapping(c); c = g2 >> 26; g2 = g2 & mask;
        var g3 = h3.addWrapping(c); c = g3 >> 26; g3 = g3 & mask;
        let top = h4.addWrapping(c);
        if top >= 1 << 26 {
            h0 = g0; h1 = g1; h2 = g2; h3 = g3; h4 = top.subtractWrapping((1 << 26));
        }

        // Repack into four 32-bit words and add s (mod 2^128).
        let w0 = (h0 | (h1 << 26)) & 0xffffffff;
        let w1 = ((h1 >> 6) | (h2 << 20)) & 0xffffffff;
        let w2 = ((h2 >> 12) | (h3 << 14)) & 0xffffffff;
        let w3 = ((h3 >> 18) | (h4 << 8)) & 0xffffffff;

        var f = w0.addWrapping(UInt64(from: loadLE32(key, 16)));
        let t0 = UInt32(truncating: f & 0xffffffff);
        f = (f >> 32).addWrapping(w1).addWrapping(UInt64(from: loadLE32(key, 20)));
        let t1 = UInt32(truncating: f & 0xffffffff);
        f = (f >> 32).addWrapping(w2).addWrapping(UInt64(from: loadLE32(key, 24)));
        let t2 = UInt32(truncating: f & 0xffffffff);
        f = (f >> 32).addWrapping(w3).addWrapping(UInt64(from: loadLE32(key, 28)));
        let t3 = UInt32(truncating: f & 0xffffffff);

        var tag: [UInt8] = [];
        tag.append(contentsOf: t0.toBytesLittleEndian().asUnsafeSlice());
        tag.append(contentsOf: t1.toBytesLittleEndian().asUnsafeSlice());
        tag.append(contentsOf: t2.toBytesLittleEndian().asUnsafeSlice());
        tag.append(contentsOf: t3.toBytesLittleEndian().asUnsafeSlice());
        tag
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
}
