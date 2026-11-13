module Aerie

import linalg.(Vec3)

// Save files: everything the player changed, so the generator can rebuild
// the rest from the seed. The layout is a flat little-endian stream, which
// the driver zlib-compresses before writing:
//
//   magic "AERI", version, seed, time of day, player position, yaw, pitch,
//   chunk count, then for each edited chunk its key and its block ids.
//
// Encoding and decoding work on bytes only, so a browser build can keep
// saves in IndexedDB instead of files.

/// "AERI" read as a little-endian UInt32.
let saveMagic: Int64 = 1230128449;
/// Bump when the layout changes; older files are then refused.
let saveVersion: Int64 = 1;

/// What a save file holds.
struct SaveState {
    var seed: Int64
    var timeOfDay: Float32
    var position: Vec3
    var yaw: Float32
    var pitch: Float32
    /// Edited chunks, by `chunkKey`.
    var chunks: [(Int64, Chunk)]
}

/// Appends little-endian values to a byte array.
struct ByteWriter {
    var data: [UInt8]

    init() {
        self.data = [];
    }

    /// The low `size` bytes of `value`.
    mutating func int(value: Int64, size size: Int64) {
        for i in 0..<size {
            self.data.append(UInt8(from: (value >> (i * 8)) & 255));
        }
    }

    mutating func float(value: Float32) {
        self.int(Int64(from: value.bitPattern), size: 4);
    }
}

/// Reads little-endian values back; every read fails past the end.
struct ByteReader {
    var data: [UInt8]
    var at: Int64

    init(data: [UInt8]) {
        self.data = data;
        self.at = 0;
    }

    /// `size` bytes as an unsigned value; 8 bytes round-trip any Int64.
    mutating func int(size size: Int64) -> Int64? {
        guard self.at + size <= self.data.count else {
            return .None;
        }
        var value: Int64 = 0;
        for i in 0..<size {
            value = value | (Int64(from: self.data(self.at + i)) << (i * 8));
        }
        self.at = self.at + size;
        .Some(value)
    }

    mutating func float() -> Float32? {
        let bits = (try self.int(size: 4));
        .Some(Float32(bitPattern: UInt32(from: bits)))
    }
}

extend SaveState {
    /// The state as uncompressed bytes.
    func encode() -> [UInt8] {
        var out = ByteWriter();
        out.int(saveMagic, size: 4);
        out.int(saveVersion, size: 4);
        out.int(self.seed, size: 8);
        out.float(self.timeOfDay);
        out.float(self.position.x);
        out.float(self.position.y);
        out.float(self.position.z);
        out.float(self.yaw);
        out.float(self.pitch);
        out.int(self.chunks.count, size: 4);
        for (key, chunk) in self.chunks {
            out.int(key, size: 8);
            for id in chunk.blocks {
                out.int(Int64(from: id), size: 2);
            }
        }
        out.data
    }

    /// The save in `data`, or `None` if it's truncated, from another
    /// version, or not a save at all.
    static func decode(data: [UInt8]) -> SaveState? {
        var input = ByteReader(data);
        let magic = (try input.int(size: 4));
        let version = (try input.int(size: 4));
        guard magic == saveMagic and version == saveVersion else {
            return .None;
        }
        let seed = (try input.int(size: 8));
        let timeOfDay = (try input.float());
        let position = Vec3(x: (try input.float()), y: (try input.float()), z: (try input.float()));
        let yaw = (try input.float());
        let pitch = (try input.float());
        let count = (try input.int(size: 4));
        var chunks: [(Int64, Chunk)] = [];
        for _ in 0..<count {
            let key = (try input.int(size: 8));
            chunks.append((key, Chunk((try input.chunkBlocks()))));
        }
        .Some(SaveState(seed: seed, timeOfDay: timeOfDay, position: position, yaw: yaw, pitch: pitch, chunks: chunks))
    }
}

extend ByteReader {
    /// One chunk's block ids, or `None` if the data runs out.
    mutating func chunkBlocks() -> [UInt16]? {
        var blocks: [UInt16] = [];
        blocks.reserveCapacity(chunkSize * chunkSize * chunkSize);
        for _ in 0..<(chunkSize * chunkSize * chunkSize) {
            blocks.append(UInt16(from: (try self.int(size: 2))));
        }
        .Some(blocks)
    }
}
