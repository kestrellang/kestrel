// Round trips through every encoder, color conversion, HDR tone mapping,
// metadata, orientation, operations, limits and malformed input. Also
// saves images to $IMAGES_OUT for check_with_python.py.

module images_tests

import deflate.(CompressionLevel)
import images.(Image, RawImage, Plane, PixelLayout, ColorInfo, AlphaMode, Orientation, Rect, ImageMetadata, ImageError, ImageErrorKind)
import images.(Pixel, Gray8, GrayAlpha8, Rgb8, Rgba8, Rgba16, RgbaF16, RgbaF32, halfToFloat, floatToHalf)
import images.(ImageFormat, PngFormat, QoiFormat, BmpFormat, Formats, DecodeOptions, EncodeOptions, ConversionOptions, ResizeFilter, exifOrientation)
import images.(ImageDecoder, ImageSink, ImageInfo, SinkChoice, DecodeStatus, CollectingSink)

/// A deterministic test image: every byte of every pixel varies with x, y
/// and channel. make_fixtures and check_with_python use the same formula.
func patternRaw(layout: PixelLayout, width: Int64, height: Int64) -> RawImage {
    var raw = RawImage(width: width, height: height, layout: layout);
    var plane = raw.planes(0);
    let bpp = layout.bytesPerPixel;
    for y in 0..<height {
        for x in 0..<width {
            for k in 0..<bpp {
                plane.bytes((y * width + x) * bpp + k) = UInt8(from: (x * 7 + y * 13 + k * 51 + x * y) % 256);
            }
        }
    }
    raw.planes(0) = plane;
    // Through a binding: see `subscript_assign_then_return_array`.
    let out = raw;
    out
}

func sameBytes(a: RawImage, b: RawImage) -> Bool {
    if a.width != b.width or a.height != b.height or a.layout != b.layout { return false; }
    let pa = a.planes(0);
    let pb = b.planes(0);
    let row = a.width * a.layout.bytesPerPixel;
    for y in 0..<a.height {
        for i in 0..<row {
            if pa.bytes(y * pa.strideBytes + i) != pb.bytes(y * pb.strideBytes + i) { return false; }
        }
    }
    true
}

func roundTrip(format: any ImageFormat, layout: PixelLayout, label: String) {
    let raw = patternRaw(layout, 37, 23);
    match format.encode(raw, EncodeOptions()) {
        .Ok(bytes) => {
            var formats = Formats.standard;
            match RawImage.decode(data: bytes, formats: formats) {
                .Ok(back) => check("\(label) round trip", sameBytes(raw, back)),
                .Err(e) => check("\(label) decodes: \(e.description)", false)
            }
            let path = directory("IMAGES_OUT") + label + "." + format.name;
            check("\(label) saves", raw.save(path: path).isOk);
        },
        .Err(e) => check("\(label) encodes: \(e.description)", false)
    }
}

func errorKind(result: Result[RawImage, ImageError]) -> String {
    match result {
        .Ok(_) => "ok",
        .Err(e) => e.kind.name
    }
}

func near(a: Float32, b: Float32, tolerance: Float32) -> Bool { (a - b).abs() <= tolerance }

/// A format that recognizes "TEST" and decodes a 1x1 red image, to show
/// third-party formats plug in.
struct TestFormat: ImageFormat {
    var name: String { "test" }
    var extensions: Array[String] { ["tst"] }
    func matches(header: Array[UInt8]) -> Bool { header.count >= 4 and header(0) == 84 and header(1) == 69 }
    func makeDecoder(options: DecodeOptions) -> any ImageDecoder { TestDecoder() }
    func canEncode(layout: PixelLayout) -> Bool { false }
    func preferredLayout(for layout: PixelLayout) -> PixelLayout { .rgb8 }
    func encode(image: RawImage, options: EncodeOptions) -> Result[Array[UInt8], ImageError] {
        .Err(ImageError(kind: .unsupported, format: "test", message: "decode only"))
    }
}

struct TestDecoder: ImageDecoder {
    mutating func write(input: Array[UInt8], mutating into sink: any ImageSink) -> Result[DecodeStatus, ImageError] {
        let info = ImageInfo(width: 1, height: 1, layout: .rgb8, color: .srgb, alpha: .none, metadata: ImageMetadata());
        if sink.beginImage(0, info) == .decode {
            let red: [UInt8] = [255, 0, 0];
            sink.receive(0, Rect(x: 0, y: 0, width: 1, height: 1), red, 3);
            sink.endImage(0, ImageMetadata());
        }
        .Ok(.done)
    }
    mutating func finish(mutating into sink: any ImageSink) -> Result[(), ImageError] { .Ok(()) }
}

func testRoundTrips() {
    let png: any ImageFormat = PngFormat();
    for layout in [PixelLayout.gray8, PixelLayout.grayAlpha8, PixelLayout.rgb8, PixelLayout.rgba8, PixelLayout.gray16, PixelLayout.grayAlpha16, PixelLayout.rgb16, PixelLayout.rgba16] {
        roundTrip(png, layout, layout.description);
    }
    let stored: any ImageFormat = PngFormat(level: .none);
    roundTrip(stored, .rgba8, "rgba8-stored");
    let qoi: any ImageFormat = QoiFormat();
    roundTrip(qoi, .rgb8, "rgb8");
    roundTrip(qoi, .rgba8, "rgba8");
    let bmp: any ImageFormat = BmpFormat();
    roundTrip(bmp, .rgb8, "rgb8");
    roundTrip(bmp, .rgba8, "rgba8");

    // Saving a layout PNG can't hold converts it first.
    let floats = Image[RgbaF32](width: 2, height: 2, filling: RgbaF32(r: 0.5, g: 0.25, b: 1.0, a: 1.0));
    match floats.encoded(format: "png") {
        .Ok(bytes) => match RawImage.decode(data: bytes) {
            .Ok(raw) => check("float image saves as rgba16 png", raw.layout == .rgba16 and raw.color == ColorInfo.linearSrgb),
            .Err(_) => check("float png decodes", false)
        },
        .Err(e) => check("float image encodes: \(e.description)", false)
    }
}

func testConversion() {
    // 8-bit sRGB → linear float → 8-bit sRGB is exact for every value.
    var gray = Image[Rgba8](width: 256, height: 1);
    for v in 0..<256 {
        let b = UInt8(from: v);
        gray(v, 0) = Rgba8(r: b, g: b, b: b, a: 255);
    }
    var toLinear = ConversionOptions();
    toLinear.color = .Some(.linearSrgb);
    match gray.converted[RgbaF32](options: toLinear) {
        .Ok(linear) => {
            check("sRGB 128 is linear 0.2158", near(linear(128, 0).r, 0.2158605, 0.0001));
            var back = ConversionOptions();
            back.color = .Some(.srgb);
            match linear.converted[Rgba8](options: back) {
                .Ok(again) => {
                    var same = true;
                    for v in 0..<256 { if again(v, 0) != gray(v, 0) { same = false; } }
                    check("sRGB ↔ linear round trip is exact", same);
                },
                .Err(_) => check("linear → sRGB", false)
            }
        },
        .Err(_) => check("sRGB → linear", false)
    }

    // Bit depth reduction rounds: 33050 / 257 = 128.6 becomes 129.
    let deep = Image[Rgba16](width: 1, height: 1, filling: Rgba16(r: 33050, g: 32896, b: 65535, a: 0));
    match deep.converted[Rgba8](options: ConversionOptions()) {
        .Ok(small) => check("16 → 8 bit rounds", small(0, 0).r == 129 and small(0, 0).g == 128 and small(0, 0).b == 255),
        .Err(_) => check("16 → 8 bit", false)
    }

    // Half floats.
    check("half 1.0", floatToHalf(1.0) == 0x3C00 and halfToFloat(0x3C00) == 1.0);
    check("half -2.5", halfToFloat(floatToHalf(-2.5)) == -2.5);
    check("half subnormal", halfToFloat(1) > 0.0 and halfToFloat(1) < 0.0000001);
    check("half overflow is infinity", floatToHalf(100000.0) == 0x7C00);

    // Gray from color uses Rec. 709 weights.
    let green = Image[Rgb8](width: 1, height: 1, filling: Rgb8(r: 0, g: 255, b: 0));
    match green.converted[Gray8](options: ConversionOptions()) {
        .Ok(g) => check("green to gray", g(0, 0).value == 182),
        .Err(_) => check("green to gray", false)
    }

    // Premultiplied alpha.
    var premultiplied = ConversionOptions();
    premultiplied.alpha = .Some(.premultiplied);
    let half = Image[Rgba8](width: 1, height: 1, filling: Rgba8(r: 200, g: 100, b: 0, a: 128));
    match half.converted[Rgba8](options: premultiplied) {
        .Ok(p) => check("premultiply", p(0, 0).r == 100 and p(0, 0).g == 50 and p(0, 0).a == 128 and p.alpha == .premultiplied),
        .Err(_) => check("premultiply", false)
    }

    // Display P3 red is outside sRGB; it clips to pure red.
    var p3 = Image[Rgb8](width: 1, height: 1, filling: Rgb8(r: 255, g: 0, b: 0));
    p3.color = .displayP3;
    var toSrgb = ConversionOptions();
    toSrgb.color = .Some(.srgb);
    match p3.converted[Rgb8](options: toSrgb) {
        .Ok(s) => check("P3 red to sRGB", s(0, 0).r == 255 and s(0, 0).g == 0 and s(0, 0).b == 0),
        .Err(_) => check("P3 red to sRGB", false)
    }
    // And sRGB red sits inside P3.
    var srgbRed = Image[RgbaF32](width: 1, height: 1, filling: RgbaF32(r: 1.0, g: 0.0, b: 0.0, a: 1.0));
    srgbRed.color = .srgb;
    var toP3 = ConversionOptions();
    toP3.color = .Some(.displayP3);
    match srgbRed.converted[RgbaF32](options: toP3) {
        .Ok(c) => check("sRGB red in P3", near(c(0, 0).r, 0.9175, 0.002) and near(c(0, 0).g, 0.2003, 0.002) and near(c(0, 0).b, 0.1386, 0.002)),
        .Err(_) => check("sRGB red in P3", false)
    }
}

/// A one-row PQ image with the given luminances in nits.
func pqRow(nits: Array[Float32]) -> Image[RgbaF32] {
    var image = Image[RgbaF32](width: nits.count, height: 1);
    image.color = .bt2100Pq;
    for i in 0..<nits.count {
        let v = pqSignal(nits(i));
        image(i, 0) = RgbaF32(r: v, g: v, b: v, a: 1.0);
    }
    image
}

/// The PQ curve, written out independently of the package.
func pqSignal(nits: Float32) -> Float32 {
    let y = (nits / 10000.0).pow(0.1593017578125);
    ((0.8359375 + 18.8515625 * y) / (1.0 + 18.6875 * y)).pow(78.84375)
}

func testHdr() {
    let ramp = pqRow([0.0, 10.0, 100.0, 203.0, 500.0, 1000.0, 4000.0]);
    match ramp.converted[Rgba8](options: ConversionOptions()) {
        .Ok(sdr) => {
            check("HDR to 8-bit becomes sRGB", sdr.color == ColorInfo.srgb);
            check("black stays black", sdr(0, 0).r == 0);
            var rising = true;
            for i in 1..<6 { if sdr(i, 0).r <= sdr(i - 1, 0).r { rising = false; } }
            check("tone curve rises", rising);
            check("1000-nit peak maps to white", sdr(5, 0).r >= 254);
            check("brighter than peak clips", sdr(6, 0).r == sdr(5, 0).r);
            check("shadows pass through", sdr(1, 0).r > 60 and sdr(1, 0).r < 70);
        },
        .Err(e) => check("HDR to 8-bit: \(e.description)", false)
    }
    var strict = ConversionOptions();
    strict.toneMap = false;
    match ramp.converted[Rgba8](options: strict) {
        .Ok(_) => check("tone mapping off fails", false),
        .Err(e) => check("tone mapping off fails", e.kind == .needsToneMapping)
    }
    // Float targets keep HDR as linear light above 1.0.
    match ramp.converted[RgbaF32](options: ConversionOptions()) {
        .Ok(linear) => {
            check("HDR to float is linear", linear.color.isLinear and linear.color.primaries == 9);
            check("203 nits is 1.0", near(linear(3, 0).r, 1.0, 0.01));
            check("1000 nits is about 4.9", near(linear(5, 0).r, 1000.0 / 203.0, 0.05));
        },
        .Err(_) => check("HDR to float", false)
    }
    // HLG round trips through linear.
    var hlg = Image[RgbaF32](width: 3, height: 1);
    hlg.color = .bt2100Hlg;
    hlg(0, 0) = RgbaF32(r: 0.1, g: 0.2, b: 0.3, a: 1.0);
    hlg(1, 0) = RgbaF32(r: 0.5, g: 0.5, b: 0.5, a: 1.0);
    hlg(2, 0) = RgbaF32(r: 0.9, g: 0.7, b: 0.6, a: 1.0);
    match hlg.converted[RgbaF32](options: ConversionOptions()) {
        .Ok(linear) => {
            var back = ConversionOptions();
            back.color = .Some(.bt2100Hlg);
            match linear.converted[RgbaF32](options: back) {
                .Ok(again) => check("HLG round trip", near(again(0, 0).g, 0.2, 0.001) and near(again(2, 0).r, 0.9, 0.001)),
                .Err(_) => check("HLG back", false)
            }
        },
        .Err(_) => check("HLG to linear", false)
    }
    // cICP survives PNG.
    var pq16 = patternRaw(.rgba16, 4, 4);
    pq16.color = .bt2100Pq;
    match pq16.encoded(format: "png") {
        .Ok(bytes) => match RawImage.decode(data: bytes) {
            .Ok(back) => check("PNG keeps cICP", back.color == ColorInfo.bt2100Pq and sameBytes(pq16, back)),
            .Err(_) => check("PNG with cICP decodes", false)
        },
        .Err(_) => check("PNG with cICP encodes", false)
    }
}

/// Little-endian TIFF bytes with one orientation entry.
func exifWithOrientation(value: Int64) -> Array[UInt8] {
    [73, 73, 42, 0, 8, 0, 0, 0, 1, 0, 0x12, 0x01, 3, 0, 1, 0, 0, 0, UInt8(from: value), 0, 0, 0, 0, 0, 0, 0]
}

func testMetadata() {
    var image = Image[Rgb8](width: 3, height: 2);
    image(2, 0) = Rgb8(r: 255, g: 0, b: 0);
    image.metadata.text.append(("Title", "Kestrel"));
    image.metadata.text.append(("Comment", "naïve"));
    image.metadata.dotsPerMeter = .Some((3780, 3780));
    image.metadata.exif = .Some(exifWithOrientation(6));
    let icc: [UInt8] = [1, 2, 3, 4, 5, 6, 7, 8];
    image.metadata.iccProfile = .Some(icc);
    image.metadata.unknownChunks.append(("teSt", [9, 9, 9]));
    match image.encoded(format: "png") {
        .Ok(bytes) => {
            match RawImage.decode(data: bytes) {
                .Ok(raw) => {
                    let m = raw.metadata;
                    check("text survives", m.text.count == 2 and m.text(0).1 == "Kestrel" and m.text(1).1 == "naïve");
                    var dpm: Int64 = 0;
                    if let .Some(pair) = m.dotsPerMeter { dpm = pair.0 + pair.1; }
                    check("pHYs survives", dpm == 7560);
                    check("ICC survives", m.iccProfile == .Some(icc));
                    check("unknown chunk survives", m.unknownChunks.count == 1 and m.unknownChunks(0).0 == "teSt");
                    check("EXIF orientation is read", raw.orientation.exifValue == 6);
                    check("raw keeps stored size", raw.width == 3 and raw.height == 2);
                },
                .Err(_) => check("metadata png decodes", false)
            }
            // The typed layer turns it upright: 6 means rotate 90° clockwise.
            match Image[Rgb8].decode(data: bytes) {
                .Ok(upright) => check("orientation applied", upright.width == 2 and upright.height == 3 and upright(1, 2).r == 255),
                .Err(_) => check("orientation decode", false)
            }
            var keepSideways = DecodeOptions();
            keepSideways.applyOrientation = false;
            match Image[Rgb8].decode(data: bytes, options: keepSideways) {
                .Ok(sideways) => check("orientation can be left alone", sideways.width == 3),
                .Err(_) => check("orientation off decode", false)
            }
        },
        .Err(_) => check("metadata png encodes", false)
    }
    var bigEndian: [UInt8] = [77, 77, 0, 42, 0, 0, 0, 8, 0, 1, 0x01, 0x12, 0, 3, 0, 0, 0, 1, 0, 3, 0, 0];
    check("big-endian EXIF", exifOrientation(bigEndian).exifValue == 3);
    let junk: [UInt8] = [1, 2, 3];
    check("junk EXIF is normal", exifOrientation(junk) == .normal);
}

func testOps() {
    var image = Image[Rgba8](width: 3, height: 2, filling: Rgba8(r: 0, g: 0, b: 0, a: 255));
    image(0, 0) = Rgba8(r: 1, g: 0, b: 0, a: 255);
    image(2, 1) = Rgba8(r: 2, g: 0, b: 0, a: 255);
    let r90 = image.rotated90();
    check("rotate 90 size", r90.width == 2 and r90.height == 3);
    check("rotate 90 moves top-left to top-right", r90(1, 0).r == 1 and r90(0, 2).r == 2);
    let r270 = image.rotated270();
    check("rotate 270 moves top-left to bottom-left", r270(0, 2).r == 1);
    let r180 = image.rotated180();
    check("rotate 180", r180(2, 1).r == 1 and r180(0, 0).r == 2);
    let full = image.rotated90().rotated90().rotated90().rotated90();
    check("four quarter turns", full(0, 0).r == 1 and full(2, 1).r == 2 and full.width == 3);
    check("flip horizontal", image.flippedHorizontally()(2, 0).r == 1);
    check("flip vertical", image.flippedVertically()(0, 1).r == 1);
    let crop = image.cropped(rect: Rect(x: 1, y: 1, width: 5, height: 5));
    check("crop clips", crop.width == 2 and crop.height == 1 and crop(1, 0).r == 2);
    let view = image.view(rect: Rect(x: 2, y: 1, width: 1, height: 1));
    check("view reads through", view(0, 0).r == 2);

    var canvas = Image[Rgba8](width: 4, height: 4, filling: Rgba8(r: 0, g: 0, b: 255, a: 255));
    canvas.fill(rect: Rect(x: 1, y: 1, width: 2, height: 2), with: Rgba8(r: 255, g: 0, b: 0, a: 255));
    check("fill rect", canvas(1, 1).r == 255 and canvas(0, 0).r == 0 and canvas(3, 3).b == 255);

    // Resizing a solid image keeps its color with every filter.
    let solid = Image[Rgba8](width: 10, height: 7, filling: Rgba8(r: 10, g: 128, b: 250, a: 255));
    for filter in [ResizeFilter.nearest, ResizeFilter.bilinear, ResizeFilter.lanczos3] {
        let small = solid.resized(width: 3, height: 4, filter: filter);
        let large = solid.resized(width: 23, height: 15, filter: filter);
        check("\(filter.name) keeps a solid color", small(1, 1) == solid(0, 0) and large(20, 14) == solid(0, 0) and small.width == 3 and large.height == 15);
    }
    // Averaging black and white happens in linear light: 50% gray is 188, not 128.
    var stripes = Image[Rgba8](width: 2, height: 1);
    stripes(0, 0) = Rgba8(r: 0, g: 0, b: 0, a: 255);
    stripes(1, 0) = Rgba8(r: 255, g: 255, b: 255, a: 255);
    let averaged = stripes.resized(width: 1, height: 1, filter: .bilinear);
    check("resize averages in linear light", averaged(0, 0).r == 188);
    // A transparent neighbor doesn't darken the edge.
    var edge = Image[Rgba8](width: 2, height: 1);
    edge(0, 0) = Rgba8(r: 255, g: 0, b: 0, a: 255);
    edge(1, 0) = Rgba8(r: 0, g: 0, b: 0, a: 0);
    let soft = edge.resized(width: 1, height: 1, filter: .bilinear);
    check("resize respects alpha", soft(0, 0).r == 255 and soft(0, 0).a == 128);
    let pixelArt = stripes.resized(width: 4, height: 2, filter: .nearest);
    check("nearest upscale", pixelArt(1, 1).r == 0 and pixelArt(2, 0).r == 255);

    // Half-transparent white over black is linear-light 50%: 188.
    var base = Image[Rgba8](width: 2, height: 2, filling: Rgba8(r: 0, g: 0, b: 0, a: 255));
    let overlay = Image[Rgba8](width: 1, height: 1, filling: Rgba8(r: 255, g: 255, b: 255, a: 128));
    base.draw(overlay, x: 1, y: 1);
    check("blend in linear light", base(1, 1).r == 188 and base(1, 1).a == 255 and base(0, 0).r == 0);
    base.draw(overlay, x: 5, y: 5);
    check("drawing off the edge is clipped", base(1, 1).r == 188);
}

func testErrorsAndRegistry() {
    let good = patternRaw(.rgba8, 8, 8);
    guard let .Ok(png) = good.encoded(format: "png") else {
        check("encode for error tests", false);
        return;
    }
    var cut: [UInt8] = [];
    for i in 0..<(png.count - 20) { cut.append(png(i)); }
    check("truncated PNG", errorKind(RawImage.decode(data: cut)) == "truncated");
    var badCrc = png;
    badCrc(30) = badCrc(30) ^ 1;
    check("bad CRC", errorKind(RawImage.decode(data: badCrc)) == "corrupt");
    let junk: [UInt8] = [1, 2, 3, 4, 5, 6, 7, 8, 9];
    check("unknown format", errorKind(RawImage.decode(data: junk)) == "unknownFormat");
    var small = DecodeOptions();
    small.maxPixels = 63;
    check("maxPixels", errorKind(RawImage.decode(data: png, options: small)) == "limitExceeded");
    guard let .Ok(qoi) = good.encoded(format: "qoi") else { return; }
    var qoiCut: [UInt8] = [];
    for i in 0..<(qoi.count - 3) { qoiCut.append(qoi(i)); }
    check("truncated QOI", errorKind(RawImage.decode(data: qoiCut)) == "truncated");
    guard let .Ok(bmp) = good.encoded(format: "bmp") else { return; }
    var bmpCut: [UInt8] = [];
    for i in 0..<(bmp.count - 3) { bmpCut.append(bmp(i)); }
    check("truncated BMP", errorKind(RawImage.decode(data: bmpCut)) == "truncated");

    let formats = Formats.standard;
    check("detect PNG", formats.detect(png).isSome);
    check("format by extension", formats.forPath("a/b/photo.QOI").isSome);
    check("no format for .txt", formats.forPath("notes.txt").isNone);
    var custom = Formats.standard;
    custom.add(TestFormat());
    let header: [UInt8] = [84, 69, 83, 84];
    match RawImage.decode(data: header, formats: custom) {
        .Ok(raw) => check("added format decodes", raw.width == 1 and raw.planes(0).bytes(0) == 255),
        .Err(_) => check("added format decodes", false)
    }
    check("standard registry doesn't know it", errorKind(RawImage.decode(data: header)) == "unknownFormat");

    // A sink can skip images and still finish.
    var decoder = PngFormat().makeDecoder(DecodeOptions());
    var skipper: any ImageSink = CollectingSink(only: 5);
    let status = decoder.write(png, into: skipper);
    let finished = decoder.finish(into: skipper);
    check("skipping sink", status.isOk and finished.isOk);
}

@main
func main() {
    testRoundTrips();
    testConversion();
    testHdr();
    testMetadata();
    testOps();
    testErrorsAndRegistry();
}
