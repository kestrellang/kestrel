module crypto.digest

/// BLAKE3, the plain 32-byte hash.
///
/// Input is split into 1 KiB chunks hashed into a binary tree, so it
/// parallelises and streams well (this implementation is sequential).
/// BLAKE3's keyed mode, key derivation mode and extendable output are
/// not exposed yet.
///
/// `blockSize` is BLAKE3's 64-byte block. HMAC-BLAKE3 works through
/// `Digest`, but BLAKE3's own keyed mode is the idiomatic MAC.
public struct BLAKE3: Digest {
    var chunk: BLAKE3Chunk;
    /// Chaining values of completed subtrees, merged pairwise as chunks
    /// finish (one entry per set bit of the chunk count).
    var stack: Array[Array[UInt32]];

    public static var digestSize: Int64 { 32 }
    public static var blockSize: Int64 { 64 }

    public init() {
        self.chunk = BLAKE3Chunk(BLAKE3.IV, 0);
        self.stack = Array[Array[UInt32]]();
    }

    public mutating func update[S](bytes: S) where S: Slice[UInt8] {
        let input = bytes.asSlice();
        var offset: Int64 = 0;
        while offset < input.count {
            // A full chunk is only finished once more input arrives, so the
            // last chunk can still be marked as the root.
            if self.chunk.count == 1024 {
                let chunkValue = self.chunk.output().chainingValue();
                let totalChunks = self.chunk.counter + 1;
                self.pushChunk(chunkValue, totalChunks);
                self.chunk = BLAKE3Chunk(BLAKE3.IV, totalChunks);
            }
            var take = 1024 - self.chunk.count;
            if take > input.count - offset { take = input.count - offset; }
            self.chunk.update(input.drop(first: offset).prefix(take));
            offset = offset + take;
        }
    }

    public func finalize() -> DigestOutput {
        var output = self.chunk.output();
        var remaining = self.stack.count;
        while remaining > 0 {
            remaining = remaining - 1;
            output = BLAKE3.parent(self.stack(remaining), output.chainingValue());
        }
        DigestOutput(bytes: output.rootBytes())
    }

    // --- internals ---

    static var IV: Array[UInt32] {
        [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
            0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
        ]
    }

    static var chunkStart: UInt32 { 1 }
    static var chunkEnd: UInt32 { 2 }
    static var parentFlag: UInt32 { 4 }
    static var root: UInt32 { 8 }

    /// Adds a finished chunk's chaining value, merging completed subtrees:
    /// each trailing zero bit of `totalChunks` is one pair to merge.
    mutating func pushChunk(chunkValue: Array[UInt32], totalChunks: UInt64) {
        var value = chunkValue;
        var count = totalChunks;
        while count & 1 == 0 {
            let left = self.stack.pop().expect("a set bit below the top means a subtree is waiting");
            value = BLAKE3.parent(left, value).chainingValue();
            count = count >> 1;
        }
        self.stack.append(value);
    }

    /// The node joining two child chaining values.
    static func parent(left: Array[UInt32], right: Array[UInt32]) -> BLAKE3Output {
        var words: [UInt32] = [];
        words.append(contentsOf: left.asSlice());
        words.append(contentsOf: right.asSlice());
        BLAKE3Output(BLAKE3.IV, words, 0, 64, BLAKE3.parentFlag)
    }

    /// The compression function: 7 rounds over a 16-word state.
    static func compress(
        chainingValue: Array[UInt32],
        block: Array[UInt32],
        counter: UInt64,
        blockLength: UInt32,
        flags: UInt32
    ) -> Array[UInt32] {
        let iv = BLAKE3.IV;
        var state: [UInt32] = [];
        state.append(contentsOf: chainingValue.asSlice());
        state.append(iv(0));
        state.append(iv(1));
        state.append(iv(2));
        state.append(iv(3));
        state.append(UInt32(from: counter & 0xffffffff));
        state.append(UInt32(from: counter >> 32));
        state.append(blockLength);
        state.append(flags);

        let permutation: [Int64] = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8];
        var m = block;
        for round in 0..<7 {
            BLAKE3.mix(state, 0, 4, 8, 12, m(0), m(1));
            BLAKE3.mix(state, 1, 5, 9, 13, m(2), m(3));
            BLAKE3.mix(state, 2, 6, 10, 14, m(4), m(5));
            BLAKE3.mix(state, 3, 7, 11, 15, m(6), m(7));
            BLAKE3.mix(state, 0, 5, 10, 15, m(8), m(9));
            BLAKE3.mix(state, 1, 6, 11, 12, m(10), m(11));
            BLAKE3.mix(state, 2, 7, 8, 13, m(12), m(13));
            BLAKE3.mix(state, 3, 4, 9, 14, m(14), m(15));
            if round < 6 {
                var permuted = Array[UInt32](repeating: 0, count: 16);
                for i in 0..<16 { permuted(i) = m(permutation(i)); }
                m = permuted;
            }
        }

        for i in 0..<8 {
            state(i) = state(i) ^ state(i + 8);
            state(i + 8) = state(i + 8) ^ chainingValue(i);
        }
        state
    }

    static func mix(
        mutating v: Array[UInt32],
        a: Int64, b: Int64, c: Int64, d: Int64,
        x: UInt32, y: UInt32
    ) {
        v(a) = v(a) + v(b) + x;
        v(d) = (v(d) ^ v(a)).rotateRight(by: 16);
        v(c) = v(c) + v(d);
        v(b) = (v(b) ^ v(c)).rotateRight(by: 12);
        v(a) = v(a) + v(b) + y;
        v(d) = (v(d) ^ v(a)).rotateRight(by: 8);
        v(c) = v(c) + v(d);
        v(b) = (v(b) ^ v(c)).rotateRight(by: 7);
    }

    /// 64 little-endian bytes as 16 words (short blocks are zero-padded).
    static func words(block: Array[UInt8]) -> Array[UInt32] {
        var result = Array[UInt32](repeating: 0, count: 16);
        for i in 0..<block.count {
            result(i / 4) = result(i / 4) | (UInt32(from: block(i)) << (8 * (i % 4)));
        }
        result
    }
}

/// One 1 KiB chunk in progress: up to 16 blocks chained together.
struct BLAKE3Chunk {
    var chainingValue: Array[UInt32];
    var counter: UInt64;
    var block: Array[UInt8];
    var blocksCompressed: Int64;

    init(key: Array[UInt32], counter: UInt64) {
        self.chainingValue = key;
        self.counter = counter;
        self.block = Array[UInt8]();
        self.blocksCompressed = 0;
    }

    /// Bytes absorbed into this chunk so far.
    var count: Int64 { self.blocksCompressed * 64 + self.block.count }

    func startFlag() -> UInt32 {
        if self.blocksCompressed == 0 { BLAKE3.chunkStart } else { 0 }
    }

    mutating func update(input: ArraySlice[UInt8]) {
        var offset: Int64 = 0;
        while offset < input.count {
            // Like chunks, a full block is only compressed once more input
            // arrives, so the chunk's last block can carry CHUNK_END.
            if self.block.count == 64 {
                let words = BLAKE3.words(self.block);
                let full = BLAKE3.compress(self.chainingValue, words, self.counter, 64, self.startFlag());
                self.chainingValue = BLAKE3Output.firstEight(full);
                self.blocksCompressed = self.blocksCompressed + 1;
                self.block = Array[UInt8]();
            }
            var take = 64 - self.block.count;
            if take > input.count - offset { take = input.count - offset; }
            self.block.append(contentsOf: input.drop(first: offset).prefix(take));
            offset = offset + take;
        }
    }

    func output() -> BLAKE3Output {
        BLAKE3Output(
            self.chainingValue,
            BLAKE3.words(self.block),
            self.counter,
            UInt32(from: self.block.count),
            self.startFlag() | BLAKE3.chunkEnd
        )
    }
}

/// A node's inputs, kept unevaluated until we know whether it's the root
/// (the root is compressed with the ROOT flag; inner nodes aren't).
struct BLAKE3Output {
    var inputChainingValue: Array[UInt32];
    var block: Array[UInt32];
    var counter: UInt64;
    var blockLength: UInt32;
    var flags: UInt32;

    init(inputChainingValue: Array[UInt32], block: Array[UInt32], counter: UInt64, blockLength: UInt32, flags: UInt32) {
        self.inputChainingValue = inputChainingValue;
        self.block = block;
        self.counter = counter;
        self.blockLength = blockLength;
        self.flags = flags;
    }

    func chainingValue() -> Array[UInt32] {
        BLAKE3Output.firstEight(BLAKE3.compress(self.inputChainingValue, self.block, self.counter, self.blockLength, self.flags))
    }

    /// The first 32 output bytes of the root node.
    func rootBytes() -> Array[UInt8] {
        let words = BLAKE3.compress(self.inputChainingValue, self.block, 0, self.blockLength, self.flags | BLAKE3.root);
        var result: [UInt8] = [];
        for i in 0..<8 {
            result.append(contentsOf: words(i).toBytesLittleEndian().asSlice());
        }
        result
    }

    static func firstEight(words: Array[UInt32]) -> Array[UInt32] {
        var result: [UInt32] = [];
        result.append(contentsOf: words.asSlice().prefix(8));
        result
    }
}
