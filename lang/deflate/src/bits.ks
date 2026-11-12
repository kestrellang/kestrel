module deflate

/// Reads bits least-significant first, the order DEFLATE packs them.
///
/// Reading past the end yields zero bits rather than failing, so the hot
/// decode loop needs no error checks per bit. Callers check `isOverrun`
/// once per symbol instead.
struct BitReader {
    let data: UnsafeSlice[UInt8];
    var position: Int64;
    var buffer: UInt64;
    var bitCount: Int64;
    // Zero bytes added past the end of `data`.
    var paddingBytes: Int64;

    /// Starts reading `bitOffset` bits into `data`.
    init(data data: UnsafeSlice[UInt8], bitOffset bitOffset: Int64) {
        self.data = data;
        self.position = bitOffset / 8;
        self.buffer = 0;
        self.bitCount = 0;
        self.paddingBytes = 0;
        let skip = bitOffset % 8;
        if skip > 0 {
            self.refill();
            self.consume(skip);
        }
    }

    /// How many bits of `data` have been consumed. Saving and restoring
    /// this is how decoders back up when a symbol is cut off mid-input.
    var bitOffset: Int64 { (self.position + self.paddingBytes) * 8 - self.bitCount }

    /// True once a read consumed bits that weren't in the input.
    var isOverrun: Bool { self.paddingBytes * 8 > self.bitCount }

    /// Tops the buffer up to at least 57 bits.
    mutating func refill() {
        while self.bitCount <= 56 {
            var byte: UInt64 = 0;
            if self.position < self.data.count {
                byte = UInt64(from: self.data(self.position));
                self.position = self.position + 1;
            } else {
                self.paddingBytes = self.paddingBytes + 1;
            }
            self.buffer = self.buffer | (byte << self.bitCount);
            self.bitCount = self.bitCount + 8;
        }
    }

    /// The next `count` bits without consuming them. `count` must be at most 32.
    mutating func peek(count: Int64) -> UInt64 {
        if self.bitCount < count { self.refill(); }
        let one: UInt64 = 1;
        self.buffer & ((one << count) - 1)
    }

    mutating func consume(count: Int64) {
        self.buffer = self.buffer >> count;
        self.bitCount = self.bitCount - count;
    }

    /// Reads `count` bits (at most 32) as a number.
    mutating func read(count: Int64) -> Int64 {
        if count == 0 { return 0; }
        let value = self.peek(count);
        self.consume(count);
        Int64(truncating: value)
    }

    /// Drops the bits left in the current byte.
    mutating func alignToByte() {
        self.consume(self.bitCount % 8);
    }

    // Stored-block fast path: copies up to `max` whole bytes into `output`,
    // first what the bit buffer holds, then the rest straight from `data`
    // with a single append. The reader must be byte-aligned. Returns the
    // bytes copied; fewer than `max` means the input ran out.
    mutating func copyBytes(max: Int64, mutating into output: Array[UInt8]) -> Int64 {
        var copied: Int64 = 0;
        // Buffered real bytes; zero padding past the end doesn't count.
        while copied < max and self.bitCount - self.paddingBytes * 8 >= 8 {
            output.append(UInt8(truncating: self.buffer & 0xFF));
            self.consume(8);
            copied = copied + 1;
        }
        let available = self.data.count - self.position;
        let n = if max - copied < available { max - copied } else { available };
        if n > 0 {
            output.append(contentsOf: self.data(self.position..<(self.position + n)));
            self.position = self.position + n;
            copied = copied + n;
        }
        copied
    }
}

/// Writes bits least-significant first into a growing byte array.
struct BitWriter {
    var bytes: Array[UInt8];
    var buffer: UInt64;
    var bitCount: Int64;

    init() {
        self.bytes = [];
        self.buffer = 0;
        self.bitCount = 0;
    }

    /// Appends the low `count` bits of `value` (at most 32).
    mutating func write(value: UInt64, count: Int64) {
        self.buffer = self.buffer | (value << self.bitCount);
        self.bitCount = self.bitCount + count;
        while self.bitCount >= 8 {
            self.bytes.append(UInt8(truncating: self.buffer & 0xFF));
            self.buffer = self.buffer >> 8;
            self.bitCount = self.bitCount - 8;
        }
    }

    /// Writes a Huffman code. DEFLATE stores codes most-significant bit
    /// first, so the code is reversed into the LSB-first stream.
    mutating func writeCode(code: UInt64, length: Int64) {
        self.write(reverseBits(code, length), length);
    }

    /// Removes and returns the finished bytes written so far.
    mutating func takeBytes() -> Array[UInt8] {
        let taken = self.bytes;
        self.bytes = [];
        taken
    }

    /// Pads the current byte with zero bits.
    mutating func alignToByte() {
        if self.bitCount > 0 {
            self.write(0, 8 - self.bitCount);
        }
    }
}

/// Reverses the low `length` bits of `code`.
func reverseBits(code: UInt64, length: Int64) -> UInt64 {
    var result: UInt64 = 0;
    var value = code;
    for _ in 0..<length {
        result = (result << 1) | (value & 1);
        value = value >> 1;
    }
    result
}
