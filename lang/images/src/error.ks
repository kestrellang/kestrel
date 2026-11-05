module images

/// The broad kind of an `ImageError`.
///
/// A struct with named values rather than an enum, so formats can add
/// kinds without breaking code that matches on them.
public struct ImageErrorKind: Equatable {
    let name: String;

    public init(name name: String) { self.name = name; }

    /// The data ended before the image did.
    public static var truncated: ImageErrorKind { ImageErrorKind(name: "truncated") }
    /// The data is malformed: a bad signature, chunk, checksum or value.
    public static var corrupt: ImageErrorKind { ImageErrorKind(name: "corrupt") }
    /// Valid data using a feature this codec doesn't implement.
    public static var unsupported: ImageErrorKind { ImageErrorKind(name: "unsupported") }
    /// The image is bigger than `DecodeOptions` allows.
    public static var limitExceeded: ImageErrorKind { ImageErrorKind(name: "limitExceeded") }
    /// No registered format recognizes the data or file extension.
    public static var unknownFormat: ImageErrorKind { ImageErrorKind(name: "unknownFormat") }
    /// HDR content was loaded into a standard-range type with tone mapping off.
    public static var needsToneMapping: ImageErrorKind { ImageErrorKind(name: "needsToneMapping") }
    /// The pixel layouts or sizes involved don't match.
    public static var layoutMismatch: ImageErrorKind { ImageErrorKind(name: "layoutMismatch") }
    /// Reading or writing a file failed.
    public static var io: ImageErrorKind { ImageErrorKind(name: "io") }

    public func isEqual(to other: ImageErrorKind) -> Bool { self.name == other.name }
}

/// Why decoding, encoding or converting an image failed.
///
/// # Examples
///
/// ```
/// match Image[Rgba8].load(path: "cat.png") {
///     .Ok(image) => show(image),
///     .Err(error) => {
///         if error.kind == .truncated { retry() }
///         print(error.description)
///     }
/// }
/// ```
public struct ImageError {
    public let kind: ImageErrorKind;
    /// The format involved ("png"), or empty.
    public let format: String;
    public let message: String;

    public init(kind kind: ImageErrorKind, format format: String, message message: String) {
        self.kind = kind;
        self.format = format;
        self.message = message;
    }

    /// "png: corrupt: bad chunk CRC".
    public var description: String {
        if self.format.isEmpty { return "\(self.kind.name): \(self.message)"; }
        "\(self.format): \(self.kind.name): \(self.message)"
    }
}

func corrupt(format: String, message: String) -> ImageError {
    ImageError(kind: .corrupt, format: format, message: message)
}

func unsupported(format: String, message: String) -> ImageError {
    ImageError(kind: .unsupported, format: format, message: message)
}

func truncated(format: String) -> ImageError {
    ImageError(kind: .truncated, format: format, message: "data ended early")
}
