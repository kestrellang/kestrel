module deflate

/// Decompresses a stream fed in chunks of any size.
///
/// Each `write` returns whatever output the new input completed, so a
/// large file never has to sit in memory at once.
///
/// # Examples
///
/// ```
/// var decoder = GzipDecoder();
/// for chunk in chunks {
///     let output = try decoder.write(chunk);
///     sink.append(contentsOf: output);
/// }
/// try decoder.finish();
/// ```
public protocol Decompressor {
    /// Feeds the next chunk of compressed input and returns the newly
    /// decompressed bytes. After an error, every later call fails too.
    mutating func write[S](input: S) -> Result[Array[UInt8], DeflateError] where S: Slice[UInt8], S: not Static

    /// Says no more input is coming. Fails with `.UnexpectedEnd` if the
    /// stream stopped partway through.
    mutating func finish() -> Result[(), DeflateError]

    /// True once the end of the compressed stream has been read.
    var isFinished: Bool { get }
}

/// Compresses a stream fed in chunks of any size.
///
/// # Examples
///
/// ```
/// var encoder = GzipEncoder(level: .best);
/// var output = encoder.write(part1);
/// output.append(contentsOf: encoder.write(part2));
/// output.append(contentsOf: encoder.finish());
/// ```
public protocol Compressor {
    /// Feeds more input and returns any compressed bytes that are ready.
    /// Output is held back until there's enough input to fill a block.
    mutating func write[S](input: S) -> Array[UInt8] where S: Slice[UInt8], S: not Static

    /// Compresses everything still buffered and ends the stream.
    mutating func finish() -> Array[UInt8]
}

/// How hard the compressor searches for repeated data.
///
/// Higher levels find longer matches and produce smaller output, at the
/// cost of speed. The levels match zlib's 0 to 9.
public struct CompressionLevel: Equatable {
    /// 0 to 9.
    public let value: Int64;

    /// Clamps `value` into 0...9.
    public init(value: Int64) {
        var v = value;
        if v < 0 { v = 0; }
        if v > 9 { v = 9; }
        self.value = v;
    }

    /// No compression: stored blocks only.
    public static var none: CompressionLevel { CompressionLevel(0) }
    /// Fastest compression.
    public static var fastest: CompressionLevel { CompressionLevel(1) }
    /// zlib's default trade-off between size and speed.
    public static var standard: CompressionLevel { CompressionLevel(6) }
    /// Smallest output.
    public static var best: CompressionLevel { CompressionLevel(9) }

    public func isEqual(to other: CompressionLevel) -> Bool { self.value == other.value }
}

/// Copies `bytes(from..)` into a new array.
func copyTail(bytes: Array[UInt8], from start: Int64) -> Array[UInt8] {
    var result: [UInt8] = [];
    var from = start;
    if from < 0 { from = 0; }
    if from < bytes.count {
        result.append(contentsOf: bytes(from..));
    }
    result
}
