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
        var i: Int64 = 0;
        while i < blocks.count {
            if Block.from(blocks(i)).emits { self.emitters.append(i); }
            i = i + 1;
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

    /// From a 3x3x3 block of chunks around the middle one, ordered by y,
    /// then z, then x (see `neighborIndex`).
    static func around(chunks: [Chunk]) -> Padded {
        var blocks: [UInt16] = [];
        blocks.reserveCapacity(Padded.size * Padded.size * Padded.size);
        var y: Int64 = -1;
        while y <= chunkSize {
            let (sy, ly) = split(y);
            var z: Int64 = -1;
            while z <= chunkSize {
                let (sz, lz) = split(z);
                var x: Int64 = -1;
                while x <= chunkSize {
                    let (sx, lx) = split(x);
                    blocks.append(chunks(neighborIndex(sx, sy, sz)).blocks(Chunk.index(lx, ly, lz)));
                    x = x + 1;
                }
                z = z + 1;
            }
            y = y + 1;
        }
        Padded(blocks: blocks)
    }

    /// One chunk with air all around it.
    static func alone(chunk: Chunk) -> Padded {
        var chunks: [Chunk] = [];
        var i: Int64 = 0;
        while i < 27 {
            chunks.append(if i == 13 { chunk } else { Chunk() });
            i = i + 1;
        }
        Padded.around(chunks)
    }
}

/// Which neighbor a padded coordinate (-1 to `chunkSize`) falls in, as -1,
/// 0 or 1, and its coordinate inside that chunk.
func split(v: Int64) -> (Int64, Int64) {
    if v < 0 { return (-1, v + chunkSize); }
    if v >= chunkSize { return (1, v - chunkSize); }
    (0, v)
}

/// Where neighbor (dx, dy, dz), each -1 to 1, sits in a 27-chunk list.
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

/// A hand-built test island for milestone 1: a rounded top of mossloam over
/// loam, tapering to a driftstone point underneath with skyiron in it, an
/// emberwood tree, a few glowcaps and a small thatched hut. Milestone 3
/// replaces this with noise-based generation.
func testIsland() -> Chunk {
    var chunk = Chunk();
    let center: Float32 = 15.5;
    let radius: Float32 = 13.0;
    var z: Int64 = 0;
    while z < chunkSize {
        var x: Int64 = 0;
        while x < chunkSize {
            let dx = Float32(from: x) - center;
            let dz = Float32(from: z) - center;
            let r = (dx * dx + dz * dz).sqrt() / radius;
            if r < 1.0 {
                // Gentle hills on top; a deep, rounded keel below.
                let hills = 1.5 * (Float32(from: x) * 0.45).sin() * (Float32(from: z) * 0.35).cos();
                let top = 20 + ((hills + 2.0 * (1.0 - r)).floor().toInt64() ?? 0);
                let depth = 3 + ((14.0 * (1.0 - r * r)).floor().toInt64() ?? 0);
                var y = top - depth;
                while y <= top {
                    let block: Block = if y == top {
                        .Mossloam
                    } else if y >= top - 2 {
                        .Loam
                    } else if scatter(x, y, z) < 40 {
                        .Skyiron
                    } else {
                        .Driftstone
                    };
                    chunk.put(x, y, z, block);
                    y = y + 1;
                }
                if r < 0.8 and scatter(x, top + 1, z) < 25 {
                    chunk.put(x, top + 1, z, .Glowcap);
                }
            }
            x = x + 1;
        }
        z = z + 1;
    }
    chunk = addTree(chunk, x: 9, z: 18);
    addHut(chunk, x: 17, z: 10)
}

/// The y of the highest block in column (x, z), or -1 if it's empty.
func surface(chunk: Chunk, x: Int64, z: Int64) -> Int64 {
    var y = chunkSize - 1;
    while y >= 0 {
        match chunk.block(x, y, z) {
            .Air => {},
            .Glowcap => {},
            _ => { return y; }
        };
        y = y - 1;
    }
    return -1;
}

/// An emberwood tree standing on column (x, z).
func addTree(chunk: Chunk, x x: Int64, z z: Int64) -> Chunk {
    var out = chunk;
    let base = surface(out, x, z) + 1;
    let height: Int64 = 5;
    // Leaves first, so the trunk overwrites the middle.
    var dy: Int64 = -2;
    while dy <= 2 {
        var dz: Int64 = -3;
        while dz <= 3 {
            var dx: Int64 = -3;
            while dx <= 3 {
                let d = dx * dx + dz * dz + dy * dy * 2;
                if d <= 10 and scatter(x + dx, base + height + dy, z + dz) < 900 {
                    out.put(x + dx, base + height + dy, z + dz, .Emberleaf);
                }
                dx = dx + 1;
            }
            dz = dz + 1;
        }
        dy = dy + 1;
    }
    var y = base;
    while y < base + height + 1 {
        out.put(x, y, z, .Emberwood);
        y = y + 1;
    }
    out
}

/// A 5x5 hut of planks with a sailcloth door, cloudglass windows and a
/// featherthatch roof, its floor at the surface of column (x, z).
func addHut(chunk: Chunk, x x: Int64, z z: Int64) -> Chunk {
    var out = chunk;
    let floor = surface(out, x + 2, z + 2);
    var dz: Int64 = 0;
    while dz < 5 {
        var dx: Int64 = 0;
        while dx < 5 {
            out.put(x + dx, floor, z + dz, .Planks);
            let wall = dx == 0 or dx == 4 or dz == 0 or dz == 4;
            var dy: Int64 = 1;
            while dy <= 3 {
                let block: Block = if not wall {
                    .Air
                } else if dz == 4 and dx == 2 and dy <= 2 {
                    .Sailcloth
                } else if dy == 2 and (dx == 2 or dz == 2) {
                    .Cloudglass
                } else {
                    .Planks
                };
                out.put(x + dx, floor + dy, z + dz, block);
                dy = dy + 1;
            }
            dx = dx + 1;
        }
        dz = dz + 1;
    }
    // A stepped roof, one ring smaller per level.
    var level: Int64 = 0;
    while level < 3 {
        var rz = level - 1;
        while rz <= 5 - level {
            var rx = level - 1;
            while rx <= 5 - level {
                out.put(x + rx, floor + 4 + level, z + rz, .Featherthatch);
                rx = rx + 1;
            }
            rz = rz + 1;
        }
        level = level + 1;
    }
    out
}
