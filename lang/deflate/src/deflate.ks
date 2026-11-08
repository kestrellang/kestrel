module deflate

// How much new input the compressor buffers before emitting blocks.
let compressChunkSize: Int64 = 131072
// Most tokens in one Huffman block, so each block's code fits its data.
let tokensPerBlock: Int64 = 16384
let hashBits: Int64 = 15
let minMatch: Int64 = 3
let maxMatch: Int64 = 258

/// Match-search effort for one compression level.
struct SearchParameters {
    let maxChain: Int64;     // candidates checked per position
    let niceLength: Int64;   // stop searching once a match is this long
    let lazy: Bool;          // try the next position before taking a match

    static func forLevel(level: Int64) -> SearchParameters {
        if level <= 1 { return SearchParameters(maxChain: 4, niceLength: 8, lazy: false); }
        if level == 2 { return SearchParameters(maxChain: 8, niceLength: 16, lazy: false); }
        if level == 3 { return SearchParameters(maxChain: 16, niceLength: 32, lazy: false); }
        if level == 4 { return SearchParameters(maxChain: 16, niceLength: 16, lazy: true); }
        if level == 5 { return SearchParameters(maxChain: 32, niceLength: 32, lazy: true); }
        if level == 6 { return SearchParameters(maxChain: 128, niceLength: 128, lazy: true); }
        if level == 7 { return SearchParameters(maxChain: 256, niceLength: 258, lazy: true); }
        if level == 8 { return SearchParameters(maxChain: 1024, niceLength: 258, lazy: true); }
        SearchParameters(maxChain: 4096, niceLength: 258, lazy: true)
    }
}

/// LZ77 output: a literal when `length` is 0, otherwise a back-reference.
struct Tokens {
    var lengths: Array[Int64];
    var values: Array[Int64];      // the literal byte, or the distance
    // The input byte where each token starts, so a block can fall back to
    // storing its bytes raw.
    var positions: Array[Int64];
}

/// Compresses data into raw DEFLATE (RFC 1951), fed in chunks of any size.
///
/// # Examples
///
/// ```
/// var deflater = Deflater(level: .best);
/// var output = deflater.write(data);
/// output.append(contentsOf: deflater.finish());
/// ```
public struct Deflater: Compressor {
    let level: CompressionLevel;
    // Already-compressed history (for matches), then pending input.
    var buffer: Array[UInt8];
    var historyLength: Int64;
    var writer: BitWriter;
    var isFinished: Bool;
    // Length (3...258) to length-symbol index, and distance to distance symbol.
    var lengthSymbols: Array[Int64];
    var distanceSymbols: Array[Int64];

    public init(level level: CompressionLevel = CompressionLevel.standard) {
        self.level = level;
        self.buffer = [];
        self.historyLength = 0;
        self.writer = BitWriter();
        self.isFinished = false;
        self.lengthSymbols = makeLengthSymbolTable();
        self.distanceSymbols = makeDistanceSymbolTable();
    }

    public mutating func write[S](input: &S) -> Array[UInt8] where S: Sliceable[UInt8], S: not Static, S: not Copyable, S: not Sized {
        if self.isFinished { return []; }
        self.buffer.append(contentsOf: input.asUnsafeSlice());
        if self.buffer.count - self.historyLength >= compressChunkSize {
            self.compressPending(false);
        }
        self.writer.takeBytes()
    }

    public mutating func finish() -> Array[UInt8] {
        if self.isFinished { return []; }
        self.compressPending(true);
        self.writer.alignToByte();
        self.isFinished = true;
        self.writer.takeBytes()
    }

    // Compresses everything after the history into one or more blocks.
    mutating func compressPending(isFinal: Bool) {
        let start = self.historyLength;
        let end = self.buffer.count;

        if start == end {
            // Nothing new; a final call still has to end the stream.
            if isFinal { self.writeEmptyFinalBlock(); }
        } else if self.level.value == 0 {
            self.writeStored(start, end, isFinal);
        } else {
            let tokens = findMatches(self.buffer, start, end, SearchParameters.forLevel(self.level.value));
            var first: Int64 = 0;
            while first < tokens.lengths.count {
                var last = first + tokensPerBlock;
                if last > tokens.lengths.count { last = tokens.lengths.count; }
                let byteStart = tokens.positions(first);
                var byteEnd = end;
                if last < tokens.lengths.count { byteEnd = tokens.positions(last); }
                let blockIsFinal = isFinal and last == tokens.lengths.count;
                self.writeBlock(tokens, first, last, byteStart, byteEnd, blockIsFinal);
                first = last;
            }
        }

        // Keep the last window of input as history for the next chunk.
        var keepFrom = end - windowSize;
        if keepFrom < 0 { keepFrom = 0; }
        self.buffer = copyTail(self.buffer, from: keepFrom);
        self.historyLength = self.buffer.count;
    }

    mutating func writeEmptyFinalBlock() {
        // A fixed-Huffman block holding only end-of-block (code 0000000).
        self.writer.write(1, 1);
        self.writer.write(1, 2);
        self.writer.write(0, 7);
    }

    // Stored blocks of at most 65535 bytes each.
    mutating func writeStored(start: Int64, end: Int64, isFinal: Bool) {
        var position = start;
        while position < end {
            var length = end - position;
            if length > 65535 { length = 65535; }
            let isLast = position + length == end;
            var finalBit: UInt64 = 0;
            if isFinal and isLast { finalBit = 1; }
            self.writer.write(finalBit, 1);
            self.writer.write(0, 2);
            self.writer.alignToByte();
            self.writer.write(UInt64(from: length), 16);
            self.writer.write(UInt64(from: length ^ 0xFFFF), 16);
            for i in position..<(position + length) {
                self.writer.bytes.append(self.buffer(i));
            }
            position = position + length;
        }
    }

    // Writes tokens first..<last as whichever of dynamic, fixed or stored
    // is smallest.
    mutating func writeBlock(tokens: Tokens, first: Int64, last: Int64, byteStart: Int64, byteEnd: Int64, isFinal: Bool) {
        var literalFrequencies = Array[Int64](repeating: 0, count: 286);
        var distanceFrequencies = Array[Int64](repeating: 0, count: 30);
        for i in first..<last {
            let length = tokens.lengths(i);
            if length == 0 {
                let literal = tokens.values(i);
                literalFrequencies(literal) = literalFrequencies(literal) + 1;
            } else {
                let symbol = 257 + self.lengthSymbols(length);
                literalFrequencies(symbol) = literalFrequencies(symbol) + 1;
                let distanceSymbol = self.distanceSymbols(tokens.values(i));
                distanceFrequencies(distanceSymbol) = distanceFrequencies(distanceSymbol) + 1;
            }
        }
        literalFrequencies(256) = 1;

        let header = DynamicHeader.build(literalFrequencies, distanceFrequencies);
        let dynamicBits = 3 + header.bitCount + self.payloadBits(literalFrequencies, distanceFrequencies, header.literalLengths, header.distanceLengths);
        let fixedBits = 3 + self.payloadBits(literalFrequencies, distanceFrequencies, fixedLiteralLengths(), fixedDistanceLengths());
        let rawLength = byteEnd - byteStart;
        let storedBits = rawLength * 8 + ((rawLength + 65534) / 65535) * 40 + 10;

        if storedBits < dynamicBits and storedBits < fixedBits {
            self.writeStored(byteStart, byteEnd, isFinal);
            return;
        }

        var finalBit: UInt64 = 0;
        if isFinal { finalBit = 1; }
        self.writer.write(finalBit, 1);
        if fixedBits <= dynamicBits {
            self.writer.write(1, 2);
            self.writeTokens(tokens, first, last, fixedLiteralLengths(), fixedDistanceLengths());
        } else {
            self.writer.write(2, 2);
            header.write(self.writer);
            self.writeTokens(tokens, first, last, header.literalLengths, header.distanceLengths);
        }
    }

    // Bits for the block's symbols and extra bits under the given code.
    func payloadBits(literalFrequencies: Array[Int64], distanceFrequencies: Array[Int64], literalLengths: Array[Int64], distanceLengths: Array[Int64]) -> Int64 {
        var bits: Int64 = 0;
        for symbol in 0..<286 {
            let frequency = literalFrequencies(symbol);
            if frequency == 0 { continue; }
            var extra: Int64 = 0;
            if symbol > 256 { extra = LENGTH_EXTRA_BITS(symbol - 257); }
            bits = bits + frequency * (literalLengths(symbol) + extra);
        }
        for symbol in 0..<30 {
            let frequency = distanceFrequencies(symbol);
            if frequency == 0 { continue; }
            bits = bits + frequency * (distanceLengths(symbol) + DISTANCE_EXTRA_BITS(symbol));
        }
        bits
    }

    mutating func writeTokens(tokens: Tokens, first: Int64, last: Int64, literalLengths: Array[Int64], distanceLengths: Array[Int64]) {
        let literalCodes = canonicalCodes(literalLengths);
        let distanceCodes = canonicalCodes(distanceLengths);

        for i in first..<last {
            let length = tokens.lengths(i);
            if length == 0 {
                let literal = tokens.values(i);
                self.writer.writeCode(literalCodes(literal), literalLengths(literal));
                continue;
            }
            let lengthIndex = self.lengthSymbols(length);
            let symbol = 257 + lengthIndex;
            self.writer.writeCode(literalCodes(symbol), literalLengths(symbol));
            self.writer.write(UInt64(from: length - LENGTH_BASES(lengthIndex)), LENGTH_EXTRA_BITS(lengthIndex));

            let distance = tokens.values(i);
            let distanceSymbol = self.distanceSymbols(distance);
            self.writer.writeCode(distanceCodes(distanceSymbol), distanceLengths(distanceSymbol));
            self.writer.write(UInt64(from: distance - DISTANCE_BASES(distanceSymbol)), DISTANCE_EXTRA_BITS(distanceSymbol));
        }
        self.writer.writeCode(literalCodes(256), literalLengths(256));
    }
}

/// Index into the length tables for each match length 0...258 (entries
/// below 3 are unused).
func makeLengthSymbolTable() -> Array[Int64] {
    var table = Array[Int64](repeating: 0, count: maxMatch + 1);
    var symbol: Int64 = 0;
    for length in minMatch..<(maxMatch + 1) {
        while symbol + 1 < LENGTH_BASES.count and LENGTH_BASES(symbol + 1) <= length { symbol = symbol + 1; }
        table(length) = symbol;
    }
    // 258 has its own code, not the top of 227's range.
    table(maxMatch) = 28;
    // Returning through a binding works around the OSSA crash documented
    // by `subscript_assign_then_return_array`.
    let out = table;
    out
}

/// Distance symbol for each distance 0...32768 (entry 0 is unused).
func makeDistanceSymbolTable() -> Array[Int64] {
    var table = Array[Int64](repeating: 0, count: windowSize + 1);
    var symbol: Int64 = 0;
    for distance in 1..<(windowSize + 1) {
        while symbol + 1 < DISTANCE_BASES.count and DISTANCE_BASES(symbol + 1) <= distance { symbol = symbol + 1; }
        table(distance) = symbol;
    }
    table
}

/// Finds LZ77 matches for data(start..<end), with data(..<start) as
/// history, using hash chains over 3-byte prefixes.
func findMatches(data: Array[UInt8], start: Int64, end: Int64, search: SearchParameters) -> Tokens {
    var tokens = Tokens(lengths: [], values: [], positions: []);
    let hashSize: Int64 = 1 << hashBits;
    var head = Array[Int64](repeating: -1, count: hashSize);
    var previous = Array[Int64](repeating: -1, count: end);
    // Every position below this is already in the hash chains.
    var inserted: Int64 = 0;

    var position = start;
    while position < end {
        inserted = insertUpTo(data, end, head, previous, inserted, position);
        var found = longestMatch(data, end, head, previous, position, search);

        if search.lazy and found.0 >= minMatch and found.0 < search.niceLength and position + 1 < end {
            inserted = insertUpTo(data, end, head, previous, inserted, position + 1);
            let next = longestMatch(data, end, head, previous, position + 1, search);
            if next.0 > found.0 {
                tokens.lengths.append(0);
                tokens.values.append(Int64(from: data(position)));
                tokens.positions.append(position);
                position = position + 1;
                found = next;
            }
        }

        if found.0 >= minMatch {
            tokens.lengths.append(found.0);
            tokens.values.append(found.1);
            tokens.positions.append(position);
            position = position + found.0;
        } else {
            tokens.lengths.append(0);
            tokens.values.append(Int64(from: data(position)));
            tokens.positions.append(position);
            position = position + 1;
        }
    }
    tokens
}

func hashAt(data: Array[UInt8], position: Int64) -> Int64 {
    let a = Int64(from: data(position));
    let b = Int64(from: data(position + 1));
    let c = Int64(from: data(position + 2));
    ((a << 10) ^ (b << 5) ^ c) & ((1 << hashBits) - 1)
}

// Adds positions inserted..<target to the hash chains; returns the new mark.
func insertUpTo(data: Array[UInt8], end: Int64, mutating head: Array[Int64], mutating previous: Array[Int64], inserted: Int64, target: Int64) -> Int64 {
    var position = inserted;
    while position < target {
        if position + minMatch <= end {
            let hash = hashAt(data, position);
            previous(position) = head(hash);
            head(hash) = position;
        }
        position = position + 1;
    }
    position
}

// The longest earlier match at `position`: (length, distance), or length 0.
func longestMatch(data: Array[UInt8], end: Int64, head: Array[Int64], previous: Array[Int64], position: Int64, search: SearchParameters) -> (Int64, Int64) {
    if position + minMatch > end { return (0, 0); }
    var limit = end - position;
    if limit > maxMatch { limit = maxMatch; }

    var bestLength: Int64 = minMatch - 1;
    var bestDistance: Int64 = 0;
    var candidate = head(hashAt(data, position));
    var chain = search.maxChain;
    while candidate >= 0 and chain > 0 {
        let distance = position - candidate;
        if distance > windowSize { break; }
        // Check the byte just past the current best first: most candidates
        // fail there.
        if data(candidate + bestLength) == data(position + bestLength) {
            var length: Int64 = 0;
            while length < limit and data(candidate + length) == data(position + length) {
                length = length + 1;
            }
            if length > bestLength {
                bestLength = length;
                bestDistance = distance;
                if length >= search.niceLength or length == limit { break; }
            }
        }
        candidate = previous(candidate);
        chain = chain - 1;
    }
    if bestLength < minMatch { return (0, 0); }
    (bestLength, bestDistance)
}

/// The code-length tables of a dynamic block, ready to write.
struct DynamicHeader {
    var literalLengths: Array[Int64];
    var distanceLengths: Array[Int64];
    var literalCount: Int64;
    var distanceCount: Int64;
    var codeLengthLengths: Array[Int64];
    var codeLengthCount: Int64;
    // Run-length-coded lengths: symbol 0...18 and its extra-bit value.
    var runSymbols: Array[Int64];
    var runExtras: Array[Int64];
    // Size of everything after the 3-bit block header.
    var bitCount: Int64;

    static func build(literalFrequencies: Array[Int64], distanceFrequencies: Array[Int64]) -> DynamicHeader {
        let literalLengths = buildCodeLengths(literalFrequencies, 15);
        var distanceLengths = buildCodeLengths(distanceFrequencies, 15);
        // Decoders want at least one distance code even when none is used.
        var anyDistance = false;
        for length in distanceLengths { if length != 0 { anyDistance = true; } }
        if not anyDistance { distanceLengths(0) = 1; }

        var literalCount: Int64 = 286;
        while literalCount > 257 and literalLengths(literalCount - 1) == 0 { literalCount = literalCount - 1; }
        var distanceCount: Int64 = 30;
        while distanceCount > 1 and distanceLengths(distanceCount - 1) == 0 { distanceCount = distanceCount - 1; }

        var all: [Int64] = [];
        for i in 0..<literalCount { all.append(literalLengths(i)); }
        for i in 0..<distanceCount { all.append(distanceLengths(i)); }

        // Run-length code the lengths with symbols 16 (repeat previous),
        // 17 (short zero run) and 18 (long zero run).
        var runSymbols: [Int64] = [];
        var runExtras: [Int64] = [];
        var i: Int64 = 0;
        while i < all.count {
            let value = all(i);
            var run: Int64 = 1;
            while i + run < all.count and all(i + run) == value { run = run + 1; }
            var remaining = run;
            if value == 0 {
                while remaining >= 11 {
                    var take = remaining;
                    if take > 138 { take = 138; }
                    runSymbols.append(18);
                    runExtras.append(take - 11);
                    remaining = remaining - take;
                }
                if remaining >= 3 {
                    runSymbols.append(17);
                    runExtras.append(remaining - 3);
                    remaining = 0;
                }
            } else {
                runSymbols.append(value);
                runExtras.append(0);
                remaining = remaining - 1;
                while remaining >= 3 {
                    var take = remaining;
                    if take > 6 { take = 6; }
                    runSymbols.append(16);
                    runExtras.append(take - 3);
                    remaining = remaining - take;
                }
            }
            for _ in 0..<remaining {
                runSymbols.append(value);
                runExtras.append(0);
            }
            i = i + run;
        }

        var codeLengthFrequencies = Array[Int64](repeating: 0, count: 19);
        for symbol in runSymbols { codeLengthFrequencies(symbol) = codeLengthFrequencies(symbol) + 1; }
        let codeLengthLengths = buildCodeLengths(codeLengthFrequencies, 7);
        var codeLengthCount: Int64 = 19;
        while codeLengthCount > 4 and codeLengthLengths(CODE_LENGTH_ORDER(codeLengthCount - 1)) == 0 { codeLengthCount = codeLengthCount - 1; }

        var bits: Int64 = 5 + 5 + 4 + 3 * codeLengthCount;
        for k in 0..<runSymbols.count {
            let symbol = runSymbols(k);
            bits = bits + codeLengthLengths(symbol);
            if symbol == 16 { bits = bits + 2; }
            if symbol == 17 { bits = bits + 3; }
            if symbol == 18 { bits = bits + 7; }
        }

        DynamicHeader(
            literalLengths: literalLengths, distanceLengths: distanceLengths,
            literalCount: literalCount, distanceCount: distanceCount,
            codeLengthLengths: codeLengthLengths, codeLengthCount: codeLengthCount,
            runSymbols: runSymbols, runExtras: runExtras, bitCount: bits)
    }

    func write(mutating writer: BitWriter) {
        writer.write(UInt64(from: self.literalCount - 257), 5);
        writer.write(UInt64(from: self.distanceCount - 1), 5);
        writer.write(UInt64(from: self.codeLengthCount - 4), 4);
        for i in 0..<self.codeLengthCount {
            writer.write(UInt64(from: self.codeLengthLengths(CODE_LENGTH_ORDER(i))), 3);
        }
        let codes = canonicalCodes(self.codeLengthLengths);
        for k in 0..<self.runSymbols.count {
            let symbol = self.runSymbols(k);
            writer.writeCode(codes(symbol), self.codeLengthLengths(symbol));
            if symbol == 16 { writer.write(UInt64(from: self.runExtras(k)), 2); }
            if symbol == 17 { writer.write(UInt64(from: self.runExtras(k)), 3); }
            if symbol == 18 { writer.write(UInt64(from: self.runExtras(k)), 7); }
        }
    }
}
