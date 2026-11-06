module images

import deflate.(ZlibDecoder, Decompressor, Zlib, Crc32, CompressionLevel)

// PNG (ISO/IEC 15948, third edition). Decodes every color type, bit depth
// and Adam7 interlacing; encodes 8- and 16-bit images. APNG animation
// chunks are skipped, so animated files give their default image.

/// The PNG format. `level` sets the zlib compression level for encoding.
public struct PngFormat: ImageFormat {
    public let level: CompressionLevel;

    public init() { self.level = .standard; }
    public init(level level: CompressionLevel) { self.level = level; }

    public var name: String { "png" }
    public var extensions: Array[String] { ["png"] }

    public func matches(header: Array[UInt8]) -> Bool {
        header.count >= 8 and header(0) == 137 and header(1) == 80 and header(2) == 78 and header(3) == 71
    }

    public func makeDecoder(options: DecodeOptions) -> any ImageDecoder { PngDecoder(options: options) }

    public func canEncode(layout: PixelLayout) -> Bool {
        pngColorType(layout) >= 0
    }

    public func preferredLayout(for layout: PixelLayout) -> PixelLayout {
        let alpha = layout.channels.hasAlpha;
        let gray = layout.channels == .gray or layout.channels == .grayAlpha;
        if layout.sample.bytes >= 2 {
            if gray and alpha { return .grayAlpha16; }
            if gray { return .gray16; }
            if alpha { return .rgba16; }
            return .rgb16;
        }
        if gray and alpha { return .grayAlpha8; }
        if gray { return .gray8; }
        if alpha { return .rgba8; }
        .rgb8
    }

    public func encode(image: RawImage, options: EncodeOptions) -> Result[Array[UInt8], ImageError] {
        if not image.isInterleaved or not self.canEncode(image.layout) {
            return .Err(ImageError(kind: .layoutMismatch, format: "png", message: "PNG can't write \(image.layout.description) directly"));
        }
        .Ok(encodePng(image, self.level, options.keepMetadata))
    }
}

/// The PNG color type for a layout, or -1.
func pngColorType(layout: PixelLayout) -> Int64 {
    if layout.sample != .uint8 and layout.sample != .uint16 { return -1; }
    let name = layout.channels.name;
    if name == "gray" { return 0; }
    if name == "rgb" { return 2; }
    if name == "grayAlpha" { return 4; }
    if name == "rgba" { return 6; }
    return -1;
}

// Adam7 pass geometry.
let adam7XStart: [Int64] = [0, 4, 0, 2, 0, 1, 0];
let adam7YStart: [Int64] = [0, 0, 4, 0, 2, 0, 1];
let adam7XStep: [Int64] = [8, 8, 4, 4, 2, 2, 1];
let adam7YStep: [Int64] = [8, 8, 8, 4, 4, 2, 2];

func passSize(full: Int64, start: Int64, step: Int64) -> Int64 {
    if full <= start { return 0; }
    (full - start + step - 1) / step
}

// Decoder stages.
let pngSignature: Int64 = 0;
let pngChunkHeader: Int64 = 1;
let pngChunkBody: Int64 = 2;
let pngImageData: Int64 = 3;
let pngSkipping: Int64 = 4;
let pngDone: Int64 = 5;

struct PngDecoder: ImageDecoder {
    let options: DecodeOptions;
    var queue: ByteQueue;
    var stage: Int64;
    var failure: ImageError?;

    // The chunk being read.
    var chunkLength: Int64;
    var chunkType: String;
    var chunkRemaining: Int64;
    var crc: Crc32;

    // IHDR.
    var width: Int64;
    var height: Int64;
    var bitDepth: Int64;
    var colorType: Int64;
    var interlaced: Bool;
    var channels: Int64;

    // Color and transparency.
    var palette: Array[UInt8];
    var paletteAlpha: Array[UInt8];
    /// tRNS for gray and RGB: the raw sample values that mean transparent.
    var transparent: Array[Int64];
    var color: ColorInfo;
    var orientation: Orientation;
    var metadata: ImageMetadata;
    var metadataBytes: Int64;

    // Pixel data.
    var began: Bool;
    var deliver: Bool;
    var finishedPixels: Bool;
    var zlib: ZlibDecoder;
    var scan: ByteQueue;
    var previous: Array[UInt8];
    var outLayout: PixelLayout;
    var pass: Int64;
    var passRow: Int64;
    /// The whole image, for interlaced files: rows go out after the last pass.
    var canvas: Array[UInt8];

    init(options options: DecodeOptions) {
        self.options = options;
        self.queue = ByteQueue();
        self.stage = pngSignature;
        self.failure = .None;
        self.chunkLength = 0;
        self.chunkType = "";
        self.chunkRemaining = 0;
        self.crc = Crc32();
        self.width = 0;
        self.height = 0;
        self.bitDepth = 8;
        self.colorType = 0;
        self.interlaced = false;
        self.channels = 1;
        self.palette = [];
        self.paletteAlpha = [];
        self.transparent = [];
        self.color = .srgb;
        self.orientation = .normal;
        self.metadata = ImageMetadata();
        self.metadataBytes = 0;
        self.began = false;
        self.deliver = true;
        self.finishedPixels = false;
        self.zlib = ZlibDecoder();
        self.scan = ByteQueue();
        self.previous = [];
        self.outLayout = .rgba8;
        self.pass = 0;
        self.passRow = 0;
        self.canvas = [];
    }

    mutating func write(input: Array[UInt8], mutating into sink: any ImageSink) -> Result[DecodeStatus, ImageError] {
        if let .Some(error) = self.failure { return .Err(error); }
        match self.step(input, into: sink) {
            .Ok(status) => .Ok(status),
            .Err(error) => {
                self.failure = .Some(error);
                .Err(error)
            }
        }
    }

    mutating func finish(mutating into sink: any ImageSink) -> Result[(), ImageError] {
        if let .Some(error) = self.failure { return .Err(error); }
        if self.stage != pngDone { return .Err(truncated("png")); }
        .Ok(())
    }

    mutating func step(input: Array[UInt8], mutating into sink: any ImageSink) -> Result[DecodeStatus, ImageError] {
        if self.stage == pngDone { return .Ok(.done); }
        self.queue.append(input);
        loop {
            if self.stage == pngSignature {
                if self.queue.available < 8 { return .Ok(.needsInput); }
                let expected: [UInt8] = [137, 80, 78, 71, 13, 10, 26, 10];
                for i in 0..<8 {
                    if self.queue.peek(i) != expected(i) { return .Err(corrupt("png", "missing PNG signature")); }
                }
                self.queue.skip(8);
                self.stage = pngChunkHeader;
            } else if self.stage == pngChunkHeader {
                if self.queue.available < 8 { return .Ok(.needsInput); }
                try self.startChunk(into: sink);
            } else if self.stage == pngChunkBody {
                if self.queue.available < self.chunkLength + 4 { return .Ok(.needsInput); }
                let body = self.queue.takeBytes(self.chunkLength);
                self.crc.update(body);
                try self.checkCrc();
                try self.handleChunk(body, into: sink);
                if self.stage == pngDone { return .Ok(.done); }
                if self.stage == pngChunkBody { self.stage = pngChunkHeader; }
            } else if self.stage == pngImageData {
                if self.chunkRemaining > 0 {
                    if self.queue.available == 0 { return .Ok(.needsInput); }
                    var n = self.queue.available;
                    if n > self.chunkRemaining { n = self.chunkRemaining; }
                    let data = self.queue.takeBytes(n);
                    self.chunkRemaining = self.chunkRemaining - n;
                    self.crc.update(data);
                    try self.inflate(data, into: sink);
                } else {
                    if self.queue.available < 4 { return .Ok(.needsInput); }
                    try self.checkCrc();
                    self.stage = pngChunkHeader;
                }
            } else if self.stage == pngSkipping {
                if self.chunkRemaining > 0 {
                    if self.queue.available == 0 { return .Ok(.needsInput); }
                    var n = self.queue.available;
                    if n > self.chunkRemaining { n = self.chunkRemaining; }
                    self.queue.skip(n);
                    self.chunkRemaining = self.chunkRemaining - n;
                } else {
                    if self.queue.available < 4 { return .Ok(.needsInput); }
                    self.queue.skip(4);
                    self.stage = pngChunkHeader;
                }
            } else {
                return .Ok(.done);
            }
        }
    }

    /// Reads a chunk's length and type and picks how to read its body.
    mutating func startChunk(mutating into sink: any ImageSink) -> Result[(), ImageError] {
        self.chunkLength = self.queue.peekUInt32BE(0);
        let typeBytes = [self.queue.peek(4), self.queue.peek(5), self.queue.peek(6), self.queue.peek(7)];
        self.queue.skip(8);
        if self.chunkLength > 0x7FFFFFFF { return .Err(corrupt("png", "chunk length too large")); }
        for b in typeBytes {
            let lower = b | 0x20;
            if lower < 97 or lower > 122 { return .Err(corrupt("png", "bad chunk type")); }
        }
        self.chunkType = String(fromUtf8Lossy: typeBytes);
        self.crc = Crc32();
        self.crc.update(typeBytes);
        let isFirstChunk = self.width == 0;
        if isFirstChunk and self.chunkType != "IHDR" { return .Err(corrupt("png", "first chunk must be IHDR")); }

        if self.chunkType == "IDAT" {
            if self.zlib.isFinished { return .Err(corrupt("png", "IDAT after the image data ended")); }
            if not self.began { try self.beginImage(into: sink); }
            self.chunkRemaining = self.chunkLength;
            self.stage = pngImageData;
            return .Ok(());
        }
        if self.began and not self.zlib.isFinished and self.chunkType != "IEND" {
            return .Err(corrupt("png", "\(self.chunkType) inside the image data"));
        }
        if self.began and not self.zlib.isFinished { return .Err(truncated("png")); }
        let critical = typeBytes(0) < 97;
        let known = self.chunkType == "IHDR" or self.chunkType == "PLTE" or self.chunkType == "IEND";
        if not critical and self.chunkLength > self.options.maxMetadataBytes - self.metadataBytes {
            // Too big to keep: skip it unread.
            self.chunkRemaining = self.chunkLength;
            self.stage = pngSkipping;
            return .Ok(());
        }
        if critical and not known {
            return .Err(unsupported("png", "unknown critical chunk \(self.chunkType)"));
        }
        self.stage = pngChunkBody;
        .Ok(())
    }

    mutating func checkCrc() -> Result[(), ImageError] {
        let stored = self.queue.peekUInt32BE(0);
        self.queue.skip(4);
        if stored != Int64(from: self.crc.value) { return .Err(corrupt("png", "bad CRC in \(self.chunkType)")); }
        .Ok(())
    }

    mutating func handleChunk(body: Array[UInt8], mutating into sink: any ImageSink) -> Result[(), ImageError] {
        let t = self.chunkType;
        if t == "IHDR" { return self.readHeader(body); }
        if t == "PLTE" {
            if body.count % 3 != 0 or body.count == 0 or body.count > 768 { return .Err(corrupt("png", "bad PLTE length")); }
            self.palette = body;
            return .Ok(());
        }
        if t == "IEND" {
            if not self.began or not self.finishedPixels { return .Err(truncated("png")); }
            if self.deliver { sink.endImage(0, self.metadata); }
            self.stage = pngDone;
            return .Ok(());
        }
        if t == "tRNS" { return self.readTransparency(body); }
        if t == "cICP" {
            if body.count != 4 { return .Err(corrupt("png", "bad cICP length")); }
            self.color = ColorInfo(primaries: Int64(from: body(0)), transfer: Int64(from: body(1)), matrix: Int64(from: body(2)), fullRange: body(3) != 0);
            return .Ok(());
        }
        if t == "gAMA" {
            if body.count == 4 { self.metadata.gamma = .Some(Float64(from: readUInt32BE(body, 0)) / 100000.0); }
            return .Ok(());
        }
        if t == "pHYs" {
            if body.count == 9 and body(8) == 1 {
                self.metadata.dotsPerMeter = .Some((readUInt32BE(body, 0), readUInt32BE(body, 4)));
            }
            return .Ok(());
        }
        if t == "sRGB" or t == "cHRM" or t == "sBIT" or t == "bKGD" or t == "hIST" or t == "sPLT" or t == "tIME" {
            // Recorded as unknown so they survive a re-encode.
            self.keepUnknown(body);
            return .Ok(());
        }
        if t == "acTL" or t == "fcTL" or t == "fdAT" { return .Ok(()); }
        if t == "iCCP" {
            let name = indexOfZero(body, 0);
            if name < 0 or name + 2 > body.count { return .Err(corrupt("png", "bad iCCP chunk")); }
            match Zlib.decompress(tailBytes(body, name + 2)) {
                .Ok(profile) => { self.metadata.iccProfile = .Some(profile); },
                .Err(_) => { return .Err(corrupt("png", "bad iCCP data")); }
            }
            self.metadataBytes = self.metadataBytes + body.count;
            return .Ok(());
        }
        if t == "eXIf" {
            self.metadata.exif = .Some(body);
            self.orientation = exifOrientation(body);
            self.metadataBytes = self.metadataBytes + body.count;
            return .Ok(());
        }
        if not self.options.keepMetadata { return .Ok(()); }
        if t == "tEXt" or t == "zTXt" or t == "iTXt" {
            try self.readText(t, body);
            return .Ok(());
        }
        self.keepUnknown(body);
        .Ok(())
    }

    mutating func keepUnknown(body: Array[UInt8]) {
        if not self.options.keepMetadata { return; }
        self.metadata.unknownChunks.append((self.chunkType, body));
        self.metadataBytes = self.metadataBytes + body.count;
    }

    mutating func readHeader(body: Array[UInt8]) -> Result[(), ImageError] {
        if body.count != 13 { return .Err(corrupt("png", "bad IHDR length")); }
        self.width = readUInt32BE(body, 0);
        self.height = readUInt32BE(body, 4);
        self.bitDepth = Int64(from: body(8));
        self.colorType = Int64(from: body(9));
        if body(10) != 0 or body(11) != 0 { return .Err(corrupt("png", "unknown compression or filter method")); }
        if body(12) > 1 { return .Err(corrupt("png", "unknown interlace method")); }
        self.interlaced = body(12) == 1;
        let d = self.bitDepth;
        let ct = self.colorType;
        var ok = false;
        if ct == 0 { ok = d == 1 or d == 2 or d == 4 or d == 8 or d == 16; self.channels = 1; }
        if ct == 2 { ok = d == 8 or d == 16; self.channels = 3; }
        if ct == 3 { ok = d == 1 or d == 2 or d == 4 or d == 8; self.channels = 1; }
        if ct == 4 { ok = d == 8 or d == 16; self.channels = 2; }
        if ct == 6 { ok = d == 8 or d == 16; self.channels = 4; }
        if not ok { return .Err(corrupt("png", "bad color type \(ct) with bit depth \(d)")); }
        if self.width == 0 or self.height == 0 { return .Err(corrupt("png", "zero width or height")); }
        .Ok(())
    }

    mutating func readTransparency(body: Array[UInt8]) -> Result[(), ImageError] {
        if self.colorType == 3 {
            if body.count > self.palette.count / 3 { return .Err(corrupt("png", "tRNS longer than the palette")); }
            self.paletteAlpha = body;
            return .Ok(());
        }
        if self.colorType == 0 {
            if body.count != 2 { return .Err(corrupt("png", "bad tRNS length")); }
            self.transparent = [(Int64(from: body(0)) << 8) | Int64(from: body(1))];
            return .Ok(());
        }
        if self.colorType == 2 {
            if body.count != 6 { return .Err(corrupt("png", "bad tRNS length")); }
            self.transparent = [
                (Int64(from: body(0)) << 8) | Int64(from: body(1)),
                (Int64(from: body(2)) << 8) | Int64(from: body(3)),
                (Int64(from: body(4)) << 8) | Int64(from: body(5))
            ];
            return .Ok(());
        }
        .Err(corrupt("png", "tRNS not allowed with an alpha channel"))
    }

    mutating func readText(kind: String, body: Array[UInt8]) -> Result[(), ImageError] {
        let keyEnd = indexOfZero(body, 0);
        if keyEnd < 1 { return .Err(corrupt("png", "bad \(kind) keyword")); }
        let key = latin1ToString(body, 0, keyEnd);
        var value = "";
        if kind == "tEXt" {
            value = latin1ToString(body, keyEnd + 1, body.count);
        } else if kind == "zTXt" {
            if keyEnd + 2 > body.count { return .Err(corrupt("png", "bad zTXt chunk")); }
            match Zlib.decompress(tailBytes(body, keyEnd + 2)) {
                .Ok(text) => { value = latin1ToString(text, 0, text.count); },
                .Err(_) => { return .Err(corrupt("png", "bad zTXt data")); }
            }
        } else {
            // iTXt: flag, method, language\0, translated keyword\0, text.
            if keyEnd + 3 > body.count { return .Err(corrupt("png", "bad iTXt chunk")); }
            let compressed = body(keyEnd + 1) == 1;
            let langEnd = indexOfZero(body, keyEnd + 3);
            if langEnd < 0 { return .Err(corrupt("png", "bad iTXt chunk")); }
            let transEnd = indexOfZero(body, langEnd + 1);
            if transEnd < 0 { return .Err(corrupt("png", "bad iTXt chunk")); }
            var text = tailBytes(body, transEnd + 1);
            if compressed {
                match Zlib.decompress(text) {
                    .Ok(plain) => { text = plain; },
                    .Err(_) => { return .Err(corrupt("png", "bad iTXt data")); }
                }
            }
            value = String(fromUtf8Lossy: text);
        }
        if key == "XML:com.adobe.xmp" {
            self.metadata.xmp = .Some(value);
        } else {
            self.metadata.text.append((key, value));
        }
        self.metadataBytes = self.metadataBytes + body.count;
        .Ok(())
    }

    /// Announces the image to the sink at the first IDAT, once every
    /// chunk that affects the pixels has been read.
    mutating func beginImage(mutating into sink: any ImageSink) -> Result[(), ImageError] {
        try self.options.check("png", self.width, self.height);
        if self.colorType == 3 and self.palette.count == 0 { return .Err(corrupt("png", "missing PLTE")); }
        let wide = self.bitDepth == 16;
        let hasTrns = self.transparent.count > 0 or self.paletteAlpha.count > 0;
        let ct = self.colorType;
        if ct == 0 {
            if hasTrns {
                self.outLayout = .grayAlpha8;
                if wide { self.outLayout = .grayAlpha16; }
            } else {
                self.outLayout = .gray8;
                if wide { self.outLayout = .gray16; }
            }
        } else if ct == 2 {
            if hasTrns {
                self.outLayout = .rgba8;
                if wide { self.outLayout = .rgba16; }
            } else {
                self.outLayout = .rgb8;
                if wide { self.outLayout = .rgb16; }
            }
        } else if ct == 3 {
            self.outLayout = .rgb8;
            if hasTrns { self.outLayout = .rgba8; }
        } else if ct == 4 {
            self.outLayout = .grayAlpha8;
            if wide { self.outLayout = .grayAlpha16; }
        } else {
            self.outLayout = .rgba8;
            if wide { self.outLayout = .rgba16; }
        }
        var alpha = AlphaMode.none;
        if self.outLayout.channels.hasAlpha { alpha = .straight; }
        let info = ImageInfo(
            width: self.width, height: self.height,
            planes: [PlaneInfo(width: self.width, height: self.height, layout: self.outLayout)],
            color: self.color, alpha: alpha, orientation: self.orientation, frame: .None,
            metadata: self.metadata);
        self.deliver = sink.beginImage(0, info) == .decode;
        self.began = true;
        if self.interlaced {
            self.canvas = Array[UInt8](repeating: 0, count: self.width * self.height * self.outLayout.bytesPerPixel);
            self.pass = 0;
            self.skipEmptyPasses();
        }
        self.passRow = 0;
        self.previous = Array[UInt8](repeating: 0, count: self.rowBytes(self.passWidth()));
        .Ok(())
    }

    /// Bytes in one filtered row of `pixels` pixels, without the filter byte.
    func rowBytes(pixels: Int64) -> Int64 { (pixels * self.channels * self.bitDepth + 7) / 8 }

    func passWidth() -> Int64 {
        if not self.interlaced { return self.width; }
        if self.pass >= 7 { return 0; }
        passSize(self.width, adam7XStart(self.pass), adam7XStep(self.pass))
    }

    func passHeight() -> Int64 {
        if not self.interlaced { return self.height; }
        if self.pass >= 7 { return 0; }
        passSize(self.height, adam7YStart(self.pass), adam7YStep(self.pass))
    }

    mutating func skipEmptyPasses() {
        while self.pass < 7 and (self.passWidth() == 0 or self.passHeight() == 0) {
            self.pass = self.pass + 1;
        }
    }

    mutating func inflate(data: Array[UInt8], mutating into sink: any ImageSink) -> Result[(), ImageError] {
        match self.zlib.write(data) {
            .Ok(output) => self.scan.append(output),
            .Err(_) => { return .Err(corrupt("png", "bad image data")); }
        }
        self.unfilterRows(into: sink);
        if self.zlib.isFinished {
            if not self.finishedPixels { return .Err(truncated("png")); }
        }
        .Ok(())
    }

    /// Unfilters and delivers every complete row in `scan`.
    mutating func unfilterRows(mutating into sink: any ImageSink) {
        while not self.finishedPixels {
            let pixels = self.passWidth();
            let length = self.rowBytes(pixels);
            if self.scan.available < length + 1 { return; }
            let filter = self.scan.take();
            var row = self.scan.takeBytes(length);
            unfilter(row, self.previous, Int64(from: filter), self.filterStride());
            self.emitRow(row, pixels, into: sink);
            self.previous = row;
            self.passRow = self.passRow + 1;
            if self.passRow == self.passHeight() {
                if not self.interlaced {
                    self.finishedPixels = true;
                    return;
                }
                self.pass = self.pass + 1;
                self.skipEmptyPasses();
                self.passRow = 0;
                if self.pass >= 7 {
                    self.finishedPixels = true;
                    self.deliverCanvas(into: sink);
                    return;
                }
                self.previous = Array[UInt8](repeating: 0, count: self.rowBytes(self.passWidth()));
            }
        }
    }

    /// Bytes between corresponding samples of neighboring pixels.
    func filterStride() -> Int64 {
        let bits = self.channels * self.bitDepth;
        if bits < 8 { return 1; }
        bits / 8
    }

    mutating func emitRow(row: Array[UInt8], pixels: Int64, mutating into sink: any ImageSink) {
        if not self.deliver { return; }
        let out = self.expandRow(row, pixels);
        if not self.interlaced {
            sink.receive(0, Rect(x: 0, y: self.passRow, width: self.width, height: 1), out, out.count);
            return;
        }
        let bpp = self.outLayout.bytesPerPixel;
        let y = adam7YStart(self.pass) + self.passRow * adam7YStep(self.pass);
        let x0 = adam7XStart(self.pass);
        let step = adam7XStep(self.pass);
        self.canvas.ensureUnique();
        for i in 0..<pixels {
            let target = (y * self.width + x0 + i * step) * bpp;
            for k in 0..<bpp { self.canvas(target + k) = out(i * bpp + k); }
        }
    }

    mutating func deliverCanvas(mutating into sink: any ImageSink) {
        if not self.deliver { return; }
        let stride = self.width * self.outLayout.bytesPerPixel;
        sink.receive(0, Rect(x: 0, y: 0, width: self.width, height: self.height), self.canvas, stride);
        self.canvas = [];
    }

    /// One unfiltered row in PNG's packing to the output layout:
    /// sub-byte samples scaled to 8 bits, palettes expanded, tRNS turned
    /// into alpha, and 16-bit samples swapped to native order.
    func expandRow(row: Array[UInt8], pixels: Int64) -> Array[UInt8] {
        let depth = self.bitDepth;
        let outChannels = self.outLayout.channels.count;
        var out = Array[UInt8](repeating: 0, count: pixels * self.outLayout.bytesPerPixel);
        if self.colorType == 3 {
            let withAlpha = outChannels == 4;
            let entries = self.palette.count / 3;
            for i in 0..<pixels {
                let index = unpackSample(row, i, depth);
                let o = i * outChannels;
                if index < entries {
                    out(o) = self.palette(index * 3);
                    out(o + 1) = self.palette(index * 3 + 1);
                    out(o + 2) = self.palette(index * 3 + 2);
                }
                if withAlpha {
                    var a: UInt8 = 255;
                    if index < self.paletteAlpha.count { a = self.paletteAlpha(index); }
                    out(o + 3) = a;
                }
            }
            return out;
        }
        let addAlpha = self.transparent.count > 0;
        if depth < 8 {
            let scale = 255 / ((1 << depth) - 1);
            for i in 0..<pixels {
                let v = unpackSample(row, i, depth);
                if addAlpha {
                    out(i * 2) = UInt8(truncating: v * scale);
                    var a: UInt8 = 255;
                    if v == self.transparent(0) { a = 0; }
                    out(i * 2 + 1) = a;
                } else {
                    out(i) = UInt8(truncating: v * scale);
                }
            }
            return out;
        }
        let sampleBytes = depth / 8;
        let inChannels = self.channels;
        for i in 0..<pixels {
            var isTransparent = addAlpha;
            for c in 0..<inChannels {
                let src = (i * inChannels + c) * sampleBytes;
                let dst = (i * outChannels + c) * sampleBytes;
                if sampleBytes == 1 {
                    out(dst) = row(src);
                    if addAlpha and Int64(from: row(src)) != self.transparent(c) { isTransparent = false; }
                } else {
                    out(dst) = row(src + 1);
                    out(dst + 1) = row(src);
                    let v = (Int64(from: row(src)) << 8) | Int64(from: row(src + 1));
                    if addAlpha and v != self.transparent(c) { isTransparent = false; }
                }
            }
            if addAlpha {
                let dst = (i * outChannels + inChannels) * sampleBytes;
                var a: UInt8 = 255;
                if isTransparent { a = 0; }
                out(dst) = a;
                if sampleBytes == 2 { out(dst + 1) = a; }
            }
        }
        out
    }
}

/// The `index`th sample of `depth` bits (1, 2, 4 or 8), most significant first.
func unpackSample(row: Array[UInt8], index: Int64, depth: Int64) -> Int64 {
    if depth == 8 { return Int64(from: row(index)); }
    let bit = index * depth;
    let byte = Int64(from: row(bit / 8));
    (byte >> (8 - depth - bit % 8)) & ((1 << depth) - 1)
}

/// Reverses one PNG filter in place. `previous` is the unfiltered row
/// above (zeros for the first row of a pass).
func unfilter(mutating row: Array[UInt8], previous: Array[UInt8], filter: Int64, stride: Int64) {
    let n = row.count;
    if filter == 0 { return; }
    row.ensureUnique();
    let p = row.asUnsafeSlice().pointer;
    let up = previous.asUnsafeSlice().pointer;
    if filter == 1 {
        for i in stride..<n {
            p.offset(by: i).write(p.offset(by: i).read() + p.offset(by: i - stride).read());
        }
    } else if filter == 2 {
        for i in 0..<n {
            p.offset(by: i).write(p.offset(by: i).read() + up.offset(by: i).read());
        }
    } else if filter == 3 {
        for i in 0..<n {
            var left: Int64 = 0;
            if i >= stride { left = Int64(from: p.offset(by: i - stride).read()); }
            let avg = (left + Int64(from: up.offset(by: i).read())) / 2;
            p.offset(by: i).write(p.offset(by: i).read() + UInt8(truncating: avg));
        }
    } else {
        for i in 0..<n {
            var left: Int64 = 0;
            var corner: Int64 = 0;
            if i >= stride {
                left = Int64(from: p.offset(by: i - stride).read());
                corner = Int64(from: up.offset(by: i - stride).read());
            }
            let above = Int64(from: up.offset(by: i).read());
            p.offset(by: i).write(p.offset(by: i).read() + UInt8(truncating: paeth(left, above, corner)));
        }
    }
}

func paeth(a: Int64, b: Int64, c: Int64) -> Int64 {
    let p = a + b - c;
    var pa = p - a;
    if pa < 0 { pa = 0 - pa; }
    var pb = p - b;
    if pb < 0 { pb = 0 - pb; }
    var pc = p - c;
    if pc < 0 { pc = 0 - pc; }
    if pa <= pb and pa <= pc { return a; }
    if pb <= pc { return b; }
    c
}

/// The index of the first zero byte at or after `start`, or -1.
func indexOfZero(data: Array[UInt8], start: Int64) -> Int64 {
    var i = start;
    while i < data.count {
        if data(i) == 0 { return i; }
        i = i + 1;
    }
    return -1;
}

/// The bytes of `data` from `start` on.
func tailBytes(data: Array[UInt8], start: Int64) -> Array[UInt8] {
    var out: [UInt8] = [];
    var i = start;
    while i < data.count {
        out.append(data(i));
        i = i + 1;
    }
    out
}

/// Latin-1 bytes `start..<end` as a string.
func latin1ToString(data: Array[UInt8], start: Int64, end: Int64) -> String {
    var utf8: [UInt8] = [];
    var i = start;
    while i < end {
        let b = data(i);
        if b < 128 {
            utf8.append(b);
        } else {
            utf8.append(0xC0 | (b >> 6));
            utf8.append(0x80 | (b & 0x3F));
        }
        i = i + 1;
    }
    String(fromUtf8Lossy: utf8)
}

// ---------------------------------------------------------------------------
// Encoding
// ---------------------------------------------------------------------------

func appendChunk(mutating out: Array[UInt8], kind: String, body: Array[UInt8]) {
    appendUInt32BE(out, body.count);
    var typed: [UInt8] = [];
    appendAscii(typed, kind);
    out.append(contentsOf: typed);
    out.append(contentsOf: body);
    var crc = Crc32();
    crc.update(typed);
    crc.update(body);
    appendUInt32BE(out, Int64(from: crc.value));
}

func isAscii(text: String) -> Bool {
    for b in text.bytes {
        if b >= 128 { return false; }
    }
    true
}

func encodePng(image: RawImage, level: CompressionLevel, keepMetadata: Bool) -> Array[UInt8] {
    let plane = image.planes(0);
    let layout = plane.layout;
    let sampleBytes = layout.sample.bytes;
    let bpp = layout.bytesPerPixel;
    let rowLength = image.width * bpp;

    var out: [UInt8] = [137, 80, 78, 71, 13, 10, 26, 10];
    var header: [UInt8] = [];
    appendUInt32BE(header, image.width);
    appendUInt32BE(header, image.height);
    header.append(UInt8(truncating: sampleBytes * 8));
    header.append(UInt8(truncating: pngColorType(layout)));
    header.append(0);
    header.append(0);
    header.append(0);
    appendChunk(out, "IHDR", header);

    if image.color != ColorInfo.srgb {
        let c = image.color;
        var fullRange: UInt8 = 0;
        if c.fullRange { fullRange = 1; }
        let body: [UInt8] = [UInt8(truncating: c.primaries), UInt8(truncating: c.transfer), UInt8(truncating: c.matrix), fullRange];
        appendChunk(out, "cICP", body);
    }
    let meta = image.metadata;
    if let .Some(profile) = meta.iccProfile {
        var body: [UInt8] = [];
        appendAscii(body, "ICC profile");
        body.append(0);
        body.append(0);
        body.append(contentsOf: Zlib.compress(profile, level: level));
        appendChunk(out, "iCCP", body);
    }
    if let .Some(dpm) = meta.dotsPerMeter {
        var body: [UInt8] = [];
        appendUInt32BE(body, dpm.0);
        appendUInt32BE(body, dpm.1);
        body.append(1);
        appendChunk(out, "pHYs", body);
    }
    if keepMetadata {
        if let .Some(exif) = meta.exif { appendChunk(out, "eXIf", exif); }
        for entry in meta.text {
            var body: [UInt8] = [];
            appendAscii(body, entry.0);
            body.append(0);
            if isAscii(entry.1) {
                appendAscii(body, entry.1);
                appendChunk(out, "tEXt", body);
            } else {
                // iTXt: uncompressed, no language or translated keyword.
                body.append(0);
                body.append(0);
                body.append(0);
                body.append(0);
                appendAscii(body, entry.1);
                appendChunk(out, "iTXt", body);
            }
        }
        if let .Some(xmp) = meta.xmp {
            var body: [UInt8] = [];
            appendAscii(body, "XML:com.adobe.xmp");
            for _ in 0..<5 { body.append(0); }
            appendAscii(body, xmp);
            appendChunk(out, "iTXt", body);
        }
        for chunk in meta.unknownChunks {
            if chunk.0 == "IHDR" or chunk.0 == "IDAT" or chunk.0 == "IEND" or chunk.0 == "PLTE" { continue; }
            appendChunk(out, chunk.0, chunk.1);
        }
    }

    // Filter each row with whichever filter gives the smallest sum of
    // absolute values, the heuristic libpng uses.
    var filtered: [UInt8] = [];
    var previous = Array[UInt8](repeating: 0, count: rowLength);
    var row = Array[UInt8](repeating: 0, count: rowLength);
    var candidate = Array[UInt8](repeating: 0, count: rowLength);
    var best = Array[UInt8](repeating: 0, count: rowLength);
    for y in 0..<image.height {
        let start = y * plane.strideBytes;
        // PNG stores 16-bit samples big-endian.
        for i in 0..<rowLength {
            if sampleBytes == 2 {
                let pair = i - i % 2;
                row(i) = plane.bytes(start + pair + 1 - i % 2);
            } else {
                row(i) = plane.bytes(start + i);
            }
        }
        var bestFilter: Int64 = 0;
        var bestScore: Int64 = -1;
        for f in 0..<5 {
            let score = applyFilter(row, previous, f, bpp, candidate);
            if bestScore < 0 or score < bestScore {
                bestScore = score;
                bestFilter = f;
                let swap = best;
                best = candidate;
                candidate = swap;
            }
        }
        filtered.append(UInt8(truncating: bestFilter));
        filtered.append(contentsOf: best);
        let swap = previous;
        previous = row;
        row = swap;
    }
    let compressed = Zlib.compress(filtered, level: level);
    // IDAT chunks of at most 1 MiB.
    var offset: Int64 = 0;
    loop {
        var size = compressed.count - offset;
        if size > 1048576 { size = 1048576; }
        var body: [UInt8] = [];
        for i in 0..<size { body.append(compressed(offset + i)); }
        appendChunk(out, "IDAT", body);
        offset = offset + size;
        if offset >= compressed.count { break; }
    }
    let empty: [UInt8] = [];
    appendChunk(out, "IEND", empty);
    out
}

/// Writes `row` filtered with `filter` into `out` and returns its cost:
/// the sum of the bytes read as signed values.
func applyFilter(row: Array[UInt8], previous: Array[UInt8], filter: Int64, stride: Int64, mutating out: Array[UInt8]) -> Int64 {
    out.ensureUnique();
    let n = row.count;
    let r = row.asUnsafeSlice().pointer;
    let up = previous.asUnsafeSlice().pointer;
    let o = out.asUnsafeSlice().pointer;
    var cost: Int64 = 0;
    for i in 0..<n {
        let x = Int64(from: r.offset(by: i).read());
        var a: Int64 = 0;
        var c: Int64 = 0;
        if i >= stride {
            a = Int64(from: r.offset(by: i - stride).read());
            c = Int64(from: up.offset(by: i - stride).read());
        }
        let b = Int64(from: up.offset(by: i).read());
        var predicted: Int64 = 0;
        if filter == 1 { predicted = a; }
        if filter == 2 { predicted = b; }
        if filter == 3 { predicted = (a + b) / 2; }
        if filter == 4 { predicted = paeth(a, b, c); }
        let v = (x - predicted) & 0xFF;
        o.offset(by: i).write(UInt8(truncating: v));
        if v < 128 { cost = cost + v; } else { cost = cost + 256 - v; }
    }
    cost
}
