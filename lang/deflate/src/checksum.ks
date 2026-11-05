module deflate

/// CRC-32 (IEEE 802.3, the polynomial gzip and PNG use).
///
/// # Examples
///
/// ```
/// var crc = Crc32();
/// crc.update(header);
/// crc.update(body);
/// let value = crc.value;
///
/// let once = Crc32.checksum(data);
/// ```
public struct Crc32 {
    var table: Array[UInt32];
    var state: UInt32;

    public init() {
        self.table = Crc32.makeTable();
        self.state = 0xFFFFFFFF;
    }

    /// The CRC of every byte passed to `update` so far.
    public var value: UInt32 { !self.state }

    /// Feeds more bytes into the checksum.
    public mutating func update[S](bytes: S) where S: Slice[UInt8], S: not Static {
        let data = bytes.asUnsafeSlice();
        var crc = self.state;
        for i in 0..<data.count {
            let index = Int64(from: (crc ^ UInt32(from: data(i))) & 0xFF);
            crc = self.table(index) ^ (crc >> 8);
        }
        self.state = crc;
    }

    /// The CRC-32 of `bytes` in one call.
    public static func checksum[S](bytes: S) -> UInt32 where S: Slice[UInt8], S: not Static {
        var crc = Crc32();
        crc.update(bytes);
        crc.value
    }

    // Byte-at-a-time table for the reflected polynomial 0xEDB88320.
    static func makeTable() -> Array[UInt32] {
        var table = Array[UInt32](repeating: 0, count: 256);
        for n in 0..<256 {
            var c = UInt32(from: n);
            for _ in 0..<8 {
                if (c & 1) == 1 {
                    c = 0xEDB88320 ^ (c >> 1);
                } else {
                    c = c >> 1;
                }
            }
            table(n) = c;
        }
        table
    }
}

/// Adler-32, the checksum zlib streams end with.
///
/// # Examples
///
/// ```
/// let value = Adler32.checksum(data);
/// ```
public struct Adler32 {
    var a: UInt32;
    var b: UInt32;

    public init() {
        self.a = 1;
        self.b = 0;
    }

    /// The checksum of every byte passed to `update` so far.
    public var value: UInt32 { (self.b << 16) | self.a }

    /// Feeds more bytes into the checksum.
    public mutating func update[S](bytes: S) where S: Slice[UInt8], S: not Static {
        let data = bytes.asUnsafeSlice();
        var a = self.a;
        var b = self.b;
        var i: Int64 = 0;
        // 5552 is the most bytes that can be summed before b overflows 32 bits,
        // so the modulo only runs once per chunk.
        while i < data.count {
            var end = i + 5552;
            if end > data.count { end = data.count; }
            while i < end {
                a = a + UInt32(from: data(i));
                b = b + a;
                i = i + 1;
            }
            a = a % 65521;
            b = b % 65521;
        }
        self.a = a;
        self.b = b;
    }

    /// The Adler-32 of `bytes` in one call.
    public static func checksum[S](bytes: S) -> UInt32 where S: Slice[UInt8], S: not Static {
        var adler = Adler32();
        adler.update(bytes);
        adler.value
    }
}
