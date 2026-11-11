module deflate

// What a zlib or gzip decoder expects next.
let wrapperHeader: Int64 = 0
let wrapperBody: Int64 = 1
let wrapperTrailer: Int64 = 2
let wrapperDone: Int64 = 3

/// Reads a little-endian UInt32 from `bytes` at `offset`.
func readUInt32LittleEndian(bytes: Array[UInt8], offset: Int64) -> UInt32 {
    UInt32(from: bytes(offset)) | (UInt32(from: bytes(offset + 1)) << 8) | (UInt32(from: bytes(offset + 2)) << 16) | (UInt32(from: bytes(offset + 3)) << 24)
}

/// Reads a big-endian UInt32 from `bytes` at `offset`.
func readUInt32BigEndian(bytes: Array[UInt8], offset: Int64) -> UInt32 {
    (UInt32(from: bytes(offset)) << 24) | (UInt32(from: bytes(offset + 1)) << 16) | (UInt32(from: bytes(offset + 2)) << 8) | UInt32(from: bytes(offset + 3))
}

func appendUInt32LittleEndian(mutating bytes: Array[UInt8], value: UInt32) {
    bytes.append(UInt8(truncating: value & 0xFF));
    bytes.append(UInt8(truncating: (value >> 8) & 0xFF));
    bytes.append(UInt8(truncating: (value >> 16) & 0xFF));
    bytes.append(UInt8(truncating: (value >> 24) & 0xFF));
}

func appendUInt32BigEndian(mutating bytes: Array[UInt8], value: UInt32) {
    bytes.append(UInt8(truncating: (value >> 24) & 0xFF));
    bytes.append(UInt8(truncating: (value >> 16) & 0xFF));
    bytes.append(UInt8(truncating: (value >> 8) & 0xFF));
    bytes.append(UInt8(truncating: value & 0xFF));
}

// ============================================================================
// ZLIB (RFC 1950)
// ============================================================================

/// Decompresses a zlib stream (RFC 1950): a 2-byte header, DEFLATE data
/// and a big-endian Adler-32 of the output.
public struct ZlibDecoder: Decompressor {
    var state: Int64;
    var pending: Array[UInt8];     // header or trailer bytes not yet complete
    var inflater: Inflater;
    var checksum: Adler32;
    var failure: DeflateError?;

    public init() {
        self.state = wrapperHeader;
        self.pending = [];
        self.inflater = Inflater();
        self.checksum = Adler32();
        self.failure = .None;
    }

    public var isFinished: Bool { self.state == wrapperDone }

    public mutating func write(input: &Slice[UInt8]) -> Result[Array[UInt8], DeflateError] {
        if let .Some(error) = self.failure { return .Err(error); }
        match self.step(input.asUnsafeSlice()) {
            .Ok(output) => .Ok(output),
            .Err(error) => {
                self.failure = .Some(error);
                .Err(error)
            }
        }
    }

    public mutating func finish() -> Result[(), DeflateError] {
        if let .Some(error) = self.failure { return .Err(error); }
        if self.state != wrapperDone { return .Err(.UnexpectedEnd); }
        .Ok(())
    }

    mutating func step(input: UnsafeSlice[UInt8]) -> Result[Array[UInt8], DeflateError] {
        var output: [UInt8] = [];
        var body: [UInt8] = [];

        if self.state == wrapperHeader {
            self.pending.append(contentsOf: input);
            if self.pending.count < 2 { return .Ok(output); }
            let cmf = Int64(from: self.pending(0));
            let flags = Int64(from: self.pending(1));
            if (cmf & 0x0F) != 8 or (cmf >> 4) > 7 or ((cmf << 8) | flags) % 31 != 0 {
                return .Err(.InvalidHeader);
            }
            if (flags & 0x20) != 0 { return .Err(.UnsupportedDictionary); }
            body = copyTail(self.pending, from: 2);
            self.pending = [];
            self.state = wrapperBody;
        } else if self.state == wrapperBody {
            body.append(contentsOf: input);
        } else {
            self.pending.append(contentsOf: input);
        }

        if self.state == wrapperBody {
            output = try self.inflater.write(body);
            self.checksum.update(output);
            if self.inflater.isFinished {
                self.pending = self.inflater.remainingInput;
                self.state = wrapperTrailer;
            }
        }

        if self.state == wrapperTrailer and self.pending.count >= 4 {
            if self.pending.count > 4 { return .Err(.TrailingData); }
            if readUInt32BigEndian(self.pending, 0) != self.checksum.value { return .Err(.ChecksumMismatch); }
            self.state = wrapperDone;
        } else if self.state == wrapperDone and self.pending.count > 4 {
            return .Err(.TrailingData);
        }
        .Ok(output)
    }
}

/// Compresses into a zlib stream (RFC 1950).
public struct ZlibEncoder: Compressor {
    var deflater: Deflater;
    var checksum: Adler32;
    var wroteHeader: Bool;
    var level: CompressionLevel;

    public init(level level: CompressionLevel = CompressionLevel.standard) {
        self.deflater = Deflater(level: level);
        self.checksum = Adler32();
        self.wroteHeader = false;
        self.level = level;
    }

    public mutating func write(input: &Slice[UInt8]) -> Array[UInt8] {
        let data = input.asUnsafeSlice();
        self.checksum.update(data);
        var output = self.header();
        output.append(contentsOf: self.deflater.write(data));
        output
    }

    public mutating func finish() -> Array[UInt8] {
        var output = self.header();
        output.append(contentsOf: self.deflater.finish());
        appendUInt32BigEndian(output, self.checksum.value);
        output
    }

    // The header on first use, empty after. The level hint follows zlib.
    mutating func header() -> Array[UInt8] {
        if self.wroteHeader { return []; }
        self.wroteHeader = true;
        let value = self.level.value;
        if value <= 1 { return [0x78, 0x01]; }
        if value <= 5 { return [0x78, 0x5E]; }
        if value <= 6 { return [0x78, 0x9C]; }
        [0x78, 0xDA]
    }
}

// ============================================================================
// GZIP (RFC 1952)
// ============================================================================

/// Decompresses gzip data (RFC 1952), including several members
/// concatenated together, as `gzip` itself writes when appending.
public struct GzipDecoder: Decompressor {
    var state: Int64;
    var pending: Array[UInt8];
    var inflater: Inflater;
    var checksum: Crc32;
    var memberLength: Int64;
    var failure: DeflateError?;

    public init() {
        self.state = wrapperHeader;
        self.pending = [];
        self.inflater = Inflater();
        self.checksum = Crc32();
        self.memberLength = 0;
        self.failure = .None;
    }

    /// True after a complete member, when no further member has started.
    public var isFinished: Bool { self.state == wrapperDone }

    public mutating func write(input: &Slice[UInt8]) -> Result[Array[UInt8], DeflateError] {
        if let .Some(error) = self.failure { return .Err(error); }
        self.pending.append(contentsOf: input.asUnsafeSlice());
        match self.step() {
            .Ok(output) => .Ok(output),
            .Err(error) => {
                self.failure = .Some(error);
                .Err(error)
            }
        }
    }

    public mutating func finish() -> Result[(), DeflateError] {
        if let .Some(error) = self.failure { return .Err(error); }
        if self.state != wrapperDone or self.pending.count > 0 { return .Err(.UnexpectedEnd); }
        .Ok(())
    }

    // Works through `pending` as far as it goes, possibly across members.
    mutating func step() -> Result[Array[UInt8], DeflateError] {
        var output: [UInt8] = [];
        loop {
            if self.state == wrapperHeader or self.state == wrapperDone {
                if self.pending.count == 0 { return .Ok(output); }
                guard let .Some(headerLength) = try parseGzipHeader(self.pending) else { return .Ok(output); }
                let body = copyTail(self.pending, from: headerLength);
                self.pending = [];
                self.inflater = Inflater();
                self.checksum = Crc32();
                self.memberLength = 0;
                self.state = wrapperBody;
                output.append(contentsOf: try self.inflateMore(body));
            } else if self.state == wrapperBody {
                let body = self.pending;
                self.pending = [];
                output.append(contentsOf: try self.inflateMore(body));
                if self.state == wrapperBody { return .Ok(output); }
            } else {
                // Trailer: CRC-32 then the length mod 2^32, both little-endian.
                if self.pending.count < 8 { return .Ok(output); }
                if readUInt32LittleEndian(self.pending, 0) != self.checksum.value { return .Err(.ChecksumMismatch); }
                let expectedLength = Int64(from: readUInt32LittleEndian(self.pending, 4));
                if expectedLength != (self.memberLength & 0xFFFFFFFF) { return .Err(.LengthMismatch); }
                self.pending = copyTail(self.pending, from: 8);
                self.state = wrapperDone;
            }
        }
    }

    mutating func inflateMore(body: Array[UInt8]) -> Result[Array[UInt8], DeflateError] {
        let output = try self.inflater.write(body);
        self.checksum.update(output);
        self.memberLength = self.memberLength + output.count;
        if self.inflater.isFinished {
            self.pending = self.inflater.remainingInput;
            self.state = wrapperTrailer;
        }
        .Ok(output)
    }
}

/// The length of the gzip member header at the start of `bytes`, or None
/// if more bytes are needed to tell.
func parseGzipHeader(bytes: Array[UInt8]) -> Result[Int64?, DeflateError] {
    if bytes.count >= 1 and bytes(0) != 0x1F { return .Err(.InvalidHeader); }
    if bytes.count >= 2 and bytes(1) != 0x8B { return .Err(.InvalidHeader); }
    if bytes.count >= 3 and bytes(2) != 8 { return .Err(.InvalidHeader); }
    if bytes.count < 10 { return .Ok(.None); }
    let flags = Int64(from: bytes(3));
    if (flags & 0xE0) != 0 { return .Err(.InvalidHeader); }

    var position: Int64 = 10;
    // FEXTRA: a 2-byte length, then that many bytes.
    if (flags & 4) != 0 {
        if bytes.count < position + 2 { return .Ok(.None); }
        position = position + 2 + (Int64(from: bytes(position)) | (Int64(from: bytes(position + 1)) << 8));
    }
    // FNAME and FCOMMENT: zero-terminated strings.
    for flag in [8, 16] {
        if (flags & flag) != 0 {
            loop {
                if position >= bytes.count { return .Ok(.None); }
                position = position + 1;
                if bytes(position - 1) == 0 { break; }
            }
        }
    }
    // FHCRC: a 2-byte header CRC.
    if (flags & 2) != 0 { position = position + 2; }
    if position > bytes.count { return .Ok(.None); }
    .Ok(.Some(position))
}

/// Compresses into a single gzip member (RFC 1952), with no file name and
/// a zero timestamp so output is reproducible.
public struct GzipEncoder: Compressor {
    var deflater: Deflater;
    var checksum: Crc32;
    var length: Int64;
    var wroteHeader: Bool;
    var level: CompressionLevel;

    public init(level level: CompressionLevel = CompressionLevel.standard) {
        self.deflater = Deflater(level: level);
        self.checksum = Crc32();
        self.length = 0;
        self.wroteHeader = false;
        self.level = level;
    }

    public mutating func write(input: &Slice[UInt8]) -> Array[UInt8] {
        let data = input.asUnsafeSlice();
        self.checksum.update(data);
        self.length = self.length + data.count;
        var output = self.header();
        output.append(contentsOf: self.deflater.write(data));
        output
    }

    public mutating func finish() -> Array[UInt8] {
        var output = self.header();
        output.append(contentsOf: self.deflater.finish());
        appendUInt32LittleEndian(output, self.checksum.value);
        appendUInt32LittleEndian(output, UInt32(from: self.length & 0xFFFFFFFF));
        output
    }

    mutating func header() -> Array[UInt8] {
        if self.wroteHeader { return []; }
        self.wroteHeader = true;
        // XFL: 2 = best compression, 4 = fastest. OS 255 = unknown.
        var extraFlags: UInt8 = 0;
        if self.level.value >= 9 { extraFlags = 2; }
        if self.level.value == 1 { extraFlags = 4; }
        [0x1F, 0x8B, 8, 0, 0, 0, 0, 0, extraFlags, 255]
    }
}

// ============================================================================
// ONE-SHOT HELPERS
// ============================================================================

func decompressAll[D](decoder: D, data: UnsafeSlice[UInt8]) -> Result[Array[UInt8], DeflateError] where D: Decompressor {
    var d = decoder;
    let output = try d.write(data);
    try d.finish();
    .Ok(output)
}

func compressAll[C](encoder: C, data: UnsafeSlice[UInt8]) -> Array[UInt8] where C: Compressor {
    var e = encoder;
    var output = e.write(data);
    output.append(contentsOf: e.finish());
    output
}

/// Raw DEFLATE (RFC 1951) in one call.
///
/// # Examples
///
/// ```
/// let packed = Deflate.compress(data, level: .best);
/// let unpacked = try Deflate.decompress(packed);
/// ```
public struct Deflate {
    public static func compress(data: &Slice[UInt8], level level: CompressionLevel = CompressionLevel.standard) -> Array[UInt8] {
        compressAll(Deflater(level: level), data.asUnsafeSlice())
    }

    /// Fails on malformed or truncated input. Bytes after the end of the
    /// stream are ignored.
    public static func decompress(data: &Slice[UInt8]) -> Result[Array[UInt8], DeflateError] {
        decompressAll(Inflater(), data.asUnsafeSlice())
    }
}

/// zlib (RFC 1950) in one call.
public struct Zlib {
    public static func compress(data: &Slice[UInt8], level level: CompressionLevel = CompressionLevel.standard) -> Array[UInt8] {
        compressAll(ZlibEncoder(level: level), data.asUnsafeSlice())
    }

    /// Fails on malformed or truncated input, a bad checksum, or trailing bytes.
    public static func decompress(data: &Slice[UInt8]) -> Result[Array[UInt8], DeflateError] {
        decompressAll(ZlibDecoder(), data.asUnsafeSlice())
    }
}

/// gzip (RFC 1952) in one call.
public struct Gzip {
    public static func compress(data: &Slice[UInt8], level level: CompressionLevel = CompressionLevel.standard) -> Array[UInt8] {
        compressAll(GzipEncoder(level: level), data.asUnsafeSlice())
    }

    /// Decompresses every member. Fails on malformed or truncated input, a
    /// bad checksum or length, or trailing bytes that aren't a member.
    public static func decompress(data: &Slice[UInt8]) -> Result[Array[UInt8], DeflateError] {
        decompressAll(GzipDecoder(), data.asUnsafeSlice())
    }
}
