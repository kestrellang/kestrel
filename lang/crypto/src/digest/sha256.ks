module crypto.digest

/// SHA-256 cryptographic hash function (FIPS 180-4).
///
/// Produces a 32-byte (256-bit) digest.
///
/// # Examples
///
/// ```
/// let digest = SHA256.hash(data.asSlice());
/// let hex = hexString(from: digest);
/// ```
public struct SHA256: Digest {
    var state: Array[UInt32];
    var buffer: Array[UInt8];
    var totalLen: UInt64;

    public static var digestSize: Int64 { 32 }

    public init() {
        self.state = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54f4c58,
            0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
        ];
        self.buffer = Array[UInt8]();
        self.totalLen = 0;
    }

    public mutating func update[S](bytes: S) where S: Slice[UInt8] {
        let sl = bytes.asSlice();
        self.totalLen = self.totalLen + UInt64(from: sl.count);
        var offset: Int64 = 0;

        if self.buffer.count > 0 {
            let needed = 64 - self.buffer.count;
            if sl.count < needed {
                self.buffer.append(contentsOf: sl);
                return;
            }
            self.buffer.append(contentsOf: sl.prefix(needed));
            var block = self.buffer;
            self.buffer = Array[UInt8]();
            SHA256.compress(block.asSlice(), self.state);
            offset = needed;
        }

        while offset + 64 <= sl.count {
            SHA256.compress(sl.drop(first: offset).prefix(64), self.state);
            offset = offset + 64;
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
        while buf.count % 64 != 56 {
            buf.append(0);
        }

        let bitLen = self.totalLen * 8;
        buf.append(contentsOf: bitLen.toBytesBigEndian().asSlice());

        var offset: Int64 = 0;
        while offset < buf.count {
            SHA256.compress(buf.asSlice().drop(first: offset).prefix(64), st);
            offset = offset + 64;
        }

        var result = Array[UInt8]();
        for i in 0..<8 {
            result.append(contentsOf: st(i).toBytesBigEndian().asSlice());
        }
        return DigestOutput(bytes: result);
    }

    public static func hash[S](bytes: S) -> DigestOutput where S: Slice[UInt8] {
        var h = SHA256();
        h.update(bytes);
        return h.finalize();
    }

    // --- internals ---

    static func compress(block: ArraySlice[UInt8], mutating state: Array[UInt32]) {
        let k: [UInt32] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
            0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
            0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
            0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
            0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
            0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
            0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
            0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
            0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
            0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
            0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
            0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
            0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
            0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
        ];

        var w = Array[UInt32](repeating: 0, count: 64);
        for i in 0..<16 {
            let off = i * 4;
            let b0 = UInt32(from: block(off));
            let b1 = UInt32(from: block(off + 1));
            let b2 = UInt32(from: block(off + 2));
            let b3 = UInt32(from: block(off + 3));
            w(i) = (b0 << 24) | (b1 << 16) | (b2 << 8) | b3;
        }
        for i in 16..<64 {
            let w15 = w(i - 15);
            let w2 = w(i - 2);
            let s0 = w15.rotateRight(by: 7) ^ w15.rotateRight(by: 18) ^ (w15 >> 3);
            let s1 = w2.rotateRight(by: 17) ^ w2.rotateRight(by: 19) ^ (w2 >> 10);
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

        for i in 0..<64 {
            let s1 = e.rotateRight(by: 6) ^ e.rotateRight(by: 11) ^ e.rotateRight(by: 25);
            let ch = (e & f) ^ (!e & g);
            let temp1 = h + s1 + ch + k(i) + w(i);
            let s0 = a.rotateRight(by: 2) ^ a.rotateRight(by: 13) ^ a.rotateRight(by: 22);
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
