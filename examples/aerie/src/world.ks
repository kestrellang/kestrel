module Aerie

/// Chunks stacked in each column: the world is 128 blocks tall.
let worldChunksY: Int64 = 4;

/// One Int64 per chunk position, for dictionary keys. Horizontal chunk
/// coordinates must stay within ±2^20 (about 33 million blocks).
func chunkKey(cx: Int64, cy: Int64, cz: Int64) -> Int64 {
    ((cx + 1048576) * 2097152 + (cz + 1048576)) * 16 + cy
}

/// `v / chunkSize`, rounded down (toward -infinity, unlike `/`).
func chunkOf(v: Int64) -> Int64 {
    if v >= 0 { v / chunkSize } else { (v + 1) / chunkSize - 1 }
}

/// The generated chunks around the player. Chunks above and below the
/// world's height are always air and never stored.
struct World {
    var chunks: Dictionary[Int64, Chunk]
    var generator: Generator
    var air: Chunk

    init(seed: Int64) {
        self.chunks = Dictionary[Int64, Chunk]();
        self.generator = Generator(seed);
        self.air = Chunk();
    }

    static func inRange(cy: Int64) -> Bool {
        cy >= 0 and cy < worldChunksY
    }

    func has(cx: Int64, cy: Int64, cz: Int64) -> Bool {
        not World.inRange(cy) or self.chunks.contains(chunkKey(cx, cy, cz))
    }

    func chunk(cx: Int64, cy: Int64, cz: Int64) -> Chunk {
        if not World.inRange(cy) {
            return self.air;
        }
        self.chunks(chunkKey(cx, cy, cz)) ?? self.air
    }

    /// The block at world (x, y, z): air above and below the world, and
    /// `None` where the chunk isn't loaded.
    func blockAt(x: Int64, y: Int64, z: Int64) -> Block? {
        let (cx, cy, cz) = (chunkOf(x), chunkOf(y), chunkOf(z));
        if not World.inRange(cy) {
            return .Some(.Air);
        }
        match self.chunks(chunkKey(cx, cy, cz)) {
            .Some(chunk) => .Some(chunk.block(x - cx * chunkSize, y - cy * chunkSize, z - cz * chunkSize)),
            .None => .None
        }
    }

    /// Set the block at world (x, y, z). False if it's outside the world or
    /// its chunk isn't loaded.
    mutating func setBlock(x: Int64, y: Int64, z: Int64, block: Block) -> Bool {
        let (cx, cy, cz) = (chunkOf(x), chunkOf(y), chunkOf(z));
        let key = chunkKey(cx, cy, cz);
        if not World.inRange(cy) or not self.chunks.contains(key) {
            return false;
        }
        let _ = self.chunks.modify(key) { (chunk) in
            chunk.put(x - cx * chunkSize, y - cy * chunkSize, z - cz * chunkSize, block);
        };
        true
    }

    /// Generate and keep the whole column of chunks at (cx, cz).
    mutating func generateColumn(cx: Int64, cz: Int64) {
        let columns = self.generator.columns(cx, cz);
        var cy: Int64 = 0;
        while cy < worldChunksY {
            let _ = self.chunks.insert(chunkKey(cx, cy, cz), self.generator.chunk(cx, cy, cz, columns));
            cy = cy + 1;
        }
    }

    func hasColumn(cx: Int64, cz: Int64) -> Bool {
        self.chunks.contains(chunkKey(cx, 0, cz))
    }

    /// Whether every chunk touching column (cx, cz) exists, so it can be
    /// meshed without seams.
    func neighborsReady(cx: Int64, cz: Int64) -> Bool {
        var dz: Int64 = -1;
        while dz <= 1 {
            var dx: Int64 = -1;
            while dx <= 1 {
                if not self.hasColumn(cx + dx, cz + dz) { return false; }
                dx = dx + 1;
            }
            dz = dz + 1;
        }
        true
    }

    /// Chunk (cx, cy, cz) with its neighbors' border, for the mesher.
    func padded(cx: Int64, cy: Int64, cz: Int64) -> Padded {
        var around: [Chunk] = [];
        var dy: Int64 = -1;
        while dy <= 1 {
            var dz: Int64 = -1;
            while dz <= 1 {
                var dx: Int64 = -1;
                while dx <= 1 {
                    around.append(self.chunk(cx + dx, cy + dy, cz + dz));
                    dx = dx + 1;
                }
                dz = dz + 1;
            }
            dy = dy + 1;
        }
        Padded.around(around)
    }

    /// Forget columns further than `radius` chunks from (cx, cz).
    mutating func unload(cx: Int64, cz: Int64, radius: Int64) {
        self.chunks.retain(where: { (key: Int64, chunk: Chunk) in
            let column = key / 16;
            let kx = column / 2097152 - 1048576;
            let kz = column % 2097152 - 1048576;
            (kx - cx).abs() <= radius and (kz - cz).abs() <= radius
        });
    }
}
