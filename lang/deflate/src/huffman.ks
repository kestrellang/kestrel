module deflate

// Codes up to this many bits decode with one table lookup.
let fastBits: Int64 = 9

/// A canonical Huffman decoding table built from per-symbol code lengths.
struct HuffmanDecoder {
    // How many codes have each length, 0...15.
    var counts: Array[Int64];
    // Symbols ordered by (length, symbol), the canonical code order.
    var symbols: Array[Int64];
    // Indexed by the next `fastBits` input bits: (symbol << 4) | length, or
    // 0 when the code is longer than `fastBits`.
    var fast: Array[Int64];

    /// Builds the table. Fails when the lengths over-subscribe the code
    /// space. Incomplete codes are allowed: a stream that uses a missing
    /// code fails at decode time instead.
    static func build(lengths: Array[Int64]) -> Result[HuffmanDecoder, DeflateError] {
        var counts = Array[Int64](repeating: 0, count: 16);
        for length in lengths { counts(length) = counts(length) + 1; }
        counts(0) = 0;

        var left: Int64 = 1;
        for length in 1..<16 {
            left = (left << 1) - counts(length);
            if left < 0 { return .Err(.InvalidHuffmanCode); }
        }

        // Where each length's symbols start in `symbols`.
        var offsets = Array[Int64](repeating: 0, count: 16);
        for length in 1..<15 {
            offsets(length + 1) = offsets(length) + counts(length);
        }
        var symbols = Array[Int64](repeating: 0, count: lengths.count);
        for symbol in 0..<lengths.count {
            let length = lengths(symbol);
            if length != 0 {
                symbols(offsets(length)) = symbol;
                offsets(length) = offsets(length) + 1;
            }
        }

        // First canonical code of each length.
        var nextCode = Array[Int64](repeating: 0, count: 16);
        var code: Int64 = 0;
        for length in 1..<16 {
            code = (code + counts(length - 1)) << 1;
            nextCode(length) = code;
        }

        let fastSize: Int64 = 1 << fastBits;
        var fast = Array[Int64](repeating: 0, count: fastSize);
        for symbol in 0..<lengths.count {
            let length = lengths(symbol);
            if length == 0 { continue; }
            let symbolCode = nextCode(length);
            nextCode(length) = symbolCode + 1;
            if length > fastBits { continue; }
            // The stream is read LSB first, so index by the reversed code and
            // fill every slot whose low bits match it.
            var slot = Int64(from: reverseBits(UInt64(from: symbolCode), length));
            while slot < fastSize {
                fast(slot) = (symbol << 4) | length;
                slot = slot + (1 << length);
            }
        }

        .Ok(HuffmanDecoder(counts: counts, symbols: symbols, fast: fast))
    }

    /// Decodes one symbol, or returns -1 for a code the table doesn't define.
    func decode(mutating reader: BitReader) -> Int64 {
        let entry = self.fast(Int64(from: reader.peek(fastBits)));
        if entry != 0 {
            reader.consume(entry & 15);
            return entry >> 4;
        }
        self.decodeSlow(reader)
    }

    // Walks the canonical code one bit at a time (as zlib's puff does), for
    // codes longer than the fast table covers.
    func decodeSlow(mutating reader: BitReader) -> Int64 {
        var code: Int64 = 0;
        var first: Int64 = 0;
        var index: Int64 = 0;
        for length in 1..<16 {
            code = code | reader.read(1);
            let count = self.counts(length);
            if code - first < count {
                return self.symbols(index + code - first);
            }
            index = index + count;
            first = (first + count) << 1;
            code = code << 1;
        }
        return -1;
    }
}

/// Builds code lengths for `frequencies`, no longer than `maxLength` bits.
///
/// Uses the Moffat–Katajainen in-place algorithm for optimal lengths, then
/// rebalances the longest codes (the approach miniz and zlib take) when
/// the optimal tree is deeper than DEFLATE allows. Symbols with frequency
/// 0 get length 0. A lone used symbol gets length 1.
func buildCodeLengths(frequencies: Array[Int64], maxLength: Int64) -> Array[Int64] {
    var lengths = Array[Int64](repeating: 0, count: frequencies.count);

    // (frequency, symbol) pairs for used symbols, sorted by frequency.
    var used: Array[(Int64, Int64)] = [];
    for symbol in 0..<frequencies.count {
        if frequencies(symbol) > 0 { used.append((frequencies(symbol), symbol)); }
    }
    if used.count == 0 { return lengths; }
    if used.count == 1 {
        lengths(used(0).1) = 1;
        // Through a binding: see `subscript_assign_then_return_array`.
        let out = lengths;
        return out;
    }
    used.sort(by: { (a: (Int64, Int64), b: (Int64, Int64)) in a.0 < b.0 or (a.0 == b.0 and a.1 < b.1) });

    var weights = Array[Int64](repeating: 0, count: used.count);
    for i in 0..<used.count { weights(i) = used(i).0; }
    let depths = minimumRedundancy(weights);

    // Count codes per length, folding anything deeper than maxLength into
    // maxLength, then fix the Kraft sum.
    var lengthCounts = Array[Int64](repeating: 0, count: 64);
    for depth in depths {
        var d = depth;
        if d > maxLength { d = maxLength; }
        lengthCounts(d) = lengthCounts(d) + 1;
    }
    enforceMaxLength(lengthCounts, maxLength);

    // Longest codes go to the rarest symbols. `used` is sorted rarest first.
    var next: Int64 = 0;
    var length = maxLength;
    while length >= 1 {
        for _ in 0..<lengthCounts(length) {
            lengths(used(next).1) = length;
            next = next + 1;
        }
        length = length - 1;
    }
    lengths
}

/// Optimal code lengths for weights sorted ascending (Moffat & Katajainen,
/// "In-Place Calculation of Minimum-Redundancy Codes", 1995). The result
/// is in the same order as the input.
func minimumRedundancy(weights: Array[Int64]) -> Array[Int64] {
    var a = weights;
    let n = a.count;

    // Phase 1: build the tree, leaving parent pointers in a.
    var root: Int64 = 0;
    var leaf: Int64 = 0;
    for next in 0..<(n - 1) {
        if leaf >= n or (root < next and a(root) < a(leaf)) {
            a(next) = a(root);
            a(root) = next;
            root = root + 1;
        } else {
            a(next) = a(leaf);
            leaf = leaf + 1;
        }
        if leaf >= n or (root < next and a(root) < a(leaf)) {
            a(next) = a(next) + a(root);
            a(root) = next;
            root = root + 1;
        } else {
            a(next) = a(next) + a(leaf);
            leaf = leaf + 1;
        }
    }

    // Phase 2: convert parent pointers to internal node depths.
    a(n - 2) = 0;
    var i = n - 3;
    while i >= 0 {
        a(i) = a(a(i)) + 1;
        i = i - 1;
    }

    // Phase 3: convert internal node depths to leaf depths.
    var available: Int64 = 1;
    var used: Int64 = 0;
    var depth: Int64 = 0;
    var r = n - 2;
    var out = n - 1;
    while available > 0 {
        while r >= 0 and a(r) == depth {
            used = used + 1;
            r = r - 1;
        }
        while available > used {
            a(out) = depth;
            out = out - 1;
            available = available - 1;
        }
        available = 2 * used;
        depth = depth + 1;
        used = 0;
    }
    a
}

/// Adjusts per-length code counts so the Kraft sum is exactly 1 after
/// codes deeper than `maxLength` were clamped to it.
func enforceMaxLength(mutating lengthCounts: Array[Int64], maxLength: Int64) {
    var total: Int64 = 0;
    for length in 1..<(maxLength + 1) {
        total = total + (lengthCounts(length) << (maxLength - length));
    }
    let limit: Int64 = 1 << maxLength;
    while total > limit {
        // Move one code from the deepest level up, and split a shallower
        // leaf into two to absorb it.
        lengthCounts(maxLength) = lengthCounts(maxLength) - 1;
        var length = maxLength - 1;
        while length > 0 {
            if lengthCounts(length) != 0 {
                lengthCounts(length) = lengthCounts(length) - 1;
                lengthCounts(length + 1) = lengthCounts(length + 1) + 2;
                break;
            }
            length = length - 1;
        }
        total = total - 1;
    }
}

/// Canonical codes for `lengths`, as numbers to write MSB first.
func canonicalCodes(lengths: Array[Int64]) -> Array[UInt64] {
    var counts = Array[Int64](repeating: 0, count: 16);
    for length in lengths { counts(length) = counts(length) + 1; }
    counts(0) = 0;
    var nextCode = Array[Int64](repeating: 0, count: 16);
    var code: Int64 = 0;
    for length in 1..<16 {
        code = (code + counts(length - 1)) << 1;
        nextCode(length) = code;
    }
    var codes = Array[UInt64](repeating: 0, count: lengths.count);
    for symbol in 0..<lengths.count {
        let length = lengths(symbol);
        if length != 0 {
            codes(symbol) = UInt64(from: nextCode(length));
            nextCode(length) = nextCode(length) + 1;
        }
    }
    codes
}
