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
        var i: Int64 = 0;
        while i < size {
            self.data.append(UInt8(from: (value >> (i * 8)) & 255));
            i = i + 1;
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
        if self.at + size > self.data.count {
            return .None;
        }
        var value: Int64 = 0;
        var i: Int64 = 0;
        while i < size {
            value = value | (Int64(from: self.data(self.at + i)) << (i * 8));
            i = i + 1;
        }
        self.at = self.at + size;
        .Some(value)
    }

    mutating func float() -> Float32? {
        let bits = (try self.int(size: 4));
        .Some(Float32(bitPattern: UInt32(from: bits)))
    }
}

/// `state` as uncompressed bytes.
func encodeSave(state: SaveState) -> [UInt8] {
    var out = ByteWriter();
    out.int(saveMagic, size: 4);
    out.int(saveVersion, size: 4);
    out.int(state.seed, size: 8);
    out.float(state.timeOfDay);
    out.float(state.position.x);
    out.float(state.position.y);
    out.float(state.position.z);
    out.float(state.yaw);
    out.float(state.pitch);
    out.int(state.chunks.count, size: 4);
    for (key, chunk) in state.chunks {
        out.int(key, size: 8);
        for id in chunk.blocks {
            out.int(Int64(from: id), size: 2);
        }
    }
    let bytes = out.data;
    bytes
}

/// The save in `data`, or `None` if it's truncated, from another version,
/// or not a save at all.
func decodeSave(data: [UInt8]) -> SaveState? {
    var input = ByteReader(data);
    let magic = (try input.int(size: 4));
    let version = (try input.int(size: 4));
    if magic != saveMagic or version != saveVersion {
        return .None;
    }
    let seed = (try input.int(size: 8));
    let timeOfDay = (try input.float());
    let position = Vec3(x: (try input.float()), y: (try input.float()), z: (try input.float()));
    let yaw = (try input.float());
    let pitch = (try input.float());
    let count = (try input.int(size: 4));
    var chunks: [(Int64, Chunk)] = [];
    var c: Int64 = 0;
    while c < count {
        let key = (try input.int(size: 8));
        var blocks: [UInt16] = [];
        var i: Int64 = 0;
        while i < chunkSize * chunkSize * chunkSize {
            blocks.append(UInt16(from: (try input.int(size: 2))));
            i = i + 1;
        }
        chunks.append((key, Chunk(blocks)));
        c = c + 1;
    }
    .Some(SaveState(seed: seed, timeOfDay: timeOfDay, position: position, yaw: yaw, pitch: pitch, chunks: chunks))
}
