module deflate

/// Errors from decompressing malformed or truncated data.
public enum DeflateError {
    /// The input ended in the middle of a block or header.
    case UnexpectedEnd

    /// A block header used the reserved block type 3.
    case InvalidBlockType

    /// A stored block's length and its one's-complement check disagree.
    case InvalidStoredLength

    /// A Huffman code table is over-subscribed or incomplete, or the data
    /// used a code the table doesn't define.
    case InvalidHuffmanCode

    /// A back-reference pointed before the start of the output.
    case InvalidDistance

    /// A zlib or gzip header is malformed or names an unsupported method.
    case InvalidHeader

    /// The zlib stream needs a preset dictionary, which isn't supported.
    case UnsupportedDictionary

    /// The trailing CRC-32 or Adler-32 didn't match the decompressed data.
    case ChecksumMismatch

    /// Bytes followed the end of a zlib stream, or a gzip member was
    /// followed by something that isn't another member.
    case TrailingData

    /// A gzip member's recorded size didn't match the decompressed size.
    case LengthMismatch
}
