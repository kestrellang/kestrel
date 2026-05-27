module crypto.digest

/// BLAKE2b cryptographic hash function (RFC 7693).
///
/// Produces a configurable-length digest up to 64 bytes. Defaults to 32 bytes.
///
/// # Examples
///
/// ```
/// let digest = BLAKE2b.hash(data.asSlice());
///
/// var hasher = BLAKE2b(outputLength: 64);
/// hasher.update(data.asSlice());
/// let digest = hasher.finalize();
/// ```
public struct BLAKE2b: Digest {
    var state: Array[UInt64];
    var buffer: Array[UInt8];
    var t0: UInt64;
    var t1: UInt64;
    var outputLen: Int64;

    public static var digestSize: Int64 { 32 }
    public static var blockSize: Int64 { 128 }

    public init() {
        self.outputLen = 32;
        var h = BLAKE2b.IV;
        h(0) = h(0) ^ 0x01010000 ^ 32;
        self.state = h;
        self.buffer = Array[UInt8]();
        self.t0 = 0;
        self.t1 = 0;
    }

    public init(outputLength outputLength: Int64) {
        self.outputLen = outputLength;
        var h = BLAKE2b.IV;
        h(0) = h(0) ^ 0x01010000 ^ UInt64(from: outputLength);
        self.state = h;
        self.buffer = Array[UInt8]();
        self.t0 = 0;
        self.t1 = 0;
    }

    public mutating func update[S](bytes: S) where S: Slice[UInt8] {
        let sl = bytes.asSlice();
        var offset: Int64 = 0;

        if self.buffer.count > 0 {
            let needed = 128 - self.buffer.count;
            if sl.count <= needed {
                self.buffer.append(contentsOf: sl);
                return;
            }
            self.buffer.append(contentsOf: sl.prefix(needed));
            self.incrementCounter(128);
            var block = self.buffer;
            self.buffer = Array[UInt8]();
            BLAKE2b.compress(block.asSlice(), self.state, self.t0, self.t1, false);
            offset = needed;
        }

        while offset + 128 < sl.count {
            self.incrementCounter(128);
            BLAKE2b.compress(sl.drop(first: offset).prefix(128), self.state, self.t0, self.t1, false);
            offset = offset + 128;
        }

        if offset < sl.count {
            self.buffer.append(contentsOf: sl.drop(first: offset));
        }
    }

    public func finalize() -> DigestOutput {
        var st = self.state;
        var ct0 = self.t0;
        var ct1 = self.t1;

        // Pad the remaining buffer with zeros
        var buf = Array[UInt8]();
        buf.append(contentsOf: self.buffer.asSlice());
        let remaining = UInt64(from: buf.count);

        // Increment counter by remaining bytes
        ct0 = ct0 + remaining;
        if ct0 < remaining {
            ct1 = ct1 + 1;
        }

        while buf.count < 128 {
            buf.append(0);
        }

        BLAKE2b.compress(buf.asSlice(), st, ct0, ct1, true);

        // Extract output bytes (little-endian)
        var result = Array[UInt8]();
        var bytesLeft = self.outputLen;
        var idx: Int64 = 0;
        while bytesLeft > 0 {
            let word = st(idx).toBytesLittleEndian();
            var take: Int64 = 8;
            if bytesLeft < 8 {
                take = bytesLeft;
            }
            for j in 0..<take {
                result.append(word(j));
            }
            bytesLeft = bytesLeft - take;
            idx = idx + 1;
        }
        return DigestOutput(bytes: result);
    }

    public static func hash[S](bytes: S) -> DigestOutput where S: Slice[UInt8] {
        var h = BLAKE2b();
        h.update(bytes);
        return h.finalize();
    }

    // --- internals ---

    static var IV: Array[UInt64] {
        [
            0x6a09e667f3bcc908, 0xbb67ae8584caa73b,
            0x3c6ef372fe94f82b, 0xa54ff53a5f1d36f1,
            0x510e527fade682d1, 0x9b05688c2b3e6c1f,
            0x1f83d9abfb41bd6b, 0x5be0cd19137e2179
        ]
    }

    mutating func incrementCounter(n: Int64) {
        let add = UInt64(from: n);
        self.t0 = self.t0 + add;
        if self.t0 < add {
            self.t1 = self.t1 + 1;
        }
    }

    static func compress(
        block: ArraySlice[UInt8],
        mutating state: Array[UInt64],
        t0: UInt64,
        t1: UInt64,
        last: Bool
    ) {
        let sigma: [Array[Int64]] = [
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
            [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
            [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
            [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
            [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
            [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
            [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
            [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
            [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
            [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0]
        ];

        // Load message words (little-endian)
        var m = Array[UInt64](repeating: 0, count: 16);
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
            m(i) = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24) | (b4 << 32) | (b5 << 40) | (b6 << 48) | (b7 << 56);
        }

        // Initialize working vector
        var v = Array[UInt64](repeating: 0, count: 16);
        for i in 0..<8 {
            v(i) = state(i);
        }
        let iv = BLAKE2b.IV;
        v(8) = iv(0);
        v(9) = iv(1);
        v(10) = iv(2);
        v(11) = iv(3);
        v(12) = iv(4) ^ t0;
        v(13) = iv(5) ^ t1;
        if last {
            v(14) = !iv(6);
        } else {
            v(14) = iv(6);
        }
        v(15) = iv(7);

        // 12 rounds of mixing
        for round in 0..<12 {
            let s = sigma(round % 10);

            // Column step
            BLAKE2b.mix(v, 0, 4,  8, 12, m(s(0)),  m(s(1)));
            BLAKE2b.mix(v, 1, 5,  9, 13, m(s(2)),  m(s(3)));
            BLAKE2b.mix(v, 2, 6, 10, 14, m(s(4)),  m(s(5)));
            BLAKE2b.mix(v, 3, 7, 11, 15, m(s(6)),  m(s(7)));

            // Diagonal step
            BLAKE2b.mix(v, 0, 5, 10, 15, m(s(8)),  m(s(9)));
            BLAKE2b.mix(v, 1, 6, 11, 12, m(s(10)), m(s(11)));
            BLAKE2b.mix(v, 2, 7,  8, 13, m(s(12)), m(s(13)));
            BLAKE2b.mix(v, 3, 4,  9, 14, m(s(14)), m(s(15)));
        }

        for i in 0..<8 {
            state(i) = state(i) ^ v(i) ^ v(i + 8);
        }
    }

    static func mix(
        mutating v: Array[UInt64],
        a: Int64, b: Int64, c: Int64, d: Int64,
        x: UInt64, y: UInt64
    ) {
        v(a) = v(a) + v(b) + x;
        v(d) = (v(d) ^ v(a)).rotateRight(by: 32);
        v(c) = v(c) + v(d);
        v(b) = (v(b) ^ v(c)).rotateRight(by: 24);
        v(a) = v(a) + v(b) + y;
        v(d) = (v(d) ^ v(a)).rotateRight(by: 16);
        v(c) = v(c) + v(d);
        v(b) = (v(b) ^ v(c)).rotateRight(by: 7);
    }
}
