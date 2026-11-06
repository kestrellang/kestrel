module images

// The one shared conversion path: any layout and color encoding to any
// other. Pixels go through `RgbaF32`; color goes through linear light,
// with 1.0 meaning SDR reference white (203 nits, per ITU-R BT.2408).

/// Nits of SDR reference white: linear 1.0 in this package.
let referenceWhiteNits: Float32 = 203.0;
/// Peak assumed for PQ content without mastering metadata, and HLG's
/// nominal display peak.
let assumedHdrPeakNits: Float32 = 1000.0;

/// How to convert between layouts and color encodings.
///
/// # Examples
///
/// ```
/// var options = ConversionOptions();
/// options.color = .Some(.linearSrgb);
/// let linear = try raw.converted[RgbaF32](options: options);
/// ```
public struct ConversionOptions {
    /// The color encoding to produce. `.None` keeps the source's, except
    /// that HDR sources become sRGB (tone mapped) in integer layouts and
    /// linear light in float layouts.
    public var color: ColorInfo?;
    /// The alpha mode to produce. `.None` keeps the source's.
    public var alpha: AlphaMode?;
    /// Map HDR into standard range with BT.2390 when the target can't hold
    /// it. When false that conversion fails with `.needsToneMapping`.
    public var toneMap: Bool;

    public init() {
        self.color = .None;
        self.alpha = .None;
        self.toneMap = true;
    }

    /// The color encoding these options give for a source and layout.
    func targetColor(source: ColorInfo, layout: PixelLayout) -> ColorInfo {
        if let .Some(color) = self.color { return color; }
        if not source.isHighDynamicRange { return source; }
        if layout.sample.isFloat {
            return ColorInfo(primaries: source.primaries, transfer: 8, matrix: 0, fullRange: true);
        }
        .srgb
    }

    func targetAlpha(source: AlphaMode, layout: PixelLayout) -> AlphaMode {
        if not layout.channels.hasAlpha { return .none; }
        if let .Some(alpha) = self.alpha { return alpha; }
        if source == .none { return .straight; }
        source
    }
}

// ---------------------------------------------------------------------------
// Transfer curves. `toLinear` and `fromLinear` are exact inverses.
// ---------------------------------------------------------------------------

func srgbToLinear(v: Float32) -> Float32 {
    if v <= 0.04045 { return v / 12.92; }
    ((v + 0.055) / 1.055).pow(2.4)
}

func linearToSrgb(v: Float32) -> Float32 {
    if v <= 0.0031308 { return v * 12.92; }
    1.055 * v.pow(1.0 / 2.4) - 0.055
}

let pqM1: Float32 = 0.1593017578125;
let pqM2: Float32 = 78.84375;
let pqC1: Float32 = 0.8359375;
let pqC2: Float32 = 18.8515625;
let pqC3: Float32 = 18.6875;

/// PQ signal (0...1) to absolute luminance as a fraction of 10,000 nits.
func pqToNits(v: Float32) -> Float32 {
    let p = v.clamp(0.0, 1.0).pow(1.0 / pqM2);
    var numerator = p - pqC1;
    if numerator < 0.0 { numerator = 0.0; }
    10000.0 * (numerator / (pqC2 - pqC3 * p)).pow(1.0 / pqM1)
}

func nitsToPq(nits: Float32) -> Float32 {
    var y = nits / 10000.0;
    if y < 0.0 { y = 0.0; }
    let p = y.pow(pqM1);
    ((pqC1 + pqC2 * p) / (1.0 + pqC3 * p)).pow(pqM2)
}

let hlgA: Float32 = 0.17883277;
let hlgB: Float32 = 0.28466892;
let hlgC: Float32 = 0.55991073;

func hlgToScene(v: Float32) -> Float32 {
    if v <= 0.5 { return v * v / 3.0; }
    (((v - hlgC) / hlgA).exp() + hlgB) / 12.0
}

func sceneToHlg(e: Float32) -> Float32 {
    var x = e;
    if x < 0.0 { x = 0.0; }
    if x <= 1.0 / 12.0 { return (3.0 * x).sqrt(); }
    hlgA * (12.0 * x - hlgB).ln() + hlgC
}

/// HLG's system gamma for a 1000-nit display (BT.2100).
let hlgGamma: Float32 = 1.2;

/// One channel's transfer curve, chosen by CICP transfer code.
struct Curve {
    let code: Int64;

    init(code code: Int64) {
        var c = code;
        // 1, 6, 14 and 15 are BT.709-family camera curves; show them with
        // the BT.1886 display curve (gamma 2.4). Unknown codes act as sRGB.
        if c == 6 or c == 14 or c == 15 { c = 1; }
        if c != 1 and c != 4 and c != 5 and c != 8 and c != 16 and c != 18 { c = 13; }
        self.code = c;
    }

    var isLinear: Bool { self.code == 8 }

    /// Signal to linear light, for codes that work per channel (not HLG).
    func toLinear(v: Float32) -> Float32 {
        if self.code == 13 { return srgbToLinear(v); }
        if self.code == 8 { return v; }
        if self.code == 16 { return pqToNits(v) / referenceWhiteNits; }
        if v <= 0.0 { return 0.0; }
        if self.code == 1 { return v.pow(2.4); }
        if self.code == 4 { return v.pow(2.2); }
        v.pow(2.8)
    }

    func fromLinear(v: Float32) -> Float32 {
        if self.code == 13 { return linearToSrgb(v); }
        if self.code == 8 { return v; }
        if self.code == 16 { return nitsToPq(v * referenceWhiteNits); }
        if v <= 0.0 { return 0.0; }
        if self.code == 1 { return v.pow(1.0 / 2.4); }
        if self.code == 4 { return v.pow(1.0 / 2.2); }
        v.pow(1.0 / 2.8)
    }
}

// ---------------------------------------------------------------------------
// Primaries
// ---------------------------------------------------------------------------

/// Chromaticities (xr, yr, xg, yg, xb, yb) for a CICP primaries code; all
/// use the D65 white point. Unknown codes act as BT.709.
func chromaticities(code: Int64) -> Array[Float64] {
    if code == 9 { return [0.708, 0.292, 0.170, 0.797, 0.131, 0.046]; }
    if code == 11 or code == 12 { return [0.680, 0.320, 0.265, 0.690, 0.150, 0.060]; }
    [0.640, 0.330, 0.300, 0.600, 0.150, 0.060]
}

/// Normalizes a primaries code so equivalent ones compare equal.
func primariesKey(code: Int64) -> Int64 {
    if code == 9 or code == 12 { return code; }
    if code == 11 { return 12; }
    1
}

func invert3(m: Array[Float64]) -> Array[Float64] {
    let a = m(0); let b = m(1); let c = m(2);
    let d = m(3); let e = m(4); let f = m(5);
    let g = m(6); let h = m(7); let i = m(8);
    let co0 = e * i - f * h;
    let co1 = f * g - d * i;
    let co2 = d * h - e * g;
    let det = a * co0 + b * co1 + c * co2;
    [
        co0 / det, (c * h - b * i) / det, (b * f - c * e) / det,
        co1 / det, (a * i - c * g) / det, (c * d - a * f) / det,
        co2 / det, (b * g - a * h) / det, (a * e - b * d) / det
    ]
}

func multiply3(a: Array[Float64], b: Array[Float64]) -> Array[Float64] {
    var out: [Float64] = [];
    for row in 0..<3 {
        for col in 0..<3 {
            out.append(a(row * 3) * b(col) + a(row * 3 + 1) * b(3 + col) + a(row * 3 + 2) * b(6 + col));
        }
    }
    out
}

/// The RGB → XYZ matrix for a set of primaries with a D65 white.
func rgbToXyz(code: Int64) -> Array[Float64] {
    let c = chromaticities(code);
    let whiteX: Float64 = 0.3127;
    let whiteY: Float64 = 0.3290;
    // Columns are each primary's XYZ at Y = 1, scaled so RGB white maps to D65.
    let p = [
        c(0) / c(1), c(2) / c(3), c(4) / c(5),
        1.0, 1.0, 1.0,
        (1.0 - c(0) - c(1)) / c(1), (1.0 - c(2) - c(3)) / c(3), (1.0 - c(4) - c(5)) / c(5)
    ];
    let wx = whiteX / whiteY;
    let wz = (1.0 - whiteX - whiteY) / whiteY;
    let inv = invert3(p);
    let s0 = inv(0) * wx + inv(1) + inv(2) * wz;
    let s1 = inv(3) * wx + inv(4) + inv(5) * wz;
    let s2 = inv(6) * wx + inv(7) + inv(8) * wz;
    [
        p(0) * s0, p(1) * s1, p(2) * s2,
        p(3) * s0, p(4) * s1, p(5) * s2,
        p(6) * s0, p(7) * s1, p(8) * s2
    ]
}

// ---------------------------------------------------------------------------
// BT.2390 tone mapping
// ---------------------------------------------------------------------------

/// The BT.2390 EETF: compresses luminance from `sourcePeak` nits down to
/// `targetPeak` nits with a soft knee, working in the PQ domain.
struct ToneMapper {
    let sourcePq: Float32;
    let maxLum: Float32;
    let kneeStart: Float32;

    init(sourcePeak sourcePeak: Float32, targetPeak targetPeak: Float32) {
        self.sourcePq = nitsToPq(sourcePeak);
        self.maxLum = nitsToPq(targetPeak) / self.sourcePq;
        self.kneeStart = 1.5 * self.maxLum - 0.5;
    }

    func map(nits: Float32) -> Float32 {
        var e1 = nitsToPq(nits) / self.sourcePq;
        if e1 > 1.0 { e1 = 1.0; }
        var e2 = e1;
        if e1 >= self.kneeStart {
            let t = (e1 - self.kneeStart) / (1.0 - self.kneeStart);
            let t2 = t * t;
            let t3 = t2 * t;
            e2 = (2.0 * t3 - 3.0 * t2 + 1.0) * self.kneeStart + (t3 - 2.0 * t2 + t) * (1.0 - self.kneeStart) + (0.0 - 2.0 * t3 + 3.0 * t2) * self.maxLum;
        }
        pqToNits(e2 * self.sourcePq)
    }
}

// ---------------------------------------------------------------------------
// The converter
// ---------------------------------------------------------------------------

/// Converts colors from one `ColorInfo` and `AlphaMode` to another.
///
/// Built once per conversion; `convert` runs per pixel.
public struct ColorConverter {
    let sourceCurve: Curve;
    let targetCurve: Curve;
    /// Nothing to do to color: same encoding.
    let passThrough: Bool;
    let useMatrix: Bool;
    let matrix: Array[Float32];
    let toneMapping: Bool;
    let mapper: ToneMapper;
    let unpremultiply: Bool;
    let premultiply: Bool;

    /// Fails with `.needsToneMapping` when HDR must go into standard range
    /// and `toneMap` is false.
    public static func make(from source: ColorInfo, to target: ColorInfo, sourceAlpha sourceAlpha: AlphaMode, targetAlpha targetAlpha: AlphaMode, toneMap toneMap: Bool) -> Result[ColorConverter, ImageError] {
        let mustMap = source.isHighDynamicRange and not target.isHighDynamicRange and not target.isLinear;
        if mustMap and not toneMap {
            return .Err(ImageError(kind: .needsToneMapping, format: "", message: "HDR source needs tone mapping to fit the target"));
        }
        .Ok(ColorConverter(source: source, target: target, sourceAlpha: sourceAlpha, targetAlpha: targetAlpha, toneMapping: mustMap))
    }

    init(source source: ColorInfo, target target: ColorInfo, sourceAlpha sourceAlpha: AlphaMode, targetAlpha targetAlpha: AlphaMode, toneMapping toneMapping: Bool) {
        self.sourceCurve = Curve(code: source.transfer);
        self.targetCurve = Curve(code: target.transfer);
        let sameCurve = self.sourceCurve.code == self.targetCurve.code;
        let samePrimaries = primariesKey(source.primaries) == primariesKey(target.primaries);
        self.passThrough = sameCurve and samePrimaries;
        self.useMatrix = not samePrimaries;
        var m: [Float32] = [];
        if self.useMatrix {
            let full = multiply3(invert3(rgbToXyz(target.primaries)), rgbToXyz(source.primaries));
            for v in full { m.append(Float32(from: v)); }
        }
        self.matrix = m;
        self.toneMapping = toneMapping;
        var peak = assumedHdrPeakNits;
        if source.transfer == 16 { peak = assumedHdrPeakNits; }
        self.mapper = ToneMapper(sourcePeak: peak, targetPeak: referenceWhiteNits);
        self.unpremultiply = sourceAlpha == .premultiplied and targetAlpha != .premultiplied;
        self.premultiply = targetAlpha == .premultiplied and sourceAlpha != .premultiplied;
    }

    /// Signal values in the source encoding to linear light.
    func linearize(c: RgbaF32) -> RgbaF32 {
        if self.sourceCurve.code != 18 {
            return RgbaF32(r: self.sourceCurve.toLinear(c.r), g: self.sourceCurve.toLinear(c.g), b: self.sourceCurve.toLinear(c.b), a: c.a);
        }
        // HLG: inverse OETF to scene light, then the OOTF for a 1000-nit display.
        let r = hlgToScene(c.r);
        let g = hlgToScene(c.g);
        let b = hlgToScene(c.b);
        let ys = 0.2627 * r + 0.6780 * g + 0.0593 * b;
        var gain: Float32 = 0.0;
        if ys > 0.0 { gain = ys.pow(hlgGamma - 1.0) * assumedHdrPeakNits / referenceWhiteNits; }
        RgbaF32(r: r * gain, g: g * gain, b: b * gain, a: c.a)
    }

    /// Linear light to signal values in the target encoding.
    func encode(c: RgbaF32) -> RgbaF32 {
        if self.targetCurve.code != 18 {
            return RgbaF32(r: self.targetCurve.fromLinear(c.r), g: self.targetCurve.fromLinear(c.g), b: self.targetCurve.fromLinear(c.b), a: c.a);
        }
        // Inverse of `linearize`'s HLG path.
        let scale = referenceWhiteNits / assumedHdrPeakNits;
        let yd = (0.2627 * c.r + 0.6780 * c.g + 0.0593 * c.b) * scale;
        var gain: Float32 = 0.0;
        if yd > 0.0 { gain = scale * yd.pow((1.0 - hlgGamma) / hlgGamma); }
        RgbaF32(r: sceneToHlg(c.r * gain), g: sceneToHlg(c.g * gain), b: sceneToHlg(c.b * gain), a: c.a)
    }

    /// Scales the color so its brightest channel follows the tone curve,
    /// which keeps hue.
    func toneMapColor(c: RgbaF32) -> RgbaF32 {
        var peak = c.r;
        if c.g > peak { peak = c.g; }
        if c.b > peak { peak = c.b; }
        if peak <= 0.0 { return c; }
        let nits = peak * referenceWhiteNits;
        let scale = self.mapper.map(nits) / nits;
        RgbaF32(r: c.r * scale, g: c.g * scale, b: c.b * scale, a: c.a)
    }

    public func convert(color: RgbaF32) -> RgbaF32 {
        var c = color;
        if self.unpremultiply and c.a > 0.0 {
            c = RgbaF32(r: c.r / c.a, g: c.g / c.a, b: c.b / c.a, a: c.a);
        }
        if not self.passThrough {
            c = self.linearize(c);
            if self.useMatrix {
                let m = self.matrix;
                c = RgbaF32(
                    r: m(0) * c.r + m(1) * c.g + m(2) * c.b,
                    g: m(3) * c.r + m(4) * c.g + m(5) * c.b,
                    b: m(6) * c.r + m(7) * c.g + m(8) * c.b,
                    a: c.a);
            }
            if self.toneMapping { c = self.toneMapColor(c); }
            c = self.encode(c);
        }
        if self.premultiply {
            c = RgbaF32(r: c.r * c.a, g: c.g * c.a, b: c.b * c.a, a: c.a);
        }
        c
    }
}

// ---------------------------------------------------------------------------
// Reading and writing pixels of any layout at runtime. Multi-byte samples
// are little-endian, which is native on every target Kestrel supports.
// ---------------------------------------------------------------------------

/// Where each of R, G, B, A sits in a pixel, in channels; -1 for none.
struct ChannelMap {
    let r: Int64;
    let g: Int64;
    let b: Int64;
    let a: Int64;
    let isGray: Bool;
    let isCmyk: Bool;

    init(order order: ChannelOrder) {
        let name = order.name;
        self.isGray = name == "gray" or name == "grayAlpha";
        self.isCmyk = name == "cmyk";
        if name == "gray" { self.r = 0; self.g = 0; self.b = 0; self.a = -1; return; }
        if name == "grayAlpha" { self.r = 0; self.g = 0; self.b = 0; self.a = 1; return; }
        if name == "rgb" { self.r = 0; self.g = 1; self.b = 2; self.a = -1; return; }
        if name == "bgra" { self.r = 2; self.g = 1; self.b = 0; self.a = 3; return; }
        if name == "argb" { self.r = 1; self.g = 2; self.b = 3; self.a = 0; return; }
        self.r = 0; self.g = 1; self.b = 2; self.a = 3;
    }
}

/// Reads and writes one pixel layout through `RgbaF32`.
struct PixelCodec {
    let layout: PixelLayout;
    let map: ChannelMap;
    let sampleBytes: Int64;
    let isFloat: Bool;

    init(layout layout: PixelLayout) {
        self.layout = layout;
        self.map = ChannelMap(order: layout.channels);
        self.sampleBytes = layout.sample.bytes;
        self.isFloat = layout.sample.isFloat;
    }

    func readSample(p: Pointer[UInt8], channel: Int64) -> Float32 {
        let at = p.offset(by: channel * self.sampleBytes);
        if self.sampleBytes == 1 { return unitFrom8(at.read()); }
        if self.sampleBytes == 2 {
            let bits = (UInt16(from: at.read()) | (UInt16(from: at.offset(by: 1).read()) << 8));
            if self.isFloat { return halfToFloat(bits); }
            return unitFrom16(bits);
        }
        let bits = (UInt32(from: at.read()) | (UInt32(from: at.offset(by: 1).read()) << 8) | (UInt32(from: at.offset(by: 2).read()) << 16) | (UInt32(from: at.offset(by: 3).read()) << 24));
        Float32(bitPattern: bits)
    }

    func writeSample(p: Pointer[UInt8], channel: Int64, value: Float32) {
        let at = p.offset(by: channel * self.sampleBytes);
        if self.sampleBytes == 1 { at.write(to8(value)); return; }
        if self.sampleBytes == 2 {
            var bits = to16(value);
            if self.isFloat { bits = floatToHalf(value); }
            at.write(UInt8(from: bits));
            at.offset(by: 1).write(UInt8(from: bits >> 8));
            return;
        }
        let bits = value.bitPattern;
        at.write(UInt8(from: bits));
        at.offset(by: 1).write(UInt8(from: bits >> 8));
        at.offset(by: 2).write(UInt8(from: bits >> 16));
        at.offset(by: 3).write(UInt8(from: bits >> 24));
    }

    func read(p: Pointer[UInt8]) -> RgbaF32 {
        if self.map.isCmyk {
            // Naive CMYK; real CMYK needs the embedded ICC profile.
            let k = 1.0 - self.readSample(p, 3);
            return RgbaF32(r: (1.0 - self.readSample(p, 0)) * k, g: (1.0 - self.readSample(p, 1)) * k, b: (1.0 - self.readSample(p, 2)) * k, a: 1.0);
        }
        var a: Float32 = 1.0;
        if self.map.a >= 0 { a = self.readSample(p, self.map.a); }
        if self.map.isGray {
            let v = self.readSample(p, 0);
            return RgbaF32(r: v, g: v, b: v, a: a);
        }
        RgbaF32(r: self.readSample(p, self.map.r), g: self.readSample(p, self.map.g), b: self.readSample(p, self.map.b), a: a)
    }

    func write(p: Pointer[UInt8], color: RgbaF32) {
        if self.map.isCmyk {
            var peak = color.r.clamp(0.0, 1.0);
            if color.g > peak { peak = color.g.clamp(0.0, 1.0); }
            if color.b > peak { peak = color.b.clamp(0.0, 1.0); }
            self.writeSample(p, 3, 1.0 - peak);
            if peak <= 0.0 {
                self.writeSample(p, 0, 0.0);
                self.writeSample(p, 1, 0.0);
                self.writeSample(p, 2, 0.0);
                return;
            }
            self.writeSample(p, 0, 1.0 - color.r / peak);
            self.writeSample(p, 1, 1.0 - color.g / peak);
            self.writeSample(p, 2, 1.0 - color.b / peak);
            return;
        }
        if self.map.a >= 0 { self.writeSample(p, self.map.a, color.a); }
        if self.map.isGray {
            self.writeSample(p, 0, color.luminance);
            return;
        }
        self.writeSample(p, self.map.r, color.r);
        self.writeSample(p, self.map.g, color.g);
        self.writeSample(p, self.map.b, color.b);
    }
}

extend RawImage {
    /// Converts to another layout and color encoding: the one conversion
    /// path every other conversion uses.
    ///
    /// Only interleaved images are supported in v1.
    public func converted(layout layout: PixelLayout, options options: ConversionOptions) -> Result[RawImage, ImageError] {
        if not self.isInterleaved {
            return .Err(unsupported("", "converting planar images"));
        }
        let source = self.planes(0);
        let color = options.targetColor(self.color, layout);
        let alpha = options.targetAlpha(self.alpha, layout);
        // Nothing to change: share the pixels.
        if source.layout == layout and color == self.color and alpha == self.alpha {
            return .Ok(self);
        }
        let converter = try ColorConverter.make(from: self.color, to: color, sourceAlpha: self.alpha, targetAlpha: alpha, toneMap: options.toneMap);
        var out = RawImage(width: self.width, height: self.height, layout: layout);
        out.color = color;
        out.alpha = alpha;
        out.orientation = self.orientation;
        out.metadata = self.metadata;
        out.frame = self.frame;
        out.auxiliary = self.auxiliary;
        var target = out.planes(0);
        target.bytes.ensureUnique();
        let reader = PixelCodec(layout: source.layout);
        let writer = PixelCodec(layout: layout);
        let sourceBpp = source.layout.bytesPerPixel;
        let targetBpp = layout.bytesPerPixel;
        let src = source.bytes.asUnsafeSlice().pointer;
        let dst = target.bytes.asUnsafeSlice().pointer;
        for y in 0..<self.height {
            let srcRow = src.offset(by: y * source.strideBytes);
            let dstRow = dst.offset(by: y * target.strideBytes);
            for x in 0..<self.width {
                let c = converter.convert(reader.read(srcRow.offset(by: x * sourceBpp)));
                writer.write(dstRow.offset(by: x * targetBpp), c);
            }
        }
        out.planes(0) = target;
        .Ok(out)
    }

    /// Converts to a typed image.
    ///
    /// # Examples
    ///
    /// ```
    /// let image = try raw.converted[Rgba8](options: ConversionOptions());
    /// ```
    public func converted[P](options options: ConversionOptions) -> Result[Image[P], ImageError] where P: Pixel {
        let raw = try self.converted(layout: P.layout, options: options);
        .Ok(Image[P](raw: raw))
    }
}
