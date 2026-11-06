module images

// Image operations: crop, flip, rotate, resize, blend and fill.
//
// Geometry works on whole pixels and never changes values. Resize and
// blend work in linear light with premultiplied alpha, so edges don't get
// dark fringes and gradients stay even.

extend RawImage {
    /// The image turned upright per `orientation`, which is then `.normal`.
    public func oriented() -> RawImage {
        if self.orientation == .normal { return self; }
        var out = self.transformed(self.orientation);
        out.orientation = .normal;
        out
    }

    /// Moves pixels as EXIF orientation `orientation` says to display them:
    /// 3 rotates 180°, 6 rotates 90° clockwise, 8 rotates 90° counter-
    /// clockwise, and 2, 4, 5, 7 add mirrors. Interleaved images only.
    func transformed(orientation: Orientation) -> RawImage {
        let o = orientation.exifValue;
        if o == 1 or not self.isInterleaved { return self; }
        let source = self.planes(0);
        let bpp = source.layout.bytesPerPixel;
        let w = self.width;
        let h = self.height;
        var outWidth = w;
        var outHeight = h;
        if orientation.swapsDimensions {
            outWidth = h;
            outHeight = w;
        }
        var target = Plane(width: outWidth, height: outHeight, layout: source.layout);
        target.bytes.ensureUnique();
        let src = source.bytes.asUnsafeSlice().pointer;
        let dst = target.bytes.asUnsafeSlice().pointer;
        for y in 0..<outHeight {
            for x in 0..<outWidth {
                var sx = x;
                var sy = y;
                if o == 2 { sx = w - 1 - x; }
                if o == 3 { sx = w - 1 - x; sy = h - 1 - y; }
                if o == 4 { sy = h - 1 - y; }
                if o == 5 { sx = y; sy = x; }
                if o == 6 { sx = y; sy = h - 1 - x; }
                if o == 7 { sx = w - 1 - y; sy = h - 1 - x; }
                if o == 8 { sx = w - 1 - y; sy = x; }
                let from = src.offset(by: sy * source.strideBytes + sx * bpp);
                let to = dst.offset(by: (y * outWidth + x) * bpp);
                for k in 0..<bpp { to.offset(by: k).write(from.offset(by: k).read()); }
            }
        }
        var out = RawImage(width: outWidth, height: outHeight, planes: [target], color: self.color, alpha: self.alpha);
        out.orientation = self.orientation;
        out.metadata = self.metadata;
        out.frame = self.frame;
        out
    }
}

/// How `resized` samples the source.
public struct ResizeFilter: Equatable {
    public let name: String;
    /// How far the filter reaches, in source pixels at 1:1.
    let support: Float32;

    init(name name: String, support support: Float32) {
        self.name = name;
        self.support = support;
    }

    /// The nearest source pixel; no blending. For pixel art.
    public static var nearest: ResizeFilter { ResizeFilter(name: "nearest", support: 0.0) }
    /// Linear interpolation between the two nearest pixels.
    public static var bilinear: ResizeFilter { ResizeFilter(name: "bilinear", support: 1.0) }
    /// Windowed sinc over three lobes: sharp, with little aliasing.
    public static var lanczos3: ResizeFilter { ResizeFilter(name: "lanczos3", support: 3.0) }

    public func isEqual(to other: ResizeFilter) -> Bool { self.name == other.name }

    func weight(x: Float32) -> Float32 {
        let d = x.abs();
        if self.name == "bilinear" {
            if d >= 1.0 { return 0.0; }
            return 1.0 - d;
        }
        if d >= 3.0 { return 0.0; }
        if d < 0.000001 { return 1.0; }
        let pi: Float32 = 3.14159265358979;
        let px = pi * d;
        3.0 * px.sin() * (px / 3.0).sin() / (px * px)
    }
}

/// For each output position along one axis, the source pixels it reads
/// and their weights, which sum to 1.
struct ResampleTable {
    var starts: Array[Int64];
    var counts: Array[Int64];
    var weights: Array[Float32];
    /// Weights per output position (the widest any position needs).
    var span: Int64;

    init(source source: Int64, target target: Int64, filter filter: ResizeFilter) {
        let scale = Float32(from: source) / Float32(from: target);
        // Downscaling widens the filter so every source pixel contributes.
        var filterScale: Float32 = 1.0;
        if scale > 1.0 { filterScale = scale; }
        let radius = filter.support * filterScale;
        self.span = (radius * 2.0).ceil().toInt64() ?? 1;
        self.span = self.span + 1;
        self.starts = [];
        self.counts = [];
        self.weights = Array[Float32](repeating: 0.0, count: target * self.span);
        for i in 0..<target {
            // Pixel centers sit at +0.5.
            let center = (Float32(from: i) + 0.5) * scale;
            var first = (center - radius).floor().toInt64() ?? 0;
            var last = (center + radius).ceil().toInt64() ?? 0;
            if first < 0 { first = 0; }
            if last > source { last = source; }
            if last - first > self.span { last = first + self.span; }
            var total: Float32 = 0.0;
            for j in first..<last {
                let w = filter.weight((Float32(from: j) + 0.5 - center) / filterScale);
                self.weights(i * self.span + j - first) = w;
                total = total + w;
            }
            if total != 0.0 {
                for j in 0..<(last - first) {
                    self.weights(i * self.span + j) = self.weights(i * self.span + j) / total;
                }
            }
            self.starts.append(first);
            self.counts.append(last - first);
        }
    }
}

extend Image[P] {
    /// A copy of the pixels inside `rect`, clipped to the image.
    public func cropped(rect rect: Rect) -> Image[P] { self.view(rect: rect).toImage() }

    /// Mirrored left to right.
    public func flippedHorizontally() -> Image[P] { self.transformed(Orientation(exifValue: 2)) }
    /// Mirrored top to bottom.
    public func flippedVertically() -> Image[P] { self.transformed(Orientation(exifValue: 4)) }
    /// Turned 90° clockwise.
    public func rotated90() -> Image[P] { self.transformed(Orientation(exifValue: 6)) }
    public func rotated180() -> Image[P] { self.transformed(Orientation(exifValue: 3)) }
    /// Turned 90° counter-clockwise.
    public func rotated270() -> Image[P] { self.transformed(Orientation(exifValue: 8)) }

    func transformed(orientation: Orientation) -> Image[P] {
        Image[P](raw: self.toRaw().transformed(orientation))
    }

    /// Sets the pixels inside `rect` to `value`.
    public mutating func fill(rect rect: Rect, with value: P) {
        let r = clampRect(rect, self.width, self.height);
        self.bytes.ensureUnique();
        let base = self.bytes.asUnsafeSlice().pointer.cast[P]();
        for y in r.y..<(r.y + r.height) {
            let row = base.offset(by: y * self.stride);
            for x in r.x..<(r.x + r.width) { row.offset(by: x).write(value); }
        }
    }

    /// Scaled to `width` × `height`. The default filter is Lanczos3 when
    /// shrinking and bilinear when enlarging.
    ///
    /// # Examples
    ///
    /// ```
    /// let thumb = photo.resized(width: 256, height: 192);
    /// let pixelArt = sprite.resized(width: 64, height: 64, filter: .nearest);
    /// ```
    public func resized(width width: Int64, height height: Int64) -> Image[P] {
        var filter = ResizeFilter.bilinear;
        if width < self.width or height < self.height { filter = .lanczos3; }
        self.resized(width: width, height: height, filter: filter)
    }

    public func resized(width width: Int64, height height: Int64, filter filter: ResizeFilter) -> Image[P] {
        if filter == .nearest { return self.resizedNearest(width, height); }
        // To linear, premultiplied float; filter; and back.
        let linear = self.toLinearPremultiplied();
        let horizontal = ResampleTable(source: self.width, target: width, filter: filter);
        let vertical = ResampleTable(source: self.height, target: height, filter: filter);
        // Horizontal pass: self.height rows of `width` pixels.
        var middle = Array[Float32](repeating: 0.0, count: width * self.height * 4);
        for y in 0..<self.height {
            for x in 0..<width {
                let start = horizontal.starts(x);
                var r: Float32 = 0.0;
                var g: Float32 = 0.0;
                var b: Float32 = 0.0;
                var a: Float32 = 0.0;
                for k in 0..<horizontal.counts(x) {
                    let w = horizontal.weights(x * horizontal.span + k);
                    let at = (y * self.width + start + k) * 4;
                    r = r + linear(at) * w;
                    g = g + linear(at + 1) * w;
                    b = b + linear(at + 2) * w;
                    a = a + linear(at + 3) * w;
                }
                let o = (y * width + x) * 4;
                middle(o) = r;
                middle(o + 1) = g;
                middle(o + 2) = b;
                middle(o + 3) = a;
            }
        }
        // Vertical pass.
        var result = RawImage(width: width, height: height, layout: .rgbaF32);
        var plane = result.planes(0);
        plane.bytes.ensureUnique();
        let out = plane.bytes.asUnsafeSlice().pointer.cast[RgbaF32]();
        for y in 0..<height {
            let start = vertical.starts(y);
            for x in 0..<width {
                var r: Float32 = 0.0;
                var g: Float32 = 0.0;
                var b: Float32 = 0.0;
                var a: Float32 = 0.0;
                for k in 0..<vertical.counts(y) {
                    let w = vertical.weights(y * vertical.span + k);
                    let at = ((start + k) * width + x) * 4;
                    r = r + middle(at) * w;
                    g = g + middle(at + 1) * w;
                    b = b + middle(at + 2) * w;
                    a = a + middle(at + 3) * w;
                }
                // Lanczos rings below zero; clamp alpha into range and keep
                // color within it.
                a = a.clamp(0.0, 1.0);
                out.offset(by: y * width + x).write(RgbaF32(r: r.clamp(0.0, a), g: g.clamp(0.0, a), b: b.clamp(0.0, a), a: a));
            }
        }
        result.planes(0) = plane;
        self.fromLinearPremultiplied(result)
    }

    func resizedNearest(width: Int64, height: Int64) -> Image[P] {
        var out = Image[P](width: width, height: height);
        out.color = self.color;
        out.alpha = self.alpha;
        out.metadata = self.metadata;
        for y in 0..<height {
            let sy = (y * self.height + self.height / 2) / height;
            for x in 0..<width {
                let sx = (x * self.width + self.width / 2) / width;
                out(x, y) = self(sx, sy);
            }
        }
        out
    }

    /// Every pixel as linear-light, premultiplied RGBA floats.
    func toLinearPremultiplied() -> Array[Float32] {
        let converter = ColorConverter(source: self.color, target: linearOf(self.color), sourceAlpha: self.alpha, targetAlpha: .premultiplied, toneMapping: false);
        var out = Array[Float32](repeating: 0.0, count: self.width * self.height * 4);
        for y in 0..<self.height {
            for x in 0..<self.width {
                let c = converter.convert(self(x, y).toRgbaF32());
                let o = (y * self.width + x) * 4;
                out(o) = c.r;
                out(o + 1) = c.g;
                out(o + 2) = c.b;
                out(o + 3) = c.a;
            }
        }
        out
    }

    /// The inverse of `toLinearPremultiplied`, giving this image's encoding.
    func fromLinearPremultiplied(raw: RawImage) -> Image[P] {
        let converter = ColorConverter(source: linearOf(self.color), target: self.color, sourceAlpha: .premultiplied, targetAlpha: self.alpha, toneMapping: false);
        var out = Image[P](width: raw.width, height: raw.height);
        out.color = self.color;
        out.alpha = self.alpha;
        out.metadata = self.metadata;
        let src = raw.planes(0).bytes.asUnsafeSlice().pointer.cast[RgbaF32]();
        for y in 0..<raw.height {
            for x in 0..<raw.width {
                out(x, y) = P.fromRgbaF32(converter.convert(src.offset(by: y * raw.width + x).read()));
            }
        }
        out
    }

    /// Draws `image` with its top-left corner at (`x`, `y`), blending by
    /// its alpha ("source over") in linear light.
    public mutating func draw(image: Image[P], x left: Int64, y top: Int64) {
        let toLinear = ColorConverter(source: self.color, target: linearOf(self.color), sourceAlpha: .straight, targetAlpha: .straight, toneMapping: false);
        let fromLinear = ColorConverter(source: linearOf(self.color), target: self.color, sourceAlpha: .straight, targetAlpha: .straight, toneMapping: false);
        let overLinear = ColorConverter(source: image.color, target: linearOf(self.color), sourceAlpha: image.alpha, targetAlpha: .straight, toneMapping: false);
        let area = clampRect(Rect(x: left, y: top, width: image.width, height: image.height), self.width, self.height);
        for y in area.y..<(area.y + area.height) {
            for x in area.x..<(area.x + area.width) {
                let s = overLinear.convert(image(x - left, y - top).toRgbaF32());
                if s.a <= 0.0 { continue; }
                let d = toLinear.convert(self(x, y).toRgbaF32());
                let a = s.a + d.a * (1.0 - s.a);
                var c = RgbaF32(r: 0.0, g: 0.0, b: 0.0, a: 0.0);
                if a > 0.0 {
                    let k = d.a * (1.0 - s.a);
                    c = RgbaF32(r: (s.r * s.a + d.r * k) / a, g: (s.g * s.a + d.g * k) / a, b: (s.b * s.a + d.b * k) / a, a: a);
                }
                self(x, y) = P.fromRgbaF32(fromLinear.convert(c));
            }
        }
    }
}

/// The linear-light version of a color encoding (same primaries).
func linearOf(color: ColorInfo) -> ColorInfo {
    ColorInfo(primaries: color.primaries, transfer: 8, matrix: 0, fullRange: true)
}
