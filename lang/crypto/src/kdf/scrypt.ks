module crypto.kdf

import crypto.digest.(SHA256)
import crypto.error.(CryptoError)
import crypto.key.(SymmetricKey)

/// scrypt (RFC 7914): a memory-hard password KDF. Each derivation needs
/// `2^logN × blockSize × 128` bytes of memory, which is what makes
/// GPU and ASIC cracking expensive.
///
/// OWASP's 2023 minimum is `logN: 17, blockSize: 8, parallelism: 1`
/// (128 MiB). `PasswordHash` wraps this for storing passwords.
///
/// # Examples
///
/// ```
/// let key = try Scrypt.deriveKey(password.asByteSlice(), salt: salt, logN: 17, blockSize: 8, parallelism: 1, length: 32);
/// ```
public struct Scrypt {
    /// Throws `.InvalidParameter` unless `logN` is 1–30, `blockSize` and
    /// `parallelism` are at least 1 with `blockSize × parallelism < 2^30`;
    /// throws `.InvalidLength` for a negative `length`.
    public static func deriveKey(
        password: some Slice[UInt8],
        salt salt: some Slice[UInt8],
        logN logN: Int64,
        blockSize r: Int64,
        parallelism p: Int64,
        length length: Int64
    ) -> SymmetricKey throws CryptoError {
        if logN < 1 or logN > 30 { throw CryptoError.InvalidParameter; }
        if r < 1 or p < 1 or r * p >= 1073741824 { throw CryptoError.InvalidParameter; }
        if length < 0 { throw CryptoError.InvalidLength; }

        let n: Int64 = 1 << logN;
        let words = 32 * r;   // one scrypt block of 128·r bytes, as u32 words

        // B = PBKDF2-HMAC-SHA256(P, S, 1, p · 128 · r), split into p blocks.
        let initial = try PBKDF2[SHA256].deriveKey(password, salt: salt, iterations: 1, length: p * 128 * r);
        var b = Scrypt.wordsFromBytes(initial.bytes);
        for i in 0..<p {
            Scrypt.roMix(b, i * words, r, n);
        }

        // DK = PBKDF2-HMAC-SHA256(P, B, 1, length)
        let mixed = Scrypt.bytesFromWords(b);
        PBKDF2[SHA256].deriveKey(password, salt: mixed, iterations: 1, length: length)
    }

    // --- internals ---

    /// ROMix (RFC 7914 §5) on the block at `offset`: fill N blocks of
    /// memory, then read them back in a data-dependent order.
    static func roMix(mutating b: Array[UInt32], offset: Int64, r: Int64, n: Int64) {
        let words = 32 * r;
        var x = Array[UInt32](repeating: 0, count: words);
        for k in 0..<words { x(k) = b(offset + k); }

        var v = Array[UInt32](repeating: 0, count: n * words);
        for i in 0..<n {
            for k in 0..<words { v(i * words + k) = x(k); }
            Scrypt.blockMix(x, r);
        }
        for _ in 0..<n {
            // Integerify: the first word of the last 64-byte sub-block, mod N.
            let j = Int64(from: x(words - 16)) & (n - 1);
            for k in 0..<words { x(k) = x(k) ^ v(j * words + k); }
            Scrypt.blockMix(x, r);
        }

        for k in 0..<words { b(offset + k) = x(k); }
    }

    /// BlockMix (RFC 7914 §4): Salsa20/8 over the 2r sub-blocks, chained,
    /// with even outputs first and odd outputs after.
    static func blockMix(mutating b: Array[UInt32], r: Int64) {
        var x = Array[UInt32](repeating: 0, count: 16);
        for k in 0..<16 { x(k) = b((2 * r - 1) * 16 + k); }

        var y = Array[UInt32](repeating: 0, count: 32 * r);
        for i in 0..<(2 * r) {
            for k in 0..<16 { x(k) = x(k) ^ b(i * 16 + k); }
            Scrypt.salsa8(x);
            let destination = if i % 2 == 0 { (i / 2) * 16 } else { (r + i / 2) * 16 };
            for k in 0..<16 { y(destination + k) = x(k); }
        }
        b = y;
    }

    /// The Salsa20/8 core: 4 double rounds, then add the input.
    static func salsa8(mutating x: Array[UInt32]) {
        var w = x;
        for _ in 0..<4 {
            // Columns
            Scrypt.step(w, 4, 0, 12, 7);   Scrypt.step(w, 8, 4, 0, 9);
            Scrypt.step(w, 12, 8, 4, 13);  Scrypt.step(w, 0, 12, 8, 18);
            Scrypt.step(w, 9, 5, 1, 7);    Scrypt.step(w, 13, 9, 5, 9);
            Scrypt.step(w, 1, 13, 9, 13);  Scrypt.step(w, 5, 1, 13, 18);
            Scrypt.step(w, 14, 10, 6, 7);  Scrypt.step(w, 2, 14, 10, 9);
            Scrypt.step(w, 6, 2, 14, 13);  Scrypt.step(w, 10, 6, 2, 18);
            Scrypt.step(w, 3, 15, 11, 7);  Scrypt.step(w, 7, 3, 15, 9);
            Scrypt.step(w, 11, 7, 3, 13);  Scrypt.step(w, 15, 11, 7, 18);
            // Rows
            Scrypt.step(w, 1, 0, 3, 7);    Scrypt.step(w, 2, 1, 0, 9);
            Scrypt.step(w, 3, 2, 1, 13);   Scrypt.step(w, 0, 3, 2, 18);
            Scrypt.step(w, 6, 5, 4, 7);    Scrypt.step(w, 7, 6, 5, 9);
            Scrypt.step(w, 4, 7, 6, 13);   Scrypt.step(w, 5, 4, 7, 18);
            Scrypt.step(w, 11, 10, 9, 7);  Scrypt.step(w, 8, 11, 10, 9);
            Scrypt.step(w, 9, 8, 11, 13);  Scrypt.step(w, 10, 9, 8, 18);
            Scrypt.step(w, 12, 15, 14, 7); Scrypt.step(w, 13, 12, 15, 9);
            Scrypt.step(w, 14, 13, 12, 13); Scrypt.step(w, 15, 14, 13, 18);
        }
        for i in 0..<16 { x(i) = x(i) + w(i); }
    }

    /// One Salsa20 step: `w[target] ^= (w[a] + w[b]) <<< shift`.
    static func step(mutating w: Array[UInt32], target: Int64, a: Int64, b: Int64, shift: Int64) {
        w(target) = w(target) ^ (w(a) + w(b)).rotateLeft(by: shift);
    }

    static func wordsFromBytes(bytes: Array[UInt8]) -> Array[UInt32] {
        var words = Array[UInt32](repeating: 0, count: bytes.count / 4);
        for i in 0..<words.count {
            words(i) = UInt32(from: bytes(4 * i))
                | (UInt32(from: bytes(4 * i + 1)) << 8)
                | (UInt32(from: bytes(4 * i + 2)) << 16)
                | (UInt32(from: bytes(4 * i + 3)) << 24);
        }
        words
    }

    static func bytesFromWords(words: Array[UInt32]) -> Array[UInt8] {
        var bytes: [UInt8] = [];
        for w in words {
            bytes.append(contentsOf: w.toBytesLittleEndian().asSlice());
        }
        bytes
    }
}
