module Aerie

/// Blocks per chunk edge. A chunk is `chunkSize`³ blocks.
let chunkSize: Int64 = 32;

/// A cube of blocks, stored as block ids in one flat array, x fastest,
/// then z, then y (so a horizontal slice is contiguous).
struct Chunk {
    var blocks: [UInt16]
    /// Indices of the blocks that glow, kept up to date by `put`, so finding
    /// the lights near a chunk doesn't scan every block.
    var emitters: [Int64]

    init() {
        self.blocks = Array(repeating: Block.Air.id, count: chunkSize * chunkSize * chunkSize);
        self.emitters = [];
    }

    /// A chunk from saved block ids, finding its emitters again.
    init(blocks: [UInt16]) {
        self.blocks = blocks;
        self.emitters = [];
        for i in 0..<blocks.count {
            if Block.from(blocks(i)).emits { self.emitters.append(i); }
        }
    }

    static func contains(x: Int64, y: Int64, z: Int64) -> Bool {
        x >= 0 and y >= 0 and z >= 0 and x < chunkSize and y < chunkSize and z < chunkSize
    }

    static func index(x: Int64, y: Int64, z: Int64) -> Int64 {
        (y * chunkSize + z) * chunkSize + x
    }

    /// The block at (x, y, z), or air outside the chunk.
    func block(x: Int64, y: Int64, z: Int64) -> Block {
        if not Chunk.contains(x, y, z) {
            return .Air;
        }
        Block.from(self.blocks(Chunk.index(x, y, z)))
    }

    /// Whether every block is air.
    func isEmpty() -> Bool {
        for id in self.blocks {
            if id != 0 { return false; }
        }
        true
    }

    /// Set (x, y, z) to `block`; outside the chunk does nothing.
    mutating func put(x: Int64, y: Int64, z: Int64, block: Block) {
        if not Chunk.contains(x, y, z) {
            return;
        }
        let i = Chunk.index(x, y, z);
        if Block.from(self.blocks(i)).emits {
            let _ = self.emitters.remove(i);
        }
        self.blocks(i) = block.id;
        if block.emits {
            self.emitters.append(i);
        }
    }
}

/// A chunk plus a one-block border taken from its 26 neighbors, so the
/// mesher can look across edges without finding the neighbor chunk for
/// every lookup. Coordinates run from -1 to `chunkSize`.
struct Padded {
    var blocks: [UInt16]

    static var size: Int64 { chunkSize + 2 }

    static func index(x: Int64, y: Int64, z: Int64) -> Int64 {
        ((y + 1) * Padded.size + (z + 1)) * Padded.size + (x + 1)
    }

    func block(x: Int64, y: Int64, z: Int64) -> Block {
        if x < -1 or y < -1 or z < -1 or x > chunkSize or y > chunkSize or z > chunkSize {
            return .Air;
        }
        Block.from(self.blocks(Padded.index(x, y, z)))
    }

    /// From a 3x3x3 block of chunks around the middle one, ordered as
    /// `neighborhood()` is.
    static func around(chunks: [Chunk]) -> Padded {
        let side = Padded.size;
        var blocks: [UInt16] = [];
        blocks.reserveCapacity(side * side * side);
        // x fastest, then z, then y, as `index` lays them out.
        for i in 0..<(side * side * side) {
            let (sx, lx) = split(i % side - 1);
            let (sz, lz) = split(i / side % side - 1);
            let (sy, ly) = split(i / (side * side) - 1);
            blocks.append(chunks(neighborIndex(sx, sy, sz)).blocks(Chunk.index(lx, ly, lz)));
        }
        Padded(blocks: blocks)
    }
}

/// Which neighbor a padded coordinate (-1 to `chunkSize`) falls in, as -1,
/// 0 or 1, and its coordinate inside that chunk.
func split(v: Int64) -> (Int64, Int64) {
    if v < 0 { return (-1, v + chunkSize); }
    if v >= chunkSize { return (1, v - chunkSize); }
    (0, v)
}

/// Where neighbor (dx, dy, dz), each -1 to 1, sits in `neighborhood()`.
func neighborIndex(dx: Int64, dy: Int64, dz: Int64) -> Int64 {
    ((dy + 1) * 3 + (dz + 1)) * 3 + (dx + 1)
}

/// A cheap, repeatable hash of a block position to 0..<1000, for scattering
/// ore and plants.
func scatter(x: Int64, y: Int64, z: Int64) -> Int64 {
    let h = (x * 73856093) ^ (y * 19349663) ^ (z * 83492791);
    let m = h % 1000;
    if m < 0 { m + 1000 } else { m }
}

/// The 27 offsets of a 3x3x3 block of chunks, each -1 to 1, ordered by y,
/// then z, then x.
func neighborhood() -> [(Int64, Int64, Int64)] {
    var offsets: [(Int64, Int64, Int64)] = [];
    for i in 0..<27 {
        offsets.append((i % 3 - 1, i / 9 - 1, i / 3 % 3 - 1));
    }
    offsets
}
