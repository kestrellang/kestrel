module images

import std.files.(Path, File)

// The one-call helpers: decode or load a whole file, encode or save an
// image. Everything here is built on `ImageFormat` and `CollectingSink`.

extend RawImage {
    /// Decodes the first image in `data`, exactly as stored (orientation
    /// recorded, not applied).
    ///
    /// # Examples
    ///
    /// ```
    /// let raw = try RawImage.decode(data: bytes);
    /// print(raw.layout.description);
    /// ```
    public static func decode(data data: Array[UInt8], options options: DecodeOptions = DecodeOptions(), formats formats: Formats = Formats.standard) -> Result[RawImage, ImageError] {
        let images = try decodeImages(data, options, formats, true);
        .Ok(images(0))
    }

    /// Decodes every image in `data`: pages, sizes or frames.
    public static func decodeAll(data data: Array[UInt8], options options: DecodeOptions = DecodeOptions(), formats formats: Formats = Formats.standard) -> Result[Array[RawImage], ImageError] {
        decodeImages(data, options, formats, false)
    }

    public static func load(path path: String, options options: DecodeOptions = DecodeOptions(), formats formats: Formats = Formats.standard) -> Result[RawImage, ImageError] {
        let data = try readImageFile(path);
        RawImage.decode(data: data, options: options, formats: formats)
    }

    public static func loadAll(path path: String, options options: DecodeOptions = DecodeOptions(), formats formats: Formats = Formats.standard) -> Result[Array[RawImage], ImageError] {
        let data = try readImageFile(path);
        RawImage.decodeAll(data: data, options: options, formats: formats)
    }

    /// Encodes in the named format ("png"), converting first if the format
    /// can't store this layout.
    public func encoded(format name: String, options options: EncodeOptions = EncodeOptions(), formats formats: Formats = Formats.standard) -> Result[Array[UInt8], ImageError] {
        guard let .Some(format) = formats.named(name) else {
            return .Err(ImageError(kind: .unknownFormat, format: name, message: "no format named \(name)"));
        }
        encodeWith(format, self, options)
    }

    /// Saves in the format matching the path's extension.
    public func save(path path: String, options options: EncodeOptions = EncodeOptions(), formats formats: Formats = Formats.standard) -> Result[(), ImageError] {
        guard let .Some(format) = formats.forPath(path) else {
            return .Err(ImageError(kind: .unknownFormat, format: "", message: "no format for \(path)"));
        }
        let data = try encodeWith(format, self, options);
        writeImageFile(path, data)
    }
}

extend Image[P] {
    /// Decodes the first image in `data`, turned upright and converted to
    /// `P`. HDR images are tone mapped into integer pixel types.
    ///
    /// # Examples
    ///
    /// ```
    /// let image = try Image[Rgba8].decode(data: bytes);
    /// ```
    public static func decode(data data: Array[UInt8], options options: DecodeOptions = DecodeOptions(), formats formats: Formats = Formats.standard) -> Result[Image[P], ImageError] {
        var raw = try RawImage.decode(data: data, options: options, formats: formats);
        if options.applyOrientation { raw = raw.oriented(); }
        raw.converted[P](options: ConversionOptions())
    }

    /// Loads the first image in a file.
    ///
    /// # Examples
    ///
    /// ```
    /// let cat = try Image[Rgba8].load(path: "cat.png");
    /// ```
    public static func load(path path: String, options options: DecodeOptions = DecodeOptions(), formats formats: Formats = Formats.standard) -> Result[Image[P], ImageError] {
        let data = try readImageFile(path);
        Image[P].decode(data: data, options: options, formats: formats)
    }

    public func encoded(format name: String, options options: EncodeOptions = EncodeOptions(), formats formats: Formats = Formats.standard) -> Result[Array[UInt8], ImageError] {
        self.toRaw().encoded(format: name, options: options, formats: formats)
    }

    public func save(path path: String, options options: EncodeOptions = EncodeOptions(), formats formats: Formats = Formats.standard) -> Result[(), ImageError] {
        self.toRaw().save(path: path, options: options, formats: formats)
    }
}

/// Runs a whole file through the decoder its magic bytes select.
func decodeImages(data: Array[UInt8], options: DecodeOptions, formats: Formats, firstOnly: Bool) -> Result[Array[RawImage], ImageError] {
    guard let .Some(format) = formats.detect(data) else {
        return .Err(ImageError(kind: .unknownFormat, format: "", message: "no format recognizes this data"));
    }
    var decoder = format.makeDecoder(options);
    var sink: any ImageSink = CollectingSink();
    if firstOnly { sink = CollectingSink(only: 0); }
    let _ = try decoder.write(data, into: sink);
    try decoder.finish(into: sink);
    guard let .Some(collected) = sink as? CollectingSink else {
        return .Err(corrupt(format.name, "lost the decoded images"));
    }
    if collected.images.count == 0 { return .Err(corrupt(format.name, "the file holds no images")); }
    .Ok(collected.images)
}

func encodeWith(format: any ImageFormat, image: RawImage, options: EncodeOptions) -> Result[Array[UInt8], ImageError] {
    if image.isInterleaved and format.canEncode(image.layout) {
        return format.encode(image, options);
    }
    let layout = format.preferredLayout(for: image.layout);
    let converted = try image.converted(layout: layout, options: ConversionOptions());
    format.encode(converted, options)
}

func readImageFile(path: String) -> Result[Array[UInt8], ImageError] {
    match File.readBytes(Path(path)) {
        .Ok(data) => .Ok(data),
        .Err(e) => .Err(ImageError(kind: .io, format: "", message: "\(path): \(e.description())"))
    }
}

func writeImageFile(path: String, data: Array[UInt8]) -> Result[(), ImageError] {
    match File.writeBytes(Path(path), data.asSlice()) {
        .Ok(_) => .Ok(()),
        .Err(e) => .Err(ImageError(kind: .io, format: "", message: "\(path): \(e.description())"))
    }
}
