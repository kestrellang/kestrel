module images

// Runtime descriptions of pixel data. Each is a struct with named values
// rather than an enum, so new values (CMYK, new HDR curves) never break a
// `match` in someone else's code.

/// Which channels a pixel has, in memory order.
public struct ChannelOrder: Equatable {
    public let name: String;
    /// Channels per pixel.
    public let count: Int64;

    public init(name name: String, count count: Int64) {
        self.name = name;
        self.count = count;
    }

    public static var gray: ChannelOrder { ChannelOrder(name: "gray", count: 1) }
    public static var grayAlpha: ChannelOrder { ChannelOrder(name: "grayAlpha", count: 2) }
    public static var rgb: ChannelOrder { ChannelOrder(name: "rgb", count: 3) }
    public static var rgba: ChannelOrder { ChannelOrder(name: "rgba", count: 4) }
    public static var bgra: ChannelOrder { ChannelOrder(name: "bgra", count: 4) }
    public static var argb: ChannelOrder { ChannelOrder(name: "argb", count: 4) }
    public static var cmyk: ChannelOrder { ChannelOrder(name: "cmyk", count: 4) }

    /// True when one of the channels is alpha.
    public var hasAlpha: Bool {
        self.name == "grayAlpha" or self.name == "rgba" or self.name == "bgra" or self.name == "argb"
    }

    public func isEqual(to other: ChannelOrder) -> Bool { self.name == other.name }
}

/// How each channel is stored. Multi-byte samples are native-endian in
/// memory; codecs swap when a file format says otherwise.
public struct SampleType: Equatable {
    public let name: String;
    public let bytes: Int64;
    public let isFloat: Bool;

    public init(name name: String, bytes bytes: Int64, isFloat isFloat: Bool) {
        self.name = name;
        self.bytes = bytes;
        self.isFloat = isFloat;
    }

    public static var uint8: SampleType { SampleType(name: "uint8", bytes: 1, isFloat: false) }
    public static var uint16: SampleType { SampleType(name: "uint16", bytes: 2, isFloat: false) }
    /// IEEE half precision, stored as its `UInt16` bit pattern.
    public static var float16: SampleType { SampleType(name: "float16", bytes: 2, isFloat: true) }
    public static var float32: SampleType { SampleType(name: "float32", bytes: 4, isFloat: true) }

    public func isEqual(to other: SampleType) -> Bool { self.name == other.name }
}

/// Channel order plus sample type: everything needed to read one pixel.
public struct PixelLayout: Equatable {
    public let channels: ChannelOrder;
    public let sample: SampleType;

    public init(channels channels: ChannelOrder, sample sample: SampleType) {
        self.channels = channels;
        self.sample = sample;
    }

    public var bytesPerPixel: Int64 { self.channels.count * self.sample.bytes }

    public static var gray8: PixelLayout { PixelLayout(channels: .gray, sample: .uint8) }
    public static var grayAlpha8: PixelLayout { PixelLayout(channels: .grayAlpha, sample: .uint8) }
    public static var rgb8: PixelLayout { PixelLayout(channels: .rgb, sample: .uint8) }
    public static var rgba8: PixelLayout { PixelLayout(channels: .rgba, sample: .uint8) }
    public static var bgra8: PixelLayout { PixelLayout(channels: .bgra, sample: .uint8) }
    public static var gray16: PixelLayout { PixelLayout(channels: .gray, sample: .uint16) }
    public static var grayAlpha16: PixelLayout { PixelLayout(channels: .grayAlpha, sample: .uint16) }
    public static var rgb16: PixelLayout { PixelLayout(channels: .rgb, sample: .uint16) }
    public static var rgba16: PixelLayout { PixelLayout(channels: .rgba, sample: .uint16) }
    public static var rgbaF16: PixelLayout { PixelLayout(channels: .rgba, sample: .float16) }
    public static var rgbaF32: PixelLayout { PixelLayout(channels: .rgba, sample: .float32) }
    public static var cmyk8: PixelLayout { PixelLayout(channels: .cmyk, sample: .uint8) }

    public func isEqual(to other: PixelLayout) -> Bool {
        self.channels == other.channels and self.sample == other.sample
    }

    /// "rgba8", "grayAlpha16", "rgbaF32".
    public var description: String {
        if self.sample.isFloat { return "\(self.channels.name)F\(self.sample.bytes * 8)"; }
        "\(self.channels.name)\(self.sample.bytes * 8)"
    }
}

/// What the numbers in an image mean, as CICP code points (ITU-T H.273):
/// the same scheme PNG's cICP chunk, AVIF, HEIF and video use.
///
/// # Examples
///
/// ```
/// if image.color.isHighDynamicRange { ... }
/// let p3 = ColorInfo.displayP3;
/// ```
public struct ColorInfo: Equatable {
    /// 1 = BT.709 / sRGB, 12 = Display P3, 9 = BT.2020.
    public var primaries: Int64;
    /// 13 = sRGB, 8 = linear, 16 = PQ (SMPTE ST 2084), 18 = HLG.
    public var transfer: Int64;
    /// 0 = RGB, 1 = BT.709 YCbCr, 6 = BT.601, 9 = BT.2020.
    public var matrix: Int64;
    public var fullRange: Bool;

    public init(primaries primaries: Int64, transfer transfer: Int64, matrix matrix: Int64, fullRange fullRange: Bool) {
        self.primaries = primaries;
        self.transfer = transfer;
        self.matrix = matrix;
        self.fullRange = fullRange;
    }

    public static var srgb: ColorInfo { ColorInfo(primaries: 1, transfer: 13, matrix: 0, fullRange: true) }
    public static var linearSrgb: ColorInfo { ColorInfo(primaries: 1, transfer: 8, matrix: 0, fullRange: true) }
    public static var displayP3: ColorInfo { ColorInfo(primaries: 12, transfer: 13, matrix: 0, fullRange: true) }
    /// BT.2100 HDR with the PQ curve.
    public static var bt2100Pq: ColorInfo { ColorInfo(primaries: 9, transfer: 16, matrix: 0, fullRange: true) }
    /// BT.2100 HDR with the HLG curve.
    public static var bt2100Hlg: ColorInfo { ColorInfo(primaries: 9, transfer: 18, matrix: 0, fullRange: true) }

    /// True for the PQ and HLG transfer curves.
    public var isHighDynamicRange: Bool { self.transfer == 16 or self.transfer == 18 }

    /// True when values are proportional to light.
    public var isLinear: Bool { self.transfer == 8 }

    public func isEqual(to other: ColorInfo) -> Bool {
        self.primaries == other.primaries and self.transfer == other.transfer and self.matrix == other.matrix and self.fullRange == other.fullRange
    }
}

/// How color channels relate to alpha.
public struct AlphaMode: Equatable {
    let name: String;
    public init(name name: String) { self.name = name; }

    /// The image has no alpha channel.
    public static var none: AlphaMode { AlphaMode(name: "none") }
    /// Color is independent of alpha (PNG, most files).
    public static var straight: AlphaMode { AlphaMode(name: "straight") }
    /// Color is already multiplied by alpha (compositing, GPUs).
    public static var premultiplied: AlphaMode { AlphaMode(name: "premultiplied") }

    public func isEqual(to other: AlphaMode) -> Bool { self.name == other.name }
}

/// How the stored pixels must be turned to display upright, as the EXIF
/// orientation values 1 to 8.
public struct Orientation: Equatable {
    /// 1 = upright, 3 = rotated 180°, 6 = needs 90° clockwise, 8 = needs
    /// 90° counter-clockwise; 2, 4, 5 and 7 add a mirror.
    public let exifValue: Int64;

    public init(exifValue exifValue: Int64) {
        var value = exifValue;
        if value < 1 or value > 8 { value = 1; }
        self.exifValue = value;
    }

    public static var normal: Orientation { Orientation(exifValue: 1) }

    /// True for the four values that swap width and height.
    public var swapsDimensions: Bool { self.exifValue >= 5 }

    public func isEqual(to other: Orientation) -> Bool { self.exifValue == other.exifValue }
}

/// What an auxiliary image holds.
public struct AuxiliaryKind: Equatable {
    public let name: String;
    public init(name name: String) { self.name = name; }

    /// An HDR gain map (Ultra HDR, ISO 21496-1).
    public static var gainMap: AuxiliaryKind { AuxiliaryKind(name: "gainMap") }
    public static var depth: AuxiliaryKind { AuxiliaryKind(name: "depth") }
    /// Alpha stored as its own image (AVIF, HEIF).
    public static var alpha: AuxiliaryKind { AuxiliaryKind(name: "alpha") }

    public func isEqual(to other: AuxiliaryKind) -> Bool { self.name == other.name }
}

/// A rectangle in pixel coordinates.
public struct Rect: Equatable {
    public let x: Int64;
    public let y: Int64;
    public let width: Int64;
    public let height: Int64;

    public init(x x: Int64, y y: Int64, width width: Int64, height height: Int64) {
        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;
    }

    public func isEqual(to other: Rect) -> Bool {
        self.x == other.x and self.y == other.y and self.width == other.width and self.height == other.height
    }
}
