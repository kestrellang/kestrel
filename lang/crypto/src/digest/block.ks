module crypto.digest

/// The algorithm-specific half of a block-based hash: its state, and how
/// one full block changes it. `BlockBuffer` supplies everything else.
protocol BlockFunction {
    mutating func compress(block: ArraySlice[UInt8])
}

/// How Merkle–Damgård padding encodes the message length (in bits).
enum LengthEncoding {
    /// SHA-1, SHA-224, SHA-256.
    case BigEndian64
    /// SHA-384, SHA-512, SHA-512/256.
    case BigEndian128
    /// MD5.
    case LittleEndian64
}

/// Feeds arbitrary input to a `BlockFunction` in whole blocks, holding the
/// partial tail until more input or finalization arrives.
///
/// The one copy of the buffering logic shared by SHA-1, SHA-2, MD5 and the
/// SHA-3 sponge. (BLAKE2b and BLAKE3 hold back their last full block so it
/// can carry a "final" flag, so they buffer differently.)
struct BlockBuffer[F] where F: BlockFunction {
    var function: F;
    var pending: Array[UInt8];
    var totalLength: UInt64;
    var blockLength: Int64;

    init(function function: F, blockLength blockLength: Int64) {
        self.function = function;
        self.pending = Array[UInt8]();
        self.totalLength = 0;
        self.blockLength = blockLength;
    }

    mutating func update(bytes: ArraySlice[UInt8]) {
        self.totalLength = self.totalLength + UInt64(from: bytes.count);
        var offset: Int64 = 0;

        // Top up a partial block first.
        if self.pending.count > 0 {
            let needed = self.blockLength - self.pending.count;
            if bytes.count < needed {
                self.pending.append(contentsOf: bytes);
                return;
            }
            self.pending.append(contentsOf: bytes.prefix(needed));
            var block = self.pending;
            self.pending = Array[UInt8]();
            self.function.compress(block.asSlice());
            offset = needed;
        }

        // Then whole blocks straight from the input, without copying.
        while offset + self.blockLength <= bytes.count {
            self.function.compress(bytes.drop(first: offset).prefix(self.blockLength));
            offset = offset + self.blockLength;
        }

        if offset < bytes.count {
            self.pending.append(contentsOf: bytes.drop(first: offset));
        }
    }

    /// The state after Merkle–Damgård padding: `0x80`, zeros, then the
    /// message length. Works on a copy, so the hasher can keep absorbing.
    func finishedWithLengthPadding(encoding: LengthEncoding) -> F {
        var function = self.function;
        let lengthSize: Int64 = match encoding { .BigEndian128 => 16, _ => 8 };

        var tail: [UInt8] = [];
        tail.append(contentsOf: self.pending.asSlice());
        tail.append(0x80);
        while tail.count % self.blockLength != self.blockLength - lengthSize {
            tail.append(0);
        }

        let bitLength = self.totalLength * 8;
        match encoding {
            .BigEndian64 => { tail.append(contentsOf: bitLength.toBytesBigEndian().asSlice()); },
            .BigEndian128 => {
                // The upper 64 bits are zero for any message under 2 EiB.
                let high: UInt64 = 0;
                tail.append(contentsOf: high.toBytesBigEndian().asSlice());
                tail.append(contentsOf: bitLength.toBytesBigEndian().asSlice());
            },
            .LittleEndian64 => { tail.append(contentsOf: bitLength.toBytesLittleEndian().asSlice()); }
        }

        var offset: Int64 = 0;
        while offset < tail.count {
            function.compress(tail.asSlice().drop(first: offset).prefix(self.blockLength));
            offset = offset + self.blockLength;
        }
        function
    }
}
