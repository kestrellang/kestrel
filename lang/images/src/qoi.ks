module images

// QOI, the "Quite OK Image" format (https://qoiformat.org): 8-bit RGB or
// RGBA, losslessly compressed with runs, a 64-entry color cache and small
// deltas.

let qoiOpIndex: Int64 = 0x00;
let qoiOpDiff: Int64 = 0x40;
let qoiOpLuma: Int64 = 0x80;
let qoiOpRun: Int64 = 0xC0;
let qoiOpRgb: Int64 = 0xFE;
let qoiOpRgba: Int64 = 0xFF;
let qoiHeaderSize: Int64 = 14;
let qoiEndSize: Int64 = 8;

/// The QOI format.
public struct QoiFormat: ImageFormat {
    public init() {}

    public var name: String { "qoi" }
    public var extensions: Array[String] { ["qoi"] }

    public func matches(header: Array[UInt8]) -> Bool { startsWith(header, "qoif") }

    public func makeDecoder(options: DecodeOptions) -> any ImageDecoder { QoiDecoder(options: options) }

    public func canEncode(layout: PixelLayout) -> Bool { layout == .rgba8 or layout == .rgb8 }

    public func preferredLayout(for layout: PixelLayout) -> PixelLayout {
        if layout.channels.hasAlpha { return .rgba8; }
        .rgb8
    }

    public func encode(image: RawImage, options: EncodeOptions) -> Result[Array[UInt8], ImageError] {
        if not image.isInterleaved or not self.canEncode(image.layout) {
            return .Err(ImageError(kind: .layoutMismatch, format: "qoi", message: "QOI writes rgb8 or rgba8, not \(image.layout.description)"));
        }
        .Ok(encodeQoi(image))
    }
}

/// The running hash QOI uses to index its color cache.
func qoiHash(r: Int64, g: Int64, b: Int64, a: Int64) -> Int64 {
    (r * 3 + g * 5 + b * 7 + a * 11) % 64
}

struct QoiDecoder: ImageDecoder {
    let options: DecodeOptions;
    var queue: ByteQueue;
    /// 0 = header, 1 = pixels, 2 = end marker, 3 = done.
    var stage: Int64;
    var width: Int64;
    var height: Int64;
    var channels: Int64;
    var deliver: Bool;
    var cache: Array[UInt8];
    var r: Int64;
    var g: Int64;
    var b: Int64;
    var a: Int64;
    var run: Int64;
    var row: Array[UInt8];
    var x: Int64;
    var y: Int64;

    init(options options: DecodeOptions) {
        self.options = options;
        self.queue = ByteQueue();
        self.stage = 0;
        self.width = 0;
        self.height = 0;
        self.channels = 4;
        self.deliver = true;
        self.cache = Array[UInt8](repeating: 0, count: 256);
        self.r = 0;
        self.g = 0;
        self.b = 0;
        self.a = 255;
        self.run = 0;
        self.row = [];
        self.x = 0;
        self.y = 0;
    }

    mutating func write(input: Array[UInt8], mutating into sink: any ImageSink) -> Result[DecodeStatus, ImageError] {
        if self.stage == 3 {
            if input.count > 0 { return .Err(corrupt("qoi", "data after the end marker")); }
            return .Ok(.done);
        }
        self.queue.append(input);
        if self.stage == 0 {
            if self.queue.available < qoiHeaderSize { return .Ok(.needsInput); }
            try self.readHeader(into: sink);
        }
        if self.stage == 1 { self.decodePixels(into: sink); }
        if self.stage == 2 {
            if self.queue.available < qoiEndSize { return .Ok(.needsInput); }
            for i in 0..<7 {
                if self.queue.peek(i) != 0 { return .Err(corrupt("qoi", "bad end marker")); }
            }
            if self.queue.peek(7) != 1 { return .Err(corrupt("qoi", "bad end marker")); }
            self.queue.skip(qoiEndSize);
            if self.queue.available > 0 { return .Err(corrupt("qoi", "data after the end marker")); }
            self.stage = 3;
            sink.endImage(0, ImageMetadata());
            return .Ok(.done);
        }
        .Ok(.needsInput)
    }

    mutating func finish(mutating into sink: any ImageSink) -> Result[(), ImageError] {
        if self.stage != 3 { return .Err(truncated("qoi")); }
        .Ok(())
    }

    mutating func readHeader(mutating into sink: any ImageSink) -> Result[(), ImageError] {
        if self.queue.peek(0) != 113 or self.queue.peek(1) != 111 or self.queue.peek(2) != 105 or self.queue.peek(3) != 102 {
            return .Err(corrupt("qoi", "missing qoif signature"));
        }
        self.width = self.queue.peekUInt32BE(4);
        self.height = self.queue.peekUInt32BE(8);
        self.channels = Int64(from: self.queue.peek(12));
        let colorspace = self.queue.peek(13);
        self.queue.skip(qoiHeaderSize);
        if self.channels != 3 and self.channels != 4 {
            return .Err(corrupt("qoi", "channel count must be 3 or 4"));
        }
        try self.options.check("qoi", self.width, self.height);
        var layout = PixelLayout.rgba8;
        var alpha = AlphaMode.straight;
        if self.channels == 3 {
            layout = .rgb8;
            alpha = .none;
        }
        var color = ColorInfo.srgb;
        if colorspace == 1 { color = .linearSrgb; }
        let info = ImageInfo(width: self.width, height: self.height, layout: layout, color: color, alpha: alpha, metadata: ImageMetadata());
        self.deliver = sink.beginImage(0, info) == .decode;
        self.row = Array[UInt8](repeating: 0, count: self.width * self.channels);
        self.stage = 1;
        .Ok(())
    }

    /// Decodes as many pixels as the queued input allows. Every op is at
    /// most 5 bytes, so one is decoded only once all of it has arrived.
    mutating func decodePixels(mutating into sink: any ImageSink) {
        while self.y < self.height {
            if self.run > 0 {
                self.run = self.run - 1;
            } else {
                if self.queue.available == 0 { return; }
                let op = Int64(from: self.queue.peek(0));
                var need: Int64 = 1;
                if op == qoiOpRgb { need = 4; }
                if op == qoiOpRgba { need = 5; }
                if op < 0xFE and (op & 0xC0) == qoiOpLuma { need = 2; }
                if self.queue.available < need { return; }
                self.queue.skip(need);
                self.applyOp(op);
            }
            let at = self.x * self.channels;
            self.row(at) = UInt8(truncating: self.r);
            self.row(at + 1) = UInt8(truncating: self.g);
            self.row(at + 2) = UInt8(truncating: self.b);
            if self.channels == 4 { self.row(at + 3) = UInt8(truncating: self.a); }
            self.x = self.x + 1;
            if self.x == self.width {
                if self.deliver {
                    sink.receive(0, Rect(x: 0, y: self.y, width: self.width, height: 1), self.row, self.width * self.channels);
                }
                self.x = 0;
                self.y = self.y + 1;
            }
        }
        self.stage = 2;
    }

    /// Updates the current color from one op, whose bytes have just been
    /// consumed (they sit right before the queue position).
    mutating func applyOp(op: Int64) {
        let base = self.queue.position;
        if op == qoiOpRgb {
            self.r = Int64(from: self.queue.data(base - 3));
            self.g = Int64(from: self.queue.data(base - 2));
            self.b = Int64(from: self.queue.data(base - 1));
        } else if op == qoiOpRgba {
            self.r = Int64(from: self.queue.data(base - 4));
            self.g = Int64(from: self.queue.data(base - 3));
            self.b = Int64(from: self.queue.data(base - 2));
            self.a = Int64(from: self.queue.data(base - 1));
        } else {
            let tag = op & 0xC0;
            if tag == qoiOpIndex {
                let i = op * 4;
                self.r = Int64(from: self.cache(i));
                self.g = Int64(from: self.cache(i + 1));
                self.b = Int64(from: self.cache(i + 2));
                self.a = Int64(from: self.cache(i + 3));
            } else if tag == qoiOpDiff {
                self.r = (self.r + ((op >> 4) & 3) - 2) & 0xFF;
                self.g = (self.g + ((op >> 2) & 3) - 2) & 0xFF;
                self.b = (self.b + (op & 3) - 2) & 0xFF;
            } else if tag == qoiOpLuma {
                let second = Int64(from: self.queue.data(base - 1));
                let dg = (op & 0x3F) - 32;
                self.r = (self.r + dg - 8 + ((second >> 4) & 0x0F)) & 0xFF;
                self.g = (self.g + dg) & 0xFF;
                self.b = (self.b + dg - 8 + (second & 0x0F)) & 0xFF;
            } else {
                // Run of 1 to 62 pixels; this one plus `run` more.
                self.run = op & 0x3F;
            }
        }
        let h = qoiHash(self.r, self.g, self.b, self.a) * 4;
        self.cache(h) = UInt8(truncating: self.r);
        self.cache(h + 1) = UInt8(truncating: self.g);
        self.cache(h + 2) = UInt8(truncating: self.b);
        self.cache(h + 3) = UInt8(truncating: self.a);
    }
}

func encodeQoi(image: RawImage) -> Array[UInt8] {
    let plane = image.planes(0);
    let channels = plane.layout.channels.count;
    var out: [UInt8] = [];
    appendAscii(out, "qoif");
    appendUInt32BE(out, image.width);
    appendUInt32BE(out, image.height);
    out.append(UInt8(truncating: channels));
    var colorspace: UInt8 = 0;
    if image.color.isLinear { colorspace = 1; }
    out.append(colorspace);

    var cache = Array[Int64](repeating: -1, count: 64);
    var pr: Int64 = 0;
    var pg: Int64 = 0;
    var pb: Int64 = 0;
    var pa: Int64 = 255;
    var run: Int64 = 0;
    let total = image.width * image.height;
    var index: Int64 = 0;
    for y in 0..<image.height {
        let rowStart = y * plane.strideBytes;
        for x in 0..<image.width {
            let at = rowStart + x * channels;
            let r = Int64(from: plane.bytes(at));
            let g = Int64(from: plane.bytes(at + 1));
            let b = Int64(from: plane.bytes(at + 2));
            var a: Int64 = 255;
            if channels == 4 { a = Int64(from: plane.bytes(at + 3)); }
            index = index + 1;
            if r == pr and g == pg and b == pb and a == pa {
                run = run + 1;
                if run == 62 or index == total {
                    out.append(UInt8(truncating: qoiOpRun | (run - 1)));
                    run = 0;
                }
                continue;
            }
            if run > 0 {
                out.append(UInt8(truncating: qoiOpRun | (run - 1)));
                run = 0;
            }
            let packed = (r << 24) | (g << 16) | (b << 8) | a;
            let h = qoiHash(r, g, b, a);
            if cache(h) == packed {
                out.append(UInt8(truncating: qoiOpIndex | h));
            } else {
                cache(h) = packed;
                if a == pa {
                    let dr = wrapDelta(r - pr);
                    let dg = wrapDelta(g - pg);
                    let db = wrapDelta(b - pb);
                    let drg = dr - dg;
                    let dbg = db - dg;
                    if dr > -3 and dr < 2 and dg > -3 and dg < 2 and db > -3 and db < 2 {
                        out.append(UInt8(truncating: qoiOpDiff | ((dr + 2) << 4) | ((dg + 2) << 2) | (db + 2)));
                    } else if drg > -9 and drg < 8 and dg > -33 and dg < 32 and dbg > -9 and dbg < 8 {
                        out.append(UInt8(truncating: qoiOpLuma | (dg + 32)));
                        out.append(UInt8(truncating: ((drg + 8) << 4) | (dbg + 8)));
                    } else {
                        out.append(UInt8(truncating: qoiOpRgb));
                        out.append(UInt8(truncating: r));
                        out.append(UInt8(truncating: g));
                        out.append(UInt8(truncating: b));
                    }
                } else {
                    out.append(UInt8(truncating: qoiOpRgba));
                    out.append(UInt8(truncating: r));
                    out.append(UInt8(truncating: g));
                    out.append(UInt8(truncating: b));
                    out.append(UInt8(truncating: a));
                }
            }
            pr = r;
            pg = g;
            pb = b;
            pa = a;
        }
    }
    for _ in 0..<7 { out.append(0); }
    out.append(1);
    out
}

/// A byte difference wrapped into -128...127.
func wrapDelta(d: Int64) -> Int64 {
    var v = d & 0xFF;
    if v > 127 { v = v - 256; }
    v
}
