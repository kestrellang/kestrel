module images

// Byte helpers shared by the codecs.

/// Input fed to a streaming decoder but not yet used.
struct ByteQueue {
    var data: Array[UInt8];
    var position: Int64;

    init() {
        self.data = [];
        self.position = 0;
    }

    /// Adds input, first dropping bytes already used.
    mutating func append(input: Array[UInt8]) {
        if self.position > 0 and self.position * 2 >= self.data.count {
            var rest: [UInt8] = [];
            for i in self.position..<self.data.count { rest.append(self.data(i)); }
            self.data = rest;
            self.position = 0;
        }
        self.data.append(contentsOf: input);
    }

    var available: Int64 { self.data.count - self.position }

    /// The byte `offset` bytes past the read position.
    func peek(offset: Int64) -> UInt8 { self.data(self.position + offset) }

    mutating func take() -> UInt8 {
        let b = self.data(self.position);
        self.position = self.position + 1;
        b
    }

    mutating func skip(count: Int64) { self.position = self.position + count; }

    /// The next `count` bytes as an array.
    mutating func takeBytes(count: Int64) -> Array[UInt8] {
        var out: [UInt8] = [];
        for i in 0..<count { out.append(self.data(self.position + i)); }
        self.position = self.position + count;
        out
    }

    func peekUInt32BE(offset: Int64) -> Int64 {
        (Int64(from: self.peek(offset)) << 24) | (Int64(from: self.peek(offset + 1)) << 16) | (Int64(from: self.peek(offset + 2)) << 8) | Int64(from: self.peek(offset + 3))
    }

    func peekUInt32LE(offset: Int64) -> Int64 {
        Int64(from: self.peek(offset)) | (Int64(from: self.peek(offset + 1)) << 8) | (Int64(from: self.peek(offset + 2)) << 16) | (Int64(from: self.peek(offset + 3)) << 24)
    }

    func peekUInt16LE(offset: Int64) -> Int64 {
        Int64(from: self.peek(offset)) | (Int64(from: self.peek(offset + 1)) << 8)
    }
}

func readUInt32BE(data: Array[UInt8], i: Int64) -> Int64 {
    (Int64(from: data(i)) << 24) | (Int64(from: data(i + 1)) << 16) | (Int64(from: data(i + 2)) << 8) | Int64(from: data(i + 3))
}

func readUInt32LE(data: Array[UInt8], i: Int64) -> Int64 {
    Int64(from: data(i)) | (Int64(from: data(i + 1)) << 8) | (Int64(from: data(i + 2)) << 16) | (Int64(from: data(i + 3)) << 24)
}

func readUInt16LE(data: Array[UInt8], i: Int64) -> Int64 {
    Int64(from: data(i)) | (Int64(from: data(i + 1)) << 8)
}

func appendUInt32BE(mutating out: Array[UInt8], value: Int64) {
    out.append(UInt8(from: value >> 24));
    out.append(UInt8(from: value >> 16));
    out.append(UInt8(from: value >> 8));
    out.append(UInt8(from: value));
}

func appendUInt32LE(mutating out: Array[UInt8], value: Int64) {
    out.append(UInt8(from: value));
    out.append(UInt8(from: value >> 8));
    out.append(UInt8(from: value >> 16));
    out.append(UInt8(from: value >> 24));
}

func appendUInt16LE(mutating out: Array[UInt8], value: Int64) {
    out.append(UInt8(from: value));
    out.append(UInt8(from: value >> 8));
}

func appendAscii(mutating out: Array[UInt8], text: String) {
    for b in text.bytes { out.append(b); }
}

/// True if `data` starts with the ASCII bytes of `prefix`.
func startsWith(data: Array[UInt8], prefix: String) -> Bool {
    var i: Int64 = 0;
    for b in prefix.bytes {
        if i >= data.count or data(i) != b { return false; }
        i = i + 1;
    }
    true
}
