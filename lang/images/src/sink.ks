module images

// The streaming layer: bytes go into an `ImageDecoder`, decoded rows come
// out to an `ImageSink`. Formats plug in through `ImageFormat` and are
// gathered in a `Formats` registry.

/// What a decoder knows about an image before its pixels arrive.
public struct ImageInfo {
    public let width: Int64;
    public let height: Int64;
    /// The layout and size of each plane; one entry for interleaved pixels.
    public let planes: Array[PlaneInfo];
    public let color: ColorInfo;
    public let alpha: AlphaMode;
    /// As stored. Load helpers apply it; the streaming layer never does.
    public let orientation: Orientation;
    /// Set for animation frames.
    public let frame: FrameInfo?;
    /// What was read before the pixels. Chunks after them (PNG text after
    /// IDAT) arrive in `endImage`.
    public let metadata: ImageMetadata;

    public init(width width: Int64, height height: Int64, planes planes: Array[PlaneInfo], color color: ColorInfo, alpha alpha: AlphaMode, orientation orientation: Orientation, frame frame: FrameInfo?, metadata metadata: ImageMetadata) {
        self.width = width;
        self.height = height;
        self.planes = planes;
        self.color = color;
        self.alpha = alpha;
        self.orientation = orientation;
        self.frame = frame;
        self.metadata = metadata;
    }

    /// One interleaved plane covering the whole image.
    public init(width width: Int64, height height: Int64, layout layout: PixelLayout, color color: ColorInfo, alpha alpha: AlphaMode, metadata metadata: ImageMetadata) {
        self.width = width;
        self.height = height;
        self.planes = [PlaneInfo(width: width, height: height, layout: layout)];
        self.color = color;
        self.alpha = alpha;
        self.orientation = .normal;
        self.frame = .None;
        self.metadata = metadata;
    }
}

/// A sink's answer to `beginImage`.
public struct SinkChoice: Equatable {
    let name: String;
    public init(name name: String) { self.name = name; }

    /// Deliver this image's rows.
    public static var decode: SinkChoice { SinkChoice(name: "decode") }
    /// Don't deliver rows for this image; the decoder may skip work.
    public static var skip: SinkChoice { SinkChoice(name: "skip") }

    public func isEqual(to other: SinkChoice) -> Bool { self.name == other.name }
}

/// Receives decoded images row band by row band.
///
/// # Examples
///
/// ```
/// struct RowCounter: ImageSink {
///     var rows: Int64 = 0;
///     mutating func beginImage(index: Int64, info: ImageInfo) -> SinkChoice { .decode }
///     mutating func receive(plane: Int64, region: Rect, data: Array[UInt8], strideBytes: Int64) {
///         self.rows = self.rows + region.height;
///     }
///     mutating func endImage(index: Int64, metadata: ImageMetadata) {}
/// }
/// ```
public protocol ImageSink {
    /// Called once per image in the file (page, size or frame), numbered
    /// from 0 in the order they're found.
    mutating func beginImage(index: Int64, info: ImageInfo) -> SinkChoice
    /// A finished band of rows for one plane. `data` starts at the
    /// region's top-left pixel; rows are `strideBytes` apart.
    mutating func receive(plane: Int64, region: Rect, data: Array[UInt8], strideBytes: Int64)
    /// The image is complete. `metadata` includes anything read after the
    /// pixels.
    mutating func endImage(index: Int64, metadata: ImageMetadata)
}

/// Whether a decoder wants more input.
public struct DecodeStatus: Equatable {
    let name: String;
    public init(name name: String) { self.name = name; }

    /// Keep feeding input.
    public static var needsInput: DecodeStatus { DecodeStatus(name: "needsInput") }
    /// The file is complete; further input is an error.
    public static var done: DecodeStatus { DecodeStatus(name: "done") }

    public func isEqual(to other: DecodeStatus) -> Bool { self.name == other.name }
}

/// Turns a file's bytes, fed in chunks of any size, into rows.
public protocol ImageDecoder {
    mutating func write(input: Array[UInt8], mutating into sink: any ImageSink) -> Result[DecodeStatus, ImageError]
    /// No more input. Fails with `.truncated` if an image was cut off;
    /// rows already delivered stay in the sink.
    mutating func finish(mutating into sink: any ImageSink) -> Result[(), ImageError]
}

/// Limits and choices for decoding.
public struct DecodeOptions {
    /// Largest width × height accepted per image: a guard against
    /// decompression bombs.
    public var maxPixels: Int64;
    /// Largest total of metadata (ICC, EXIF, text) kept per image.
    public var maxMetadataBytes: Int64;
    /// When false, codecs drop text, EXIF and unknown chunks.
    public var keepMetadata: Bool;
    /// Used by the load helpers: rotate and flip per `Orientation`.
    public var applyOrientation: Bool;

    public init() {
        self.maxPixels = 100000000;
        self.maxMetadataBytes = 16000000;
        self.keepMetadata = true;
        self.applyOrientation = true;
    }

    /// Fails if an image of this size is over `maxPixels`. Every decoder
    /// calls this before `beginImage`.
    public func check(format: String, width: Int64, height: Int64) -> Result[(), ImageError] {
        if width <= 0 or height <= 0 {
            return .Err(corrupt(format, "image has zero width or height"));
        }
        if width > self.maxPixels / height {
            return .Err(ImageError(kind: .limitExceeded, format: format, message: "\(width)x\(height) is over the \(self.maxPixels)-pixel limit"));
        }
        .Ok(())
    }
}

/// Choices for encoding. Format-specific settings (PNG compression level)
/// live on the format value instead.
public struct EncodeOptions {
    /// When false, metadata is not written.
    public var keepMetadata: Bool;

    public init() { self.keepMetadata = true; }
}

/// A file format: how to recognize, decode and encode it.
///
/// Third-party packages implement this and are added to a `Formats`
/// registry.
public protocol ImageFormat {
    /// "png".
    var name: String { get }
    /// Lowercase file extensions without the dot: ["png"].
    var extensions: Array[String] { get }
    /// True if the first bytes of a file look like this format.
    func matches(header: Array[UInt8]) -> Bool
    func makeDecoder(options: DecodeOptions) -> any ImageDecoder
    /// True if `encode` can write this layout without converting.
    func canEncode(layout: PixelLayout) -> Bool
    /// Encodes one image. Decode-only formats return `.unsupported`.
    func encode(image: RawImage, options: EncodeOptions) -> Result[Array[UInt8], ImageError]
    /// The layout to convert to when `canEncode` says no.
    func preferredLayout(for layout: PixelLayout) -> PixelLayout
}

/// A set of formats to choose from by name, extension or magic bytes.
///
/// # Examples
///
/// ```
/// var formats = Formats.standard;
/// formats.add(WebpFormat());
/// let image = try Image[Rgba8].load(path: "cat.webp", formats: formats);
/// ```
public struct Formats {
    public var all: Array[any ImageFormat];

    public init(all all: Array[any ImageFormat]) { self.all = all; }

    /// PNG, QOI and BMP.
    public static var standard: Formats {
        let png: any ImageFormat = PngFormat();
        let qoi: any ImageFormat = QoiFormat();
        let bmp: any ImageFormat = BmpFormat();
        Formats(all: [png, qoi, bmp])
    }

    /// Adds a format. Later formats win when two match the same file.
    public mutating func add(format: any ImageFormat) {
        self.all.insert(format, at: 0);
    }

    /// The format whose magic bytes match.
    public func detect(header: Array[UInt8]) -> (any ImageFormat)? {
        for format in self.all {
            if format.matches(header) { return .Some(format); }
        }
        .None
    }

    /// The format with this name ("png").
    public func named(name: String) -> (any ImageFormat)? {
        let lower = asciiLowercase(name);
        for format in self.all {
            if format.name == lower { return .Some(format); }
        }
        .None
    }

    /// The format for a path's extension.
    public func forPath(path: String) -> (any ImageFormat)? {
        let ext = asciiLowercase(extensionOf(path));
        for format in self.all {
            for candidate in format.extensions {
                if candidate == ext { return .Some(format); }
            }
        }
        .None
    }
}

/// Keeps every image a decoder delivers, as `RawImage`s.
///
/// This is what the load helpers use. After a truncated file, the rows
/// that did arrive are in `partialImage()`.
public struct CollectingSink: ImageSink {
    /// The finished images, in file order.
    public var images: Array[RawImage];
    /// Which image indices to keep; empty keeps all.
    let wanted: Array[Int64];
    /// The image being collected, without its pixels.
    var current: RawImage?;
    /// The pixels of each of `current`'s planes. Held outside `current` so
    /// writing a row never copies a plane another value still shares.
    var planeBytes: Array[Array[UInt8]];
    /// The first plane's pixels: every row of interleaved images lands
    /// here, so it gets its own field and is written in place.
    var firstPlane: Array[UInt8];

    public init() {
        self.images = [];
        self.wanted = [];
        self.current = .None;
        self.planeBytes = [];
        self.firstPlane = [];
    }

    /// Keeps only the image at `index`.
    public init(only index: Int64) {
        self.images = [];
        self.wanted = [index];
        self.current = .None;
        self.planeBytes = [];
        self.firstPlane = [];
    }

    public mutating func beginImage(index: Int64, info: ImageInfo) -> SinkChoice {
        self.current = .None;
        if self.wanted.count > 0 and not self.wanted.contains(index) { return .skip; }
        var planes: [Plane] = [];
        for p in info.planes {
            // Empty bytes for now; `finished` fills them in.
            planes.append(Plane(width: p.width, height: p.height, layout: p.layout, strideBytes: p.width * p.layout.bytesPerPixel, bytes: []));
        }
        var image = RawImage(width: info.width, height: info.height, planes: planes, color: info.color, alpha: info.alpha);
        image.orientation = info.orientation;
        image.frame = info.frame;
        image.metadata = info.metadata;
        self.current = .Some(image);
        self.planeBytes = [];
        for p in info.planes {
            self.planeBytes.append(Array[UInt8](repeating: 0, count: p.width * p.height * p.layout.bytesPerPixel));
        }
        self.firstPlane = self.planeBytes(0);
        self.planeBytes(0) = [];
        .decode
    }

    public mutating func receive(plane: Int64, region: Rect, data: Array[UInt8], strideBytes: Int64) {
        guard let .Some(image) = self.current else { return; }
        let info = image.planes(plane);
        let bpp = info.layout.bytesPerPixel;
        let rowBytes = region.width * bpp;
        for row in 0..<region.height {
            let from = row * strideBytes;
            let to = (region.y + row) * info.strideBytes + region.x * bpp;
            if plane == 0 {
                copyBytes(from: data, offset: from, into: self.firstPlane, at: to, count: rowBytes);
            } else {
                var bytes = self.planeBytes(plane);
                self.planeBytes(plane) = [];
                copyBytes(from: data, offset: from, into: bytes, at: to, count: rowBytes);
                self.planeBytes(plane) = bytes;
            }
        }
    }

    public mutating func endImage(index: Int64, metadata: ImageMetadata) {
        guard let .Some(image) = self.partialImage() else { return; }
        var done = image;
        done.metadata = metadata;
        // EXIF can arrive after the pixels (PNG eXIf after IDAT).
        if let .Some(exif) = metadata.exif {
            if done.orientation == .normal { done.orientation = exifOrientation(exif); }
        }
        self.images.append(done);
        self.current = .None;
        self.planeBytes = [];
        self.firstPlane = [];
    }

    /// The image being decoded with the rows delivered so far (the rest
    /// zero), or `.None` between images.
    public func partialImage() -> RawImage? {
        guard let .Some(image) = self.current else { return .None; }
        var out = image;
        var planes: [Plane] = [];
        for i in 0..<image.planes.count {
            let p = image.planes(i);
            var bytes = self.firstPlane;
            if i > 0 { bytes = self.planeBytes(i); }
            planes.append(Plane(width: p.width, height: p.height, layout: p.layout, strideBytes: p.strideBytes, bytes: bytes));
        }
        out.planes = planes;
        .Some(out)
    }
}

/// Copies `count` bytes between arrays.
func copyBytes(from source: Array[UInt8], offset sourceStart: Int64, mutating into target: Array[UInt8], at targetStart: Int64, count count: Int64) {
    if count <= 0 { return; }
    target.ensureUnique();
    let src = source.asUnsafeSlice().pointer.offset(by: sourceStart);
    let dst = target.asUnsafeSlice().pointer.offset(by: targetStart);
    for i in 0..<count {
        unsafe dst.offset(by: i).write(src.offset(by: i).read());
    }
}

func asciiLowercase(text: String) -> String { text.lowercasedAscii() }

/// "photo.PNG" → "PNG"; "" when there is no extension.
func extensionOf(path: String) -> String {
    var dot: Int64 = -1;
    var index: Int64 = 0;
    for byte in path.bytes {
        if byte == 46 { dot = index; }
        if byte == 47 { dot = -1; }
        index = index + 1;
    }
    if dot < 0 { return ""; }
    var bytes: [UInt8] = [];
    index = 0;
    for byte in path.bytes {
        if index > dot { bytes.append(byte); }
        index = index + 1;
    }
    String(fromUtf8Lossy: bytes)
}
