module images

/// One plane of pixel data. Interleaved images have one plane; YCbCr
/// 4:2:0 has three, with the chroma planes at half size.
public struct Plane {
    /// Width and height in this plane's own samples.
    public let width: Int64;
    public let height: Int64;
    public let layout: PixelLayout;
    /// Bytes from the start of one row to the next; at least
    /// `width * layout.bytesPerPixel`.
    public let strideBytes: Int64;
    public var bytes: Array[UInt8];

    public init(width width: Int64, height height: Int64, layout layout: PixelLayout, strideBytes strideBytes: Int64, bytes bytes: Array[UInt8]) {
        self.width = width;
        self.height = height;
        self.layout = layout;
        self.strideBytes = strideBytes;
        self.bytes = bytes;
    }

    /// A zero-filled plane with tightly packed rows.
    public init(width width: Int64, height height: Int64, layout layout: PixelLayout) {
        self.width = width;
        self.height = height;
        self.layout = layout;
        self.strideBytes = width * layout.bytesPerPixel;
        self.bytes = Array[UInt8](repeating: 0, count: width * layout.bytesPerPixel * height);
    }

    /// Bytes per row that hold pixels (excluding padding).
    public var rowBytes: Int64 { self.width * self.layout.bytesPerPixel }
}

/// The shape of one plane, as a decoder announces it.
public struct PlaneInfo {
    public let width: Int64;
    public let height: Int64;
    public let layout: PixelLayout;

    public init(width width: Int64, height height: Int64, layout layout: PixelLayout) {
        self.width = width;
        self.height = height;
        self.layout = layout;
    }
}

/// Timing and placement of one animation frame.
public struct FrameInfo {
    /// How long to show the frame, in milliseconds.
    public let durationMilliseconds: Int64;
    /// Where the frame sits on the canvas.
    public let region: Rect;
    /// "none", "background" or "previous": what happens to the frame's
    /// region before the next frame.
    public let dispose: String;
    /// "source" (replace) or "over" (alpha-blend onto the canvas).
    public let blend: String;

    public init(durationMilliseconds durationMilliseconds: Int64, region region: Rect, dispose dispose: String, blend blend: String) {
        self.durationMilliseconds = durationMilliseconds;
        self.region = region;
        self.dispose = dispose;
        self.blend = blend;
    }
}

/// Everything about an image besides its pixels. Codecs keep what they
/// don't understand in `unknownChunks` so re-encoding doesn't drop it.
public struct ImageMetadata {
    /// An embedded ICC profile, kept and re-written but not applied.
    public var iccProfile: Array[UInt8]?;
    /// File gamma (PNG gAMA), recorded only.
    public var gamma: Float64?;
    /// Physical resolution, horizontal and vertical.
    public var dotsPerMeter: (Int64, Int64)?;
    /// Key/value text (PNG tEXt, zTXt, iTXt).
    public var text: Array[(String, String)];
    /// Raw EXIF (TIFF-structured) bytes.
    public var exif: Array[UInt8]?;
    /// Raw XMP packet.
    public var xmp: String?;
    /// Chunks the codec didn't interpret: (type, data).
    public var unknownChunks: Array[(String, Array[UInt8])];

    public init() {
        self.iccProfile = .None;
        self.gamma = .None;
        self.dotsPerMeter = .None;
        self.text = [];
        self.exif = .None;
        self.xmp = .None;
        self.unknownChunks = [];
    }
}

/// An image stored alongside the main one: an HDR gain map, a depth map
/// or separate alpha.
public struct AuxiliaryImage {
    public let kind: AuxiliaryKind;
    public var image: RawImage;
    /// Format-specific parameters (gain-map metadata), kept raw.
    public var parameters: Array[UInt8];

    public init(kind kind: AuxiliaryKind, image image: RawImage, parameters parameters: Array[UInt8]) {
        self.kind = kind;
        self.image = image;
        self.parameters = parameters;
    }
}

/// Exactly what a file held: planes of bytes plus a runtime description.
///
/// Most code uses the typed `Image[P]` instead; `RawImage` is for
/// converters, editors and lossless re-encoding.
///
/// # Examples
///
/// ```
/// let raw = try RawImage.load(path: "scan.png");
/// print(raw.layout.description);          // "rgb16"
/// if let .Some(view) = raw.view[Rgb16]() { ... }
/// let image = try raw.converted[Rgba8]();
/// ```
public struct RawImage {
    public let width: Int64;
    public let height: Int64;
    public var planes: Array[Plane];
    public var color: ColorInfo;
    public var alpha: AlphaMode;
    /// The orientation as stored; `.normal` once it has been applied.
    public var orientation: Orientation;
    public var metadata: ImageMetadata;
    public var auxiliary: Array[AuxiliaryImage];
    /// Set for animation frames.
    public var frame: FrameInfo?;

    public init(width width: Int64, height height: Int64, planes planes: Array[Plane], color color: ColorInfo, alpha alpha: AlphaMode) {
        self.width = width;
        self.height = height;
        self.planes = planes;
        self.color = color;
        self.alpha = alpha;
        self.orientation = .normal;
        self.metadata = ImageMetadata();
        self.auxiliary = [];
        self.frame = .None;
    }

    /// A zero-filled single-plane image.
    public init(width width: Int64, height height: Int64, layout layout: PixelLayout) {
        self.width = width;
        self.height = height;
        self.planes = [Plane(width: width, height: height, layout: layout)];
        self.color = .srgb;
        self.alpha = .none;
        if layout.channels.hasAlpha { self.alpha = .straight; }
        self.orientation = .normal;
        self.metadata = ImageMetadata();
        self.auxiliary = [];
        self.frame = .None;
    }

    /// True for one plane holding whole pixels.
    public var isInterleaved: Bool { self.planes.count == 1 }

    /// The first plane's layout: the pixel layout of an interleaved image.
    public var layout: PixelLayout { self.planes(0).layout }
}
