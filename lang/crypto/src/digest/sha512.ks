module crypto.digest

/// SHA-512 cryptographic hash function (FIPS 180-4).
///
/// Produces a 64-byte (512-bit) digest.
///
/// # Examples
///
/// ```
/// let digest = SHA512.hash(data.asSlice());
/// let hex = hexString(from: digest);
/// ```
public struct SHA512: Digest {
    var state: Array[UInt64];
    var buffer: Array[UInt8];
    var totalLen: UInt64;

    public static var digestSize: Int64 { 64 }
    public static var blockSize: Int64 { 128 }

    public init() {
        self.state = [
            0x6a09e667f3bcc908, 0xbb67ae8584caa73b,
            0x3c6ef372fe94f82b, 0xa54ff53a5f1d36f1,
            0x510e527fade682d1, 0x9b05688c2b3e6c1f,
            0x1f83d9abfb41bd6b, 0x5be0cd19137e2179
        ];
        self.buffer = Array[UInt8]();
        self.totalLen = 0;
    }

    public mutating func update[S](bytes: S) where S: Slice[UInt8] {
        let sl = bytes.asSlice();
        self.totalLen = self.totalLen + UInt64(from: sl.count);
        var offset: Int64 = 0;

        if self.buffer.count > 0 {
            let needed = 128 - self.buffer.count;
            if sl.count < needed {
                self.buffer.append(contentsOf: sl);
                return;
            }
            self.buffer.append(contentsOf: sl.prefix(needed));
            var block = self.buffer;
            self.buffer = Array[UInt8]();
            SHA512.compress(block.asSlice(), self.state);
            offset = needed;
        }

        while offset + 128 <= sl.count {
            SHA512.compress(sl.drop(first: offset).prefix(128), self.state);
            offset = offset + 128;
        }

        if offset < sl.count {
            self.buffer.append(contentsOf: sl.drop(first: offset));
        }
    }

    public func finalize() -> DigestOutput {
        var st = self.state;
        var buf = Array[UInt8]();
        buf.append(contentsOf: self.buffer.asSlice());

        buf.append(0x80);
        while buf.count % 128 != 112 {
            buf.append(0);
        }

        // SHA-512 uses 128-bit length; upper 64 bits are zero for practical messages
        let zeroPad: UInt64 = 0;
        buf.append(contentsOf: zeroPad.toBytesBigEndian().asSlice());
        let bitLen = self.totalLen * 8;
        buf.append(contentsOf: bitLen.toBytesBigEndian().asSlice());

        var offset: Int64 = 0;
        while offset < buf.count {
            SHA512.compress(buf.asSlice().drop(first: offset).prefix(128), st);
            offset = offset + 128;
        }

        var result = Array[UInt8]();
        for i in 0..<8 {
            result.append(contentsOf: st(i).toBytesBigEndian().asSlice());
        }
        return DigestOutput(bytes: result);
    }

    public static func hash[S](bytes: S) -> DigestOutput where S: Slice[UInt8] {
        var h = SHA512();
        h.update(bytes);
        return h.finalize();
    }

    // --- internals ---

    static func compress(block: ArraySlice[UInt8], mutating state: Array[UInt64]) {
        let k: [UInt64] = [
            0x428a2f98d728ae22, 0x7137449123ef65cd, 0xb5c0fbcfec4d3b2f, 0xe9b5dba58189dbbc,
            0x3956c25bf348b538, 0x59f111f1b605d019, 0x923f82a4af194f9b, 0xab1c5ed5da6d8118,
            0xd807aa98a3030242, 0x12835b0145706fbe, 0x243185be4ee4b28c, 0x550c7dc3d5ffb4e2,
            0x72be5d74f27b896f, 0x80deb1fe3b1696b1, 0x9bdc06a725c71235, 0xc19bf174cf692694,
            0xe49b69c19ef14ad2, 0xefbe4786384f25e3, 0x0fc19dc68b8cd5b5, 0x240ca1cc77ac9c65,
            0x2de92c6f592b0275, 0x4a7484aa6ea6e483, 0x5cb0a9dcbd41fbd4, 0x76f988da831153b5,
            0x983e5152ee66dfab, 0xa831c66d2db43210, 0xb00327c898fb213f, 0xbf597fc7beef0ee4,
            0xc6e00bf33da88fc2, 0xd5a79147930aa725, 0x06ca6351e003826f, 0x142929670a0e6e70,
            0x27b70a8546d22ffc, 0x2e1b21385c26c926, 0x4d2c6dfc5ac42aed, 0x53380d139d95b3df,
            0x650a73548baf63de, 0x766a0abb3c77b2a8, 0x81c2c92e47edaee6, 0x92722c851482353b,
            0xa2bfe8a14cf10364, 0xa81a664bbc423001, 0xc24b8b70d0f89791, 0xc76c51a30654be30,
            0xd192e819d6ef5218, 0xd69906245565a910, 0xf40e35855771202a, 0x106aa07032bbd1b8,
            0x19a4c116b8d2d0c8, 0x1e376c085141ab53, 0x2748774cdf8eeb99, 0x34b0bcb5e19b48a8,
            0x391c0cb3c5c95a63, 0x4ed8aa4ae3418acb, 0x5b9cca4f7763e373, 0x682e6ff3d6b2b8a3,
            0x748f82ee5defb2fc, 0x78a5636f43172f60, 0x84c87814a1f0ab72, 0x8cc702081a6439ec,
            0x90befffa23631e28, 0xa4506cebde82bde9, 0xbef9a3f7b2c67915, 0xc67178f2e372532b,
            0xca273eceea26619c, 0xd186b8c721c0c207, 0xeada7dd6cde0eb1e, 0xf57d4f7fee6ed178,
            0x06f067aa72176fba, 0x0a637dc5a2c898a6, 0x113f9804bef90dae, 0x1b710b35131c471b,
            0x28db77f523047d84, 0x32caab7b40c72493, 0x3c9ebe0a15c9bebc, 0x431d67c49c100d4c,
            0x4cc5d4becb3e42b6, 0x597f299cfc657e2a, 0x5fcb6fab3ad6faec, 0x6c44198c4a475817
        ];

        var w = Array[UInt64](repeating: 0, count: 80);
        for i in 0..<16 {
            let off = i * 8;
            let b0 = UInt64(from: block(off));
            let b1 = UInt64(from: block(off + 1));
            let b2 = UInt64(from: block(off + 2));
            let b3 = UInt64(from: block(off + 3));
            let b4 = UInt64(from: block(off + 4));
            let b5 = UInt64(from: block(off + 5));
            let b6 = UInt64(from: block(off + 6));
            let b7 = UInt64(from: block(off + 7));
            w(i) = (b0 << 56) | (b1 << 48) | (b2 << 40) | (b3 << 32) | (b4 << 24) | (b5 << 16) | (b6 << 8) | b7;
        }
        for i in 16..<80 {
            let w15 = w(i - 15);
            let w2 = w(i - 2);
            let s0 = w15.rotateRight(by: 1) ^ w15.rotateRight(by: 8) ^ (w15 >> 7);
            let s1 = w2.rotateRight(by: 19) ^ w2.rotateRight(by: 61) ^ (w2 >> 6);
            w(i) = w(i - 16) + s0 + w(i - 7) + s1;
        }

        var a = state(0);
        var b = state(1);
        var c = state(2);
        var d = state(3);
        var e = state(4);
        var f = state(5);
        var g = state(6);
        var h = state(7);

        for i in 0..<80 {
            let s1 = e.rotateRight(by: 14) ^ e.rotateRight(by: 18) ^ e.rotateRight(by: 41);
            let ch = (e & f) ^ (!e & g);
            let temp1 = h + s1 + ch + k(i) + w(i);
            let s0 = a.rotateRight(by: 28) ^ a.rotateRight(by: 34) ^ a.rotateRight(by: 39);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0 + maj;

            h = g;
            g = f;
            f = e;
            e = d + temp1;
            d = c;
            c = b;
            b = a;
            a = temp1 + temp2;
        }

        state(0) = state(0) + a;
        state(1) = state(1) + b;
        state(2) = state(2) + c;
        state(3) = state(3) + d;
        state(4) = state(4) + e;
        state(5) = state(5) + f;
        state(6) = state(6) + g;
        state(7) = state(7) + h;
    }
}
