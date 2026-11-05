module images

// Windows BMP. Decodes 1, 4 and 8-bit palettes and 16, 24 and 32-bit
// pixels, including bit-field masks and alpha; RLE compression is not
// supported. Encodes 24-bit RGB and 32-bit RGBA.

/// The BMP format.
public struct BmpFormat: ImageFormat {
    public init() {}

    public var name: String { "bmp" }
    public var extensions: Array[String] { ["bmp", "dib"] }

    public func matches(header: Array[UInt8]) -> Bool { startsWith(header, "BM") }

    public func makeDecoder(options: DecodeOptions) -> any ImageDecoder { BmpDecoder(options: options) }

    public func canEncode(layout: PixelLayout) -> Bool { layout == .rgb8 or layout == .rgba8 }

    public func preferredLayout(for layout: PixelLayout) -> PixelLayout {
        if layout.channels.hasAlpha { return .rgba8; }
        .rgb8
    }

    public func encode(image: RawImage, options: EncodeOptions) -> Result[Array[UInt8], ImageError] {
        if not image.isInterleaved or not self.canEncode(image.layout) {
            return .Err(ImageError(kind: .layoutMismatch, format: "bmp", message: "BMP writes rgb8 or rgba8, not \(image.layout.description)"));
        }
        .Ok(encodeBmp(image))
    }
}

/// One channel's bit field: where it sits in a pixel and how to scale it
/// to 8 bits.
struct BitField {
    let mask: Int64;
    let shift: Int64;
    let maximum: Int64;

    init(mask mask: Int64) {
        self.mask = mask;
        var shift: Int64 = 0;
        if mask != 0 {
            while ((mask >> shift) & 1) == 0 { shift = shift + 1; }
        }
        self.shift = shift;
        self.maximum = mask >> shift;
    }

    func extract(pixel: Int64) -> UInt8 {
        if self.mask == 0 { return 255; }
        let v = (pixel & self.mask) >> self.shift;
        UInt8(from: (v * 255 + self.maximum / 2) / self.maximum)
    }
}

struct BmpDecoder: ImageDecoder {
    let options: DecodeOptions;
    var queue: ByteQueue;
    /// 0 = headers, 1 = pixels, 2 = done.
    var stage: Int64;
    var width: Int64;
    var height: Int64;
    var topDown: Bool;
    var bitsPerPixel: Int64;
    var palette: Array[UInt8];
    var red: BitField;
    var green: BitField;
    var blue: BitField;
    var alphaField: BitField;
    var layout: PixelLayout;
    var rowStride: Int64;
    var row: Int64;
    var deliver: Bool;

    init(options options: DecodeOptions) {
        self.options = options;
        self.queue = ByteQueue();
        self.stage = 0;
        self.width = 0;
        self.height = 0;
        self.topDown = false;
        self.bitsPerPixel = 24;
        self.palette = [];
        self.red = BitField(mask: 0);
        self.green = BitField(mask: 0);
        self.blue = BitField(mask: 0);
        self.alphaField = BitField(mask: 0);
        self.layout = .rgb8;
        self.rowStride = 0;
        self.row = 0;
        self.deliver = true;
    }

    mutating func write(input: Array[UInt8], mutating into sink: any ImageSink) -> Result[DecodeStatus, ImageError] {
        if self.stage == 2 { return .Ok(.done); }
        self.queue.append(input);
        if self.stage == 0 {
            if self.queue.available < 18 { return .Ok(.needsInput); }
            let pixelOffset = self.queue.peekUInt32LE(10);
            if pixelOffset < 26 or pixelOffset > 65536 { return .Err(corrupt("bmp", "bad pixel data offset")); }
            if self.queue.available < pixelOffset { return .Ok(.needsInput); }
            let headers = self.queue.takeBytes(pixelOffset);
            try self.readHeaders(headers, into: sink);
        }
        while self.row < self.height {
            if self.queue.available < self.rowStride { return .Ok(.needsInput); }
            let data = self.queue.takeBytes(self.rowStride);
            self.emitRow(data, into: sink);
            self.row = self.row + 1;
        }
        if self.stage == 1 {
            self.stage = 2;
            if self.deliver { sink.endImage(0, ImageMetadata()); }
        }
        .Ok(.done)
    }

    mutating func finish(mutating into sink: any ImageSink) -> Result[(), ImageError] {
        if self.stage != 2 { return .Err(truncated("bmp")); }
        .Ok(())
    }

    mutating func readHeaders(headers: Array[UInt8], mutating into sink: any ImageSink) -> Result[(), ImageError] {
        let infoSize = readUInt32LE(headers, 14);
        var paletteEntrySize: Int64 = 4;
        var compression: Int64 = 0;
        var colorsUsed: Int64 = 0;
        if infoSize == 12 {
            self.width = readUInt16LE(headers, 18);
            self.height = readUInt16LE(headers, 20);
            self.bitsPerPixel = readUInt16LE(headers, 24);
            paletteEntrySize = 3;
        } else if infoSize >= 40 and 14 + infoSize <= headers.count {
            self.width = signed32(readUInt32LE(headers, 18));
            self.height = signed32(readUInt32LE(headers, 22));
            self.bitsPerPixel = readUInt16LE(headers, 28);
            compression = readUInt32LE(headers, 30);
            colorsUsed = readUInt32LE(headers, 46);
        } else {
            return .Err(unsupported("bmp", "header size \(infoSize)"));
        }
        if self.height < 0 {
            self.topDown = true;
            self.height = 0 - self.height;
        }
        if self.width <= 0 { return .Err(corrupt("bmp", "bad width")); }
        try self.options.check("bmp", self.width, self.height);
        let bpp = self.bitsPerPixel;
        if compression == 1 or compression == 2 { return .Err(unsupported("bmp", "RLE compression")); }
        if compression != 0 and compression != 3 and compression != 6 {
            return .Err(unsupported("bmp", "compression \(compression)"));
        }
        if bpp != 1 and bpp != 4 and bpp != 8 and bpp != 16 and bpp != 24 and bpp != 32 {
            return .Err(unsupported("bmp", "\(bpp) bits per pixel"));
        }

        // Bit fields: in the header for V3+ headers, or right after a
        // 40-byte header.
        if compression == 3 or compression == 6 {
            let at = 14 + 40;
            if at + 12 > headers.count { return .Err(corrupt("bmp", "missing bit masks")); }
            self.red = BitField(mask: readUInt32LE(headers, at));
            self.green = BitField(mask: readUInt32LE(headers, at + 4));
            self.blue = BitField(mask: readUInt32LE(headers, at + 8));
            if (infoSize >= 56 or compression == 6) and at + 16 <= headers.count {
                self.alphaField = BitField(mask: readUInt32LE(headers, at + 12));
            }
        } else if bpp == 16 {
            self.red = BitField(mask: 0x7C00);
            self.green = BitField(mask: 0x03E0);
            self.blue = BitField(mask: 0x001F);
        } else if bpp == 32 {
            self.red = BitField(mask: 0x00FF0000);
            self.green = BitField(mask: 0x0000FF00);
            self.blue = BitField(mask: 0x000000FF);
        }

        if bpp <= 8 {
            var entries = colorsUsed;
            if entries == 0 or entries > (1 << bpp) { entries = 1 << bpp; }
            let start = 14 + infoSize;
            for i in 0..<entries {
                let at = start + i * paletteEntrySize;
                if at + 3 > headers.count { break; }
                self.palette.append(headers(at + 2));
                self.palette.append(headers(at + 1));
                self.palette.append(headers(at));
            }
        }
        if self.alphaField.mask != 0 { self.layout = .rgba8; }
        self.rowStride = ((self.width * bpp + 31) / 32) * 4;
        var alpha = AlphaMode.none;
        if self.layout == .rgba8 { alpha = .straight; }
        let info = ImageInfo(width: self.width, height: self.height, layout: self.layout, color: .srgb, alpha: alpha, metadata: ImageMetadata());
        self.deliver = sink.beginImage(0, info) == .decode;
        self.stage = 1;
        .Ok(())
    }

    mutating func emitRow(data: Array[UInt8], mutating into sink: any ImageSink) {
        if not self.deliver { return; }
        let channels = self.layout.channels.count;
        var out = Array[UInt8](repeating: 0, count: self.width * channels);
        let bpp = self.bitsPerPixel;
        let entries = self.palette.count / 3;
        for x in 0..<self.width {
            let o = x * channels;
            if bpp <= 8 {
                let index = unpackSample(data, x, bpp);
                if index < entries {
                    out(o) = self.palette(index * 3);
                    out(o + 1) = self.palette(index * 3 + 1);
                    out(o + 2) = self.palette(index * 3 + 2);
                }
            } else if bpp == 24 {
                out(o) = data(x * 3 + 2);
                out(o + 1) = data(x * 3 + 1);
                out(o + 2) = data(x * 3);
            } else {
                var pixel = readUInt16LE(data, x * 2);
                if bpp == 32 { pixel = readUInt32LE(data, x * 4); }
                out(o) = self.red.extract(pixel);
                out(o + 1) = self.green.extract(pixel);
                out(o + 2) = self.blue.extract(pixel);
                if channels == 4 { out(o + 3) = self.alphaField.extract(pixel); }
            }
        }
        var y = self.row;
        if not self.topDown { y = self.height - 1 - self.row; }
        sink.receive(0, Rect(x: 0, y: y, width: self.width, height: 1), out, out.count);
    }
}

/// A 32-bit field read as a signed value.
func signed32(value: Int64) -> Int64 {
    if value >= 0x80000000 { return value - 0x100000000; }
    value
}

func encodeBmp(image: RawImage) -> Array[UInt8] {
    let plane = image.planes(0);
    let channels = plane.layout.channels.count;
    let withAlpha = channels == 4;
    var bpp: Int64 = 24;
    var infoSize: Int64 = 40;
    if withAlpha {
        bpp = 32;
        infoSize = 108;
    }
    let rowStride = ((image.width * bpp + 31) / 32) * 4;
    let pixelOffset = 14 + infoSize;
    var out: [UInt8] = [66, 77];
    appendUInt32LE(out, pixelOffset + rowStride * image.height);
    appendUInt32LE(out, 0);
    appendUInt32LE(out, pixelOffset);
    appendUInt32LE(out, infoSize);
    appendUInt32LE(out, image.width);
    appendUInt32LE(out, image.height);
    appendUInt16LE(out, 1);
    appendUInt16LE(out, bpp);
    var compression: Int64 = 0;
    if withAlpha { compression = 3; }
    appendUInt32LE(out, compression);
    appendUInt32LE(out, rowStride * image.height);
    // 2835 pixels per meter is 72 DPI.
    var dpm = (2835, 2835);
    if let .Some(stored) = image.metadata.dotsPerMeter { dpm = stored; }
    appendUInt32LE(out, dpm.0);
    appendUInt32LE(out, dpm.1);
    appendUInt32LE(out, 0);
    appendUInt32LE(out, 0);
    if withAlpha {
        appendUInt32LE(out, 0x00FF0000);
        appendUInt32LE(out, 0x0000FF00);
        appendUInt32LE(out, 0x000000FF);
        appendUInt32LE(out, 0xFF000000);
        // "sRGB" color space, then unused endpoints and gamma.
        appendUInt32LE(out, 0x73524742);
        for _ in 0..<12 { appendUInt32LE(out, 0); }
    }
    // Bottom-up rows of BGR or BGRA.
    var y = image.height - 1;
    while y >= 0 {
        let start = y * plane.strideBytes;
        for x in 0..<image.width {
            let at = start + x * channels;
            out.append(plane.bytes(at + 2));
            out.append(plane.bytes(at + 1));
            out.append(plane.bytes(at));
            if withAlpha { out.append(plane.bytes(at + 3)); }
        }
        for _ in 0..<(rowStride - image.width * channels) { out.append(0); }
        y = y - 1;
    }
    out
}
