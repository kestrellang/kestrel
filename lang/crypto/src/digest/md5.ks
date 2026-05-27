module crypto.digest

/// MD5 message-digest algorithm (RFC 1321).
///
/// Produces a 16-byte (128-bit) digest. MD5 is cryptographically broken
/// and should not be used for security purposes. It is included for
/// compatibility with legacy systems and checksums.
///
/// # Examples
///
/// ```
/// let digest = MD5.hash(data.asSlice());
/// let hex = hexString(from: digest);
/// ```
public struct MD5: Digest {
    var state: Array[UInt32];
    var buffer: Array[UInt8];
    var totalLen: UInt64;

    public static var digestSize: Int64 { 16 }

    public init() {
        self.state = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476];
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
            MD5.compress(block.asSlice(), self.state);
            offset = needed;
        }

        while offset + 64 <= sl.count {
            MD5.compress(sl.drop(first: offset).prefix(64), self.state);
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

        // MD5 uses little-endian 64-bit bit count
        let bitLen = self.totalLen * 8;
        buf.append(contentsOf: bitLen.toBytesLittleEndian().asSlice());

        var offset: Int64 = 0;
        while offset < buf.count {
            MD5.compress(buf.asSlice().drop(first: offset).prefix(64), st);
            offset = offset + 64;
        }

        // Output is little-endian
        var result = Array[UInt8]();
        for i in 0..<4 {
            result.append(contentsOf: st(i).toBytesLittleEndian().asSlice());
        }
        return DigestOutput(bytes: result);
    }

    public static func hash[S](bytes: S) -> DigestOutput where S: Slice[UInt8] {
        var h = MD5();
        h.update(bytes);
        return h.finalize();
    }

    // --- internals ---

    /// Loads a little-endian UInt32 from a byte slice at the given offset.
    static func loadLE(block: ArraySlice[UInt8], at offset: Int64) -> UInt32 {
        let b0 = UInt32(from: block(offset));
        let b1 = UInt32(from: block(offset + 1));
        let b2 = UInt32(from: block(offset + 2));
        let b3 = UInt32(from: block(offset + 3));
        return b0 | (b1 << 8) | (b2 << 16) | (b3 << 24);
    }

    static func compress(block: ArraySlice[UInt8], mutating state: Array[UInt32]) {
        let t: [UInt32] = [
            0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee,
            0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
            0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
            0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
            0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa,
            0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
            0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
            0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
            0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c,
            0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
            0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05,
            0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
            0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039,
            0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
            0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
            0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391
        ];

        let shifts: [Int64] = [
            7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
            5,  9, 14, 20, 5,  9, 14, 20, 5,  9, 14, 20, 5,  9, 14, 20,
            4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
            6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21
        ];

        // Load 16 message words (little-endian)
        var m = Array[UInt32](repeating: 0, count: 16);
        for i in 0..<16 {
            m(i) = MD5.loadLE(block, at: i * 4);
        }

        var a = state(0);
        var b = state(1);
        var c = state(2);
        var d = state(3);

        for i in 0..<64 {
            var f: UInt32 = 0;
            var g: Int64 = 0;

            if i < 16 {
                f = (b & c) | (!b & d);
                g = i;
            } else if i < 32 {
                f = (d & b) | (!d & c);
                g = (5 * i + 1) % 16;
            } else if i < 48 {
                f = b ^ c ^ d;
                g = (3 * i + 5) % 16;
            } else {
                f = c ^ (b | !d);
                g = (7 * i) % 16;
            }

            let temp = d;
            d = c;
            c = b;
            let sum = a + f + t(i) + m(g);
            b = b + sum.rotateLeft(by: shifts(i));
            a = temp;
        }

        state(0) = state(0) + a;
        state(1) = state(1) + b;
        state(2) = state(2) + c;
        state(3) = state(3) + d;
    }
}
