module crypto.digest

/// SHA-1 (FIPS 180-4). Produces a 20-byte digest.
///
/// **Legacy only.** SHA-1 collisions are practical (SHAttered, 2017), so
/// never use it for signatures, certificates or anything that must resist
/// a forger. It's here because protocols still require it: the WebSocket
/// handshake (`Sec-WebSocket-Accept`), TOTP's default HMAC (RFC 6238), and
/// git object ids. HMAC-SHA1 is still considered secure as a MAC.
public struct SHA1: Digest {
    var blocks: BlockBuffer[SHA1State];

    public static var digestSize: Int64 { 20 }
    public static var blockSize: Int64 { 64 }

    public init() {
        let initial: [UInt32] = [
            0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0
        ];
        self.blocks = BlockBuffer(function: SHA1State(words: initial), blockLength: 64);
    }

    public mutating func update[S](bytes: S) where S: Slice[UInt8] {
        self.blocks.update(bytes.asSlice());
    }

    public func finalize() -> DigestOutput {
        let words = self.blocks.finishedWithLengthPadding(.BigEndian64).words;
        var result: [UInt8] = [];
        for i in 0..<5 {
            result.append(contentsOf: words(i).toBytesBigEndian().asSlice());
        }
        DigestOutput(bytes: result)
    }

    // --- internals ---

    static func compress(block: ArraySlice[UInt8], mutating state: Array[UInt32]) {
        // Message schedule: 16 big-endian words, expanded to 80.
        var w = Array[UInt32](repeating: 0, count: 80);
        for i in 0..<16 {
            let off = i * 4;
            let b0 = UInt32(from: block(off));
            let b1 = UInt32(from: block(off + 1));
            let b2 = UInt32(from: block(off + 2));
            let b3 = UInt32(from: block(off + 3));
            w(i) = (b0 << 24) | (b1 << 16) | (b2 << 8) | b3;
        }
        for i in 16..<80 {
            w(i) = (w(i - 3) ^ w(i - 8) ^ w(i - 14) ^ w(i - 16)).rotateLeft(by: 1);
        }

        var a = state(0);
        var b = state(1);
        var c = state(2);
        var d = state(3);
        var e = state(4);

        for i in 0..<80 {
            var f: UInt32 = 0;
            var k: UInt32 = 0;
            if i < 20 {
                f = (b & c) | (!b & d);
                k = 0x5a827999;
            } else if i < 40 {
                f = b ^ c ^ d;
                k = 0x6ed9eba1;
            } else if i < 60 {
                f = (b & c) | (b & d) | (c & d);
                k = 0x8f1bbcdc;
            } else {
                f = b ^ c ^ d;
                k = 0xca62c1d6;
            }
            let temp = a.rotateLeft(by: 5) + f + e + k + w(i);
            e = d;
            d = c;
            c = b.rotateLeft(by: 30);
            b = a;
            a = temp;
        }

        state(0) = state(0) + a;
        state(1) = state(1) + b;
        state(2) = state(2) + c;
        state(3) = state(3) + d;
        state(4) = state(4) + e;
    }
}

/// SHA1's chaining state, compressed one block at a time by `BlockBuffer`.
struct SHA1State: BlockFunction {
    var words: Array[UInt32];

    init(words words: Array[UInt32]) {
        self.words = words;
    }

    mutating func compress(block: ArraySlice[UInt8]) {
        SHA1.compress(block, self.words);
    }
}
