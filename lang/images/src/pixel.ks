module images

/// A typed pixel that `Image[P]` can hold.
///
/// Conversions go through `RgbaF32` in the image's own color encoding:
/// 0...1 for standard range (HDR values may exceed 1), alpha 1 when the
/// pixel has none.
public protocol Pixel: Copyable {
    /// The memory layout, which must match the struct's fields exactly.
    static var layout: PixelLayout { get }
    func toRgbaF32() -> RgbaF32
    static func fromRgbaF32(color: RgbaF32) -> Self
}

// ---------------------------------------------------------------------------
// Sample conversions. 8- and 16-bit values round to nearest, never truncate.
// ---------------------------------------------------------------------------

func unitFrom8(value: UInt8) -> Float32 { Float32(from: Int64(from: value)) / 255.0 }
func unitFrom16(value: UInt16) -> Float32 { Float32(from: Int64(from: value)) / 65535.0 }

func quantize(value: Float32, maximum: Float32) -> Int64 {
    let clamped = value.clamp(0.0, 1.0);
    (clamped * maximum + 0.5).toInt64() ?? 0
}
func to8(value: Float32) -> UInt8 { UInt8(from: quantize(value, 255.0)) }
func to16(value: Float32) -> UInt16 { UInt16(from: quantize(value, 65535.0)) }

/// IEEE half-precision bits to a float.
public func halfToFloat(bits: UInt16) -> Float32 {
    let h = UInt32(from: bits);
    let sign = (h >> 15) << 31;
    let exponent = (h >> 10) & 0x1F;
    let mantissa = h & 0x3FF;
    if exponent == 0 {
        // Zero or subnormal: mantissa × 2^-24.
        let magnitude = Float32(from: Int64(from: mantissa)) / 16777216.0;
        if sign != 0 { return 0.0 - magnitude; }
        return magnitude;
    }
    if exponent == 31 {
        return Float32(bitPattern: sign | 0x7F800000 | (mantissa << 13));
    }
    Float32(bitPattern: sign | ((exponent + 112) << 23) | (mantissa << 13))
}

/// A float to IEEE half-precision bits, rounding to nearest even.
public func floatToHalf(value: Float32) -> UInt16 {
    let f = value.bitPattern;
    let sign = (f >> 16) & 0x8000;
    let exponent = Int64(from: (f >> 23) & 0xFF);
    var mantissa = f & 0x7FFFFF;
    if exponent == 255 {
        // Infinity or NaN.
        var nan: UInt32 = 0;
        if mantissa != 0 { nan = 0x200; }
        return UInt16(from: sign | 0x7C00 | nan);
    }
    let halfExponent = exponent - 112;
    if halfExponent >= 31 { return UInt16(from: sign | 0x7C00); }
    if halfExponent <= 0 {
        if halfExponent < -10 { return UInt16(from: sign); }
        mantissa = mantissa | 0x800000;
        let shift = 14 - halfExponent;
        var half = mantissa >> shift;
        let remainder = mantissa & ((UInt32(from: 1) << shift) - 1);
        let midpoint = UInt32(from: 1) << (shift - 1);
        if remainder > midpoint or (remainder == midpoint and (half & 1) == 1) { half = half + 1; }
        return UInt16(from: sign | half);
    }
    var half = (UInt32(from: halfExponent) << 10) | (mantissa >> 13);
    let remainder = mantissa & 0x1FFF;
    if remainder > 0x1000 or (remainder == 0x1000 and (half & 1) == 1) { half = half + 1; }
    UInt16(from: sign | half)
}

// ---------------------------------------------------------------------------
// Pixel types
// ---------------------------------------------------------------------------

/// 8-bit grayscale.
public struct Gray8: Pixel {
    public var value: UInt8;
    public init(value value: UInt8) { self.value = value; }
    public static var layout: PixelLayout { .gray8 }
    public func toRgbaF32() -> RgbaF32 {
        let v = unitFrom8(self.value);
        RgbaF32(r: v, g: v, b: v, a: 1.0)
    }
    public static func fromRgbaF32(color: RgbaF32) -> Gray8 { Gray8(value: to8(color.luminance)) }
}

/// 8-bit grayscale with alpha.
public struct GrayAlpha8: Pixel {
    public var value: UInt8;
    public var alpha: UInt8;
    public init(value value: UInt8, alpha alpha: UInt8) {
        self.value = value;
        self.alpha = alpha;
    }
    public static var layout: PixelLayout { .grayAlpha8 }
    public func toRgbaF32() -> RgbaF32 {
        let v = unitFrom8(self.value);
        RgbaF32(r: v, g: v, b: v, a: unitFrom8(self.alpha))
    }
    public static func fromRgbaF32(color: RgbaF32) -> GrayAlpha8 { GrayAlpha8(value: to8(color.luminance), alpha: to8(color.a)) }
}

/// 16-bit grayscale.
public struct Gray16: Pixel {
    public var value: UInt16;
    public init(value value: UInt16) { self.value = value; }
    public static var layout: PixelLayout { .gray16 }
    public func toRgbaF32() -> RgbaF32 {
        let v = unitFrom16(self.value);
        RgbaF32(r: v, g: v, b: v, a: 1.0)
    }
    public static func fromRgbaF32(color: RgbaF32) -> Gray16 { Gray16(value: to16(color.luminance)) }
}

/// 8-bit RGB.
public struct Rgb8: Pixel {
    public var r: UInt8;
    public var g: UInt8;
    public var b: UInt8;
    public init(r r: UInt8, g g: UInt8, b b: UInt8) {
        self.r = r;
        self.g = g;
        self.b = b;
    }
    public static var layout: PixelLayout { .rgb8 }
    public func toRgbaF32() -> RgbaF32 { RgbaF32(r: unitFrom8(self.r), g: unitFrom8(self.g), b: unitFrom8(self.b), a: 1.0) }
    public static func fromRgbaF32(color: RgbaF32) -> Rgb8 { Rgb8(r: to8(color.r), g: to8(color.g), b: to8(color.b)) }
}

/// 8-bit RGBA, the layout most code wants.
public struct Rgba8: Pixel, Equatable {
    public var r: UInt8;
    public var g: UInt8;
    public var b: UInt8;
    public var a: UInt8;
    public init(r r: UInt8, g g: UInt8, b b: UInt8, a a: UInt8) {
        self.r = r;
        self.g = g;
        self.b = b;
        self.a = a;
    }
    public static var layout: PixelLayout { .rgba8 }
    public func toRgbaF32() -> RgbaF32 { RgbaF32(r: unitFrom8(self.r), g: unitFrom8(self.g), b: unitFrom8(self.b), a: unitFrom8(self.a)) }
    public static func fromRgbaF32(color: RgbaF32) -> Rgba8 { Rgba8(r: to8(color.r), g: to8(color.g), b: to8(color.b), a: to8(color.a)) }
    public func isEqual(to other: Rgba8) -> Bool {
        self.r == other.r and self.g == other.g and self.b == other.b and self.a == other.a
    }
}

/// 8-bit BGRA in memory order, as SDL, Windows and many GPUs use.
public struct Bgra8: Pixel {
    public var b: UInt8;
    public var g: UInt8;
    public var r: UInt8;
    public var a: UInt8;
    public init(b b: UInt8, g g: UInt8, r r: UInt8, a a: UInt8) {
        self.b = b;
        self.g = g;
        self.r = r;
        self.a = a;
    }
    public static var layout: PixelLayout { .bgra8 }
    public func toRgbaF32() -> RgbaF32 { RgbaF32(r: unitFrom8(self.r), g: unitFrom8(self.g), b: unitFrom8(self.b), a: unitFrom8(self.a)) }
    public static func fromRgbaF32(color: RgbaF32) -> Bgra8 { Bgra8(b: to8(color.b), g: to8(color.g), r: to8(color.r), a: to8(color.a)) }
}

/// 16-bit RGBA.
public struct Rgba16: Pixel {
    public var r: UInt16;
    public var g: UInt16;
    public var b: UInt16;
    public var a: UInt16;
    public init(r r: UInt16, g g: UInt16, b b: UInt16, a a: UInt16) {
        self.r = r;
        self.g = g;
        self.b = b;
        self.a = a;
    }
    public static var layout: PixelLayout { .rgba16 }
    public func toRgbaF32() -> RgbaF32 { RgbaF32(r: unitFrom16(self.r), g: unitFrom16(self.g), b: unitFrom16(self.b), a: unitFrom16(self.a)) }
    public static func fromRgbaF32(color: RgbaF32) -> Rgba16 { Rgba16(r: to16(color.r), g: to16(color.g), b: to16(color.b), a: to16(color.a)) }
}

/// Half-float RGBA, stored as `UInt16` bit patterns since Kestrel has no
/// `Float16`. Use `halfToFloat` / `floatToHalf` to read and write channels.
public struct RgbaF16: Pixel {
    public var r: UInt16;
    public var g: UInt16;
    public var b: UInt16;
    public var a: UInt16;
    public init(r r: UInt16, g g: UInt16, b b: UInt16, a a: UInt16) {
        self.r = r;
        self.g = g;
        self.b = b;
        self.a = a;
    }
    public static var layout: PixelLayout { .rgbaF16 }
    public func toRgbaF32() -> RgbaF32 { RgbaF32(r: halfToFloat(self.r), g: halfToFloat(self.g), b: halfToFloat(self.b), a: halfToFloat(self.a)) }
    public static func fromRgbaF32(color: RgbaF32) -> RgbaF16 {
        RgbaF16(r: floatToHalf(color.r), g: floatToHalf(color.g), b: floatToHalf(color.b), a: floatToHalf(color.a))
    }
}

/// Float RGBA: the conversion hub, and the type for HDR and linear-light work.
public struct RgbaF32: Pixel {
    public var r: Float32;
    public var g: Float32;
    public var b: Float32;
    public var a: Float32;
    public init(r r: Float32, g g: Float32, b b: Float32, a a: Float32) {
        self.r = r;
        self.g = g;
        self.b = b;
        self.a = a;
    }
    public static var layout: PixelLayout { .rgbaF32 }
    public func toRgbaF32() -> RgbaF32 { self }
    public static func fromRgbaF32(color: RgbaF32) -> RgbaF32 { color }

    /// Rec. 709 luma weights, used when converting to grayscale.
    public var luminance: Float32 { 0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b }
}
