module images

/// A typed image: what almost all code uses.
///
/// Pixels are stored as plain bytes and read through `P`'s layout, so an
/// `Image` converts to and from `RawImage` without copying, and copies
/// share storage until one is written. Orientation is always applied in
/// this layer.
///
/// # Examples
///
/// ```
/// var image = Image[Rgba8](width: 64, height: 64, filling: Rgba8(r: 255, g: 0, b: 0, a: 255));
/// image(10, 20) = Rgba8(r: 0, g: 0, b: 255, a: 255);
/// let png = try image.encoded(format: "png");
/// ```
public struct Image[P] where P: Pixel {
    public let width: Int64;
    public let height: Int64;
    /// Pixels from the start of one row to the next; at least `width`.
    public let stride: Int64;
    /// The pixels, `stride * height * P.layout.bytesPerPixel` bytes.
    public var bytes: Array[UInt8];
    public var color: ColorInfo;
    public var alpha: AlphaMode;
    public var metadata: ImageMetadata;

    /// An image of zero bytes (black, and transparent if `P` has alpha).
    public init(width width: Int64, height height: Int64) {
        self.width = width;
        self.height = height;
        self.stride = width;
        self.bytes = Array[UInt8](repeating: 0, count: width * height * P.layout.bytesPerPixel);
        self.color = defaultColor(P.layout);
        self.alpha = defaultAlpha(P.layout);
        self.metadata = ImageMetadata();
    }

    /// An image with every pixel set to `filling`.
    public init(width width: Int64, height height: Int64, filling value: P) {
        self.width = width;
        self.height = height;
        self.stride = width;
        self.bytes = Array[UInt8](repeating: 0, count: width * height * P.layout.bytesPerPixel);
        self.color = defaultColor(P.layout);
        self.alpha = defaultAlpha(P.layout);
        self.metadata = ImageMetadata();
        self.fill(value);
    }

    /// Wraps a raw image already in `P`'s layout. Rows are repacked only
    /// when the stride isn't a whole number of pixels.
    init(raw raw: RawImage) {
        let plane = raw.planes(0);
        let bpp = P.layout.bytesPerPixel;
        self.width = raw.width;
        self.height = raw.height;
        self.color = raw.color;
        self.alpha = raw.alpha;
        self.metadata = raw.metadata;
        if plane.strideBytes % bpp == 0 {
            self.stride = plane.strideBytes / bpp;
            self.bytes = plane.bytes;
            return;
        }
        self.stride = raw.width;
        var packed = Array[UInt8](repeating: 0, count: raw.width * raw.height * bpp);
        for y in 0..<raw.height {
            copyBytes(from: plane.bytes, offset: y * plane.strideBytes, into: packed, at: y * raw.width * bpp, count: raw.width * bpp);
        }
        self.bytes = packed;
    }

    /// The pixel at column `x`, row `y`, counted from the top left.
    public subscript(x: Int64, y: Int64) -> P {
        get {
            self.checkBounds(x, y);
            unsafe self.bytes.asUnsafeSlice().pointer.cast[P]().offset(by: y * self.stride + x).read()
        }
        set {
            self.checkBounds(x, y);
            self.bytes.ensureUnique();
            unsafe self.bytes.asUnsafeSlice().pointer.cast[P]().offset(by: y * self.stride + x).write(newValue);
        }
    }

    func checkBounds(x: Int64, y: Int64) {
        if x < 0 or y < 0 or x >= self.width or y >= self.height {
            fatalError("pixel (\(x), \(y)) is outside a \(self.width)x\(self.height) image");
        }
    }

    /// Sets every pixel to `value`.
    public mutating func fill(value: P) {
        self.bytes.ensureUnique();
        let base = self.bytes.asUnsafeSlice().pointer.cast[P]();
        for y in 0..<self.height {
            let row = base.offset(by: y * self.stride);
            for x in 0..<self.width { unsafe row.offset(by: x).write(value); }
        }
    }

    /// The same pixels as a `RawImage`, without copying.
    public func toRaw() -> RawImage {
        let layout = P.layout;
        let plane = Plane(width: self.width, height: self.height, layout: layout, strideBytes: self.stride * layout.bytesPerPixel, bytes: self.bytes);
        var raw = RawImage(width: self.width, height: self.height, planes: [plane], color: self.color, alpha: self.alpha);
        raw.metadata = self.metadata;
        raw
    }

    /// Converts to another pixel type, keeping the color encoding unless
    /// `options` say otherwise.
    public func converted[Q](options options: ConversionOptions) -> Result[Image[Q], ImageError] where Q: Pixel {
        self.toRaw().converted[Q](options: options)
    }

    /// A zero-copy view of part of the image.
    public func view(rect rect: Rect) -> ImageView[P] {
        ImageView[P](image: self, rect: clampRect(rect, self.width, self.height))
    }
}

/// A rectangle of an image, sharing its pixels.
public struct ImageView[P] where P: Pixel {
    let image: Image[P];
    public let rect: Rect;

    init(image image: Image[P], rect rect: Rect) {
        self.image = image;
        self.rect = rect;
    }

    public var width: Int64 { self.rect.width }
    public var height: Int64 { self.rect.height }

    public subscript(x: Int64, y: Int64) -> P {
        if x < 0 or y < 0 or x >= self.rect.width or y >= self.rect.height {
            fatalError("pixel (\(x), \(y)) is outside a \(self.rect.width)x\(self.rect.height) view");
        }
        self.image(self.rect.x + x, self.rect.y + y)
    }

    /// Copies the view into its own image.
    public func toImage() -> Image[P] {
        var out = Image[P](width: self.rect.width, height: self.rect.height);
        out.color = self.image.color;
        out.alpha = self.image.alpha;
        out.metadata = self.image.metadata;
        let bpp = P.layout.bytesPerPixel;
        for y in 0..<self.rect.height {
            let from = ((self.rect.y + y) * self.image.stride + self.rect.x) * bpp;
            copyBytes(from: self.image.bytes, offset: from, into: out.bytes, at: y * self.rect.width * bpp, count: self.rect.width * bpp);
        }
        out
    }
}

extend RawImage {
    /// A typed view of the pixels, without copying. `.None` unless the
    /// image is interleaved in exactly `P`'s layout.
    public func view[P]() -> Image[P]? where P: Pixel {
        if not self.isInterleaved or self.layout != P.layout { return .None; }
        .Some(Image[P](raw: self))
    }
}

/// The color a new image of this layout gets: linear for float layouts,
/// sRGB otherwise.
func defaultColor(layout: PixelLayout) -> ColorInfo {
    if layout.sample.isFloat { return .linearSrgb; }
    .srgb
}

func defaultAlpha(layout: PixelLayout) -> AlphaMode {
    if layout.channels.hasAlpha { return .straight; }
    .none
}

/// Clips a rectangle to an image's bounds.
func clampRect(rect: Rect, width: Int64, height: Int64) -> Rect {
    var x0 = rect.x;
    var y0 = rect.y;
    var x1 = rect.x + rect.width;
    var y1 = rect.y + rect.height;
    if x0 < 0 { x0 = 0; }
    if y0 < 0 { y0 = 0; }
    if x1 > width { x1 = width; }
    if y1 > height { y1 = height; }
    if x1 < x0 { x1 = x0; }
    if y1 < y0 { y1 = y0; }
    Rect(x: x0, y: y0, width: x1 - x0, height: y1 - y0)
}
