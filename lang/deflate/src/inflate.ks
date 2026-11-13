module deflate

// Base lengths and extra-bit counts for length symbols 257...285 (RFC 1951 §3.2.5).
// Module-level, so they are built once at startup rather than per block.
let LENGTH_BASES: Array[Int64] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258]
let LENGTH_EXTRA_BITS: Array[Int64] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0]
// Base distances and extra-bit counts for distance symbols 0...29.
let DISTANCE_BASES: Array[Int64] = [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577]
let DISTANCE_EXTRA_BITS: Array[Int64] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13]
// The order code-length code lengths are stored in a dynamic block header.
let CODE_LENGTH_ORDER: Array[Int64] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15]

/// Code lengths of the fixed literal/length code (RFC 1951 §3.2.6).
func fixedLiteralLengths() -> Array[Int64] {
    var lengths = Array[Int64](repeating: 8, count: 288);
    for i in 144..<256 { lengths(i) = 9; }
    for i in 256..<280 { lengths(i) = 7; }
    lengths
}

/// Code lengths of the fixed distance code.
func fixedDistanceLengths() -> Array[Int64] {
    Array[Int64](repeating: 5, count: 30)
}

// The most output a back-reference can reach.
let windowSize: Int64 = 32768

// What the inflater expects next.
let stateBlockHeader: Int64 = 0
let stateStored: Int64 = 1
let stateCompressed: Int64 = 2
let stateDone: Int64 = 3

/// Decompresses raw DEFLATE data (RFC 1951), fed in chunks of any size.
///
/// When input runs out mid-symbol, the inflater backs up to the last whole
/// symbol and waits for more, so chunk boundaries can fall anywhere.
///
/// # Examples
///
/// ```
/// var inflater = Inflater();
/// let output = try inflater.write(compressed);
/// try inflater.finish();
/// ```
public struct Inflater: Decompressor {
    // Input not yet consumed, starting `bitOffset` bits in.
    var input: Array[UInt8];
    var bitOffset: Int64;
    // The last `windowSize` bytes of output, for back-references.
    var window: Array[UInt8];
    var state: Int64;
    var isFinalBlock: Bool;
    var storedRemaining: Int64;
    var literals: HuffmanDecoder?;
    var distances: HuffmanDecoder?;
    var failure: DeflateError?;

    public init() {
        self.input = [];
        self.bitOffset = 0;
        self.window = [];
        self.state = stateBlockHeader;
        self.isFinalBlock = false;
        self.storedRemaining = 0;
        self.literals = .None;
        self.distances = .None;
        self.failure = .None;
    }

    public var isFinished: Bool { self.state == stateDone }

    /// Input bytes after the end of the DEFLATE stream (a zlib or gzip
    /// trailer). Only meaningful once `isFinished` is true.
    public var remainingInput: Array[UInt8] { copyTail(self.input, from: self.bitOffset / 8) }

    public mutating func write(input: &Slice[UInt8]) -> Result[Array[UInt8], DeflateError] {
        if let .Some(error) = self.failure { return .Err(error); }
        self.input.append(contentsOf: input.asUnsafeSlice());

        var output = self.window;
        let historyLength = output.count;
        match self.run(output) {
            .Ok(_) => {},
            .Err(error) => {
                self.failure = .Some(error);
                return .Err(error);
            }
        }

        // Keep only the window for future back-references, and drop input
        // that's been fully consumed.
        self.window = copyTail(output, from: output.count - windowSize);
        if self.state != stateDone {
            let consumedBytes = self.bitOffset / 8;
            self.input = copyTail(self.input, from: consumedBytes);
            self.bitOffset = self.bitOffset - consumedBytes * 8;
        }
        .Ok(copyTail(output, from: historyLength))
    }

    public mutating func finish() -> Result[(), DeflateError] {
        if let .Some(error) = self.failure { return .Err(error); }
        if self.state != stateDone { return .Err(.UnexpectedEnd); }
        .Ok(())
    }

    // Decodes as far as the buffered input allows, appending to `output`.
    // Returns Ok when it stops for lack of input or at the end of the stream.
    mutating func run(mutating output: Array[UInt8]) -> Result[(), DeflateError] {
        let data = self.input.asUnsafeSlice();
        var reader = BitReader(data: data, bitOffset: self.bitOffset);

        while self.state != stateDone {
            if self.state == stateBlockHeader {
                let checkpoint = reader.bitOffset;
                match self.readBlockHeader(reader) {
                    .Ok(complete) => {
                        if not complete {
                            self.bitOffset = checkpoint;
                            return .Ok(());
                        }
                    },
                    .Err(error) => { return .Err(error); }
                }
            } else if self.state == stateStored {
                let copied = reader.copyBytes(self.storedRemaining, into: output);
                self.storedRemaining = self.storedRemaining - copied;
                if self.storedRemaining > 0 {
                    // Input ran out mid-block; resume here on the next write.
                    self.bitOffset = reader.bitOffset;
                    return .Ok(());
                }
                self.endBlock();
            } else {
                match self.decodeSymbols(reader, output) {
                    .Ok(complete) => {
                        if not complete {
                            return .Ok(());
                        }
                    },
                    .Err(error) => { return .Err(error); }
                }
                self.endBlock();
            }
            self.bitOffset = reader.bitOffset;
        }

        // The stream ends on a byte boundary; anything after is a trailer.
        reader.alignToByte();
        self.bitOffset = reader.bitOffset;
        .Ok(())
    }

    mutating func endBlock() {
        if self.isFinalBlock {
            self.state = stateDone;
        } else {
            self.state = stateBlockHeader;
        }
    }

    // Reads a block header and any dynamic tables. Returns false, having
    // consumed nothing that matters, when the header is cut off.
    mutating func readBlockHeader(mutating reader: BitReader) -> Result[Bool, DeflateError] {
        let isFinal = reader.read(1) == 1;
        let blockType = reader.read(2);
        if reader.isOverrun { return .Ok(false); }

        if blockType == 0 {
            reader.alignToByte();
            let length = reader.read(16);
            let complement = reader.read(16);
            if reader.isOverrun { return .Ok(false); }
            if length != (complement ^ 0xFFFF) { return .Err(.InvalidStoredLength); }
            self.storedRemaining = length;
            self.state = stateStored;
        } else if blockType == 1 {
            self.literals = .Some(try HuffmanDecoder.build(fixedLiteralLengths()));
            self.distances = .Some(try HuffmanDecoder.build(fixedDistanceLengths()));
            self.state = stateCompressed;
        } else if blockType == 2 {
            match try readDynamicTables(reader) {
                .Some(tables) => {
                    self.literals = .Some(tables.0);
                    self.distances = .Some(tables.1);
                },
                .None => { return .Ok(false); }
            }
            self.state = stateCompressed;
        } else {
            return .Err(.InvalidBlockType);
        }
        self.isFinalBlock = isFinal;
        .Ok(true)
    }

    // Decodes literal/length and distance symbols until the end-of-block
    // symbol. Returns false when input runs out, with `bitOffset` left at
    // the start of the cut-off symbol.
    mutating func decodeSymbols(mutating reader: BitReader, mutating output: Array[UInt8]) -> Result[Bool, DeflateError] {
        guard let .Some(literals) = self.literals else { return .Err(.InvalidHuffmanCode); }
        guard let .Some(distances) = self.distances else { return .Err(.InvalidHuffmanCode); }

        loop {
            let checkpoint = reader.bitOffset;
            let symbol = literals.decode(reader);
            if reader.isOverrun {
                self.bitOffset = checkpoint;
                return .Ok(false);
            }
            if symbol < 0 { return .Err(.InvalidHuffmanCode); }
            if symbol < 256 {
                output.append(UInt8(truncating: symbol));
                continue;
            }
            if symbol == 256 { return .Ok(true); }

            let lengthIndex = symbol - 257;
            if lengthIndex >= 29 { return .Err(.InvalidHuffmanCode); }
            let length = LENGTH_BASES(lengthIndex) + reader.read(LENGTH_EXTRA_BITS(lengthIndex));
            let distanceSymbol = distances.decode(reader);
            if reader.isOverrun {
                self.bitOffset = checkpoint;
                return .Ok(false);
            }
            if distanceSymbol < 0 or distanceSymbol >= 30 { return .Err(.InvalidHuffmanCode); }
            let distance = DISTANCE_BASES(distanceSymbol) + reader.read(DISTANCE_EXTRA_BITS(distanceSymbol));
            if reader.isOverrun {
                self.bitOffset = checkpoint;
                return .Ok(false);
            }
            if distance > output.count { return .Err(.InvalidDistance); }

            // Byte by byte, since the source can overlap what's being written.
            // One reserve up front; `from + k` is always below `output.count`,
            // so the reads skip the bounds check.
            let from = output.count - distance;
            output.reserveCapacity(output.count + length);
            for k in 0..<length {
                output.append(unsafe output(unchecked: from + k));
            }
        }
    }
}

/// Reads the code-length tables of a dynamic block (RFC 1951 §3.2.7).
/// Returns None when the input is cut off partway.
func readDynamicTables(mutating reader: BitReader) -> Result[(HuffmanDecoder, HuffmanDecoder)?, DeflateError] {
    let literalCount = reader.read(5) + 257;
    let distanceCount = reader.read(5) + 1;
    let codeLengthCount = reader.read(4) + 4;
    if reader.isOverrun { return .Ok(.None); }
    if literalCount > 286 or distanceCount > 30 { return .Err(.InvalidHuffmanCode); }

    var codeLengthLengths = Array[Int64](repeating: 0, count: 19);
    for i in 0..<codeLengthCount {
        codeLengthLengths(CODE_LENGTH_ORDER(i)) = reader.read(3);
    }
    if reader.isOverrun { return .Ok(.None); }
    let codeLengthDecoder = try HuffmanDecoder.build(codeLengthLengths);

    // Literal and distance lengths are one run-length-coded sequence.
    let total = literalCount + distanceCount;
    var lengths: [Int64] = [];
    while lengths.count < total {
        let symbol = codeLengthDecoder.decode(reader);
        if reader.isOverrun { return .Ok(.None); }
        if symbol < 0 { return .Err(.InvalidHuffmanCode); }
        if symbol < 16 {
            lengths.append(symbol);
            continue;
        }
        var repeatValue: Int64 = 0;
        var repeatCount: Int64 = 0;
        if symbol == 16 {
            if lengths.count == 0 { return .Err(.InvalidHuffmanCode); }
            repeatValue = lengths(lengths.count - 1);
            repeatCount = 3 + reader.read(2);
        } else if symbol == 17 {
            repeatCount = 3 + reader.read(3);
        } else {
            repeatCount = 11 + reader.read(7);
        }
        if reader.isOverrun { return .Ok(.None); }
        if lengths.count + repeatCount > total { return .Err(.InvalidHuffmanCode); }
        for _ in 0..<repeatCount { lengths.append(repeatValue); }
    }

    var literalLengths: [Int64] = [];
    for i in 0..<literalCount { literalLengths.append(lengths(i)); }
    var distanceLengths: [Int64] = [];
    for i in literalCount..<total { distanceLengths.append(lengths(i)); }
    // A block must be able to end.
    if literalLengths(256) == 0 { return .Err(.InvalidHuffmanCode); }

    let literals = try HuffmanDecoder.build(literalLengths);
    let distances = try HuffmanDecoder.build(distanceLengths);
    .Ok(.Some((literals, distances)))
}
