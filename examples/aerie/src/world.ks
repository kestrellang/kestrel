module Aerie

import linalg.(Vec3)

/// How far a glowing block's light reaches, in blocks.
let lightRadius: Float32 = 10.0;
/// The most lights one chunk is shaded by.
let maxLights: Int64 = 32;

/// Chunks stacked in each column: the world is 128 blocks tall.
let worldChunksY: Int64 = 4;

/// One Int64 per chunk position, for dictionary keys. Horizontal chunk
/// coordinates must stay within ±2^20 (about 33 million blocks).
func chunkKey(cx: Int64, cy: Int64, cz: Int64) -> Int64 {
    ((cx + 1048576) * 2097152 + (cz + 1048576)) * 16 + cy
}

/// The column (cx, cz) a `chunkKey` belongs to.
func columnOfKey(key: Int64) -> (Int64, Int64) {
    let column = key / 16;
    (column / 2097152 - 1048576, column % 2097152 - 1048576)
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
    /// Loaded chunks the player has changed, by key. These are what a save
    /// keeps; untouched chunks are regenerated from the seed.
    var edited: Set[Int64]
    /// Changed chunks that aren't loaded: unloaded ones, and those read from
    /// a save. Generating their column takes them back instead.
    var stored: Dictionary[Int64, Chunk]

    init(seed: Int64) {
        self.chunks = Dictionary[Int64, Chunk]();
        self.generator = Generator(seed);
        self.air = Chunk();
        self.edited = Set[Int64]();
        self.stored = Dictionary[Int64, Chunk]();
    }

    /// Every changed chunk, loaded or not, for saving.
    func editedChunks() -> [(Int64, Chunk)] {
        var out: [(Int64, Chunk)] = [];
        for (key, chunk) in self.stored {
            out.append((key, chunk));
        }
        for key in self.edited {
            if let .Some(chunk) = self.chunks(key) {
                out.append((key, chunk));
            }
        }
        out
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
        let _ = self.edited.insert(key);
        true
    }

    /// Generate and keep the whole column of chunks at (cx, cz), taking
    /// back any stored changed chunks instead of generating them.
    mutating func generateColumn(cx: Int64, cz: Int64) {
        let columns = self.generator.columns(cx, cz);
        var cy: Int64 = 0;
        while cy < worldChunksY {
            let key = chunkKey(cx, cy, cz);
            match self.stored.remove(key) {
                .Some(chunk) => {
                    let _ = self.chunks.insert(key, chunk);
                    let _ = self.edited.insert(key);
                },
                .None => {
                    let _ = self.chunks.insert(key, self.generator.chunk(cx, cy, cz, columns));
                }
            }
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

    /// Up to `maxLights` glowing blocks that can reach chunk (cx, cy, cz),
    /// nearest its middle first, as world positions of their centers.
    func lights(cx: Int64, cy: Int64, cz: Int64) -> [Vec3] {
        let size = Float32(from: chunkSize);
        let middle = Vec3(x: (Float32(from: cx) + 0.5) * size, y: (Float32(from: cy) + 0.5) * size, z: (Float32(from: cz) + 0.5) * size);
        // Inside the chunk's box grown by the light radius.
        let low = middle - Vec3(splat: size * 0.5 + lightRadius);
        let high = middle + Vec3(splat: size * 0.5 + lightRadius);
        var found: [(Float32, Vec3)] = [];
        var dy: Int64 = -1;
        while dy <= 1 {
            var dz: Int64 = -1;
            while dz <= 1 {
                var dx: Int64 = -1;
                while dx <= 1 {
                    let (nx, ny, nz) = (cx + dx, cy + dy, cz + dz);
                    for i in self.chunk(nx, ny, nz).emitters {
                        let p = Vec3(
                            x: Float32(from: nx * chunkSize + i % chunkSize) + 0.5,
                            y: Float32(from: ny * chunkSize + i / (chunkSize * chunkSize)) + 0.5,
                            z: Float32(from: nz * chunkSize + (i / chunkSize) % chunkSize) + 0.5
                        );
                        if p.x >= low.x and p.y >= low.y and p.z >= low.z and p.x <= high.x and p.y <= high.y and p.z <= high.z {
                            found.append(((p - middle).lengthSquared, p));
                        }
                    }
                    dx = dx + 1;
                }
                dz = dz + 1;
            }
            dy = dy + 1;
        }
        found.sort(by: { (a: (Float32, Vec3), b: (Float32, Vec3)) in a.0 < b.0 });
        var out: [Vec3] = [];
        for (_, p) in found {
            if out.count < maxLights { out.append(p); }
        }
        out
    }

    /// Forget columns further than `radius` chunks from (cx, cz), moving
    /// changed chunks to `stored` so their changes survive.
    mutating func unload(cx: Int64, cz: Int64, radius: Int64) {
        let near = { (key: Int64) in
            let (kx, kz) = columnOfKey(key);
            (kx - cx).abs() <= radius and (kz - cz).abs() <= radius
        };
        var leaving: [Int64] = [];
        for key in self.edited {
            if not near(key) { leaving.append(key); }
        }
        for key in leaving {
            if let .Some(chunk) = self.chunks(key) {
                let _ = self.stored.insert(key, chunk);
            }
            let _ = self.edited.remove(key);
        }
        self.chunks.retain(where: { (key: Int64, chunk: Chunk) in near(key) });
    }
}
