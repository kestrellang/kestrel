module Aerie

import sdl.(monotonicMs)
import linalg.(Vec3)

// Chunk columns generate and mesh around the player a few milliseconds
// a frame, nearest first, and are dropped once far away.

/// The chunk column holding world position `v`, as (cx, cz).
func columnOf(v: Vec3) -> (Int64, Int64) {
    let size = Float32(from: chunkSize);
    ((v.x / size).floor().toInt64() ?? 0, (v.z / size).floor().toInt64() ?? 0)
}

/// The first column, nearest (cx, cz) first, within `radius` chunks for
/// which `want` says yes. Searches square rings outward.
func nearestColumn(cx: Int64, cz: Int64, radius: Int64, want: (Int64, Int64) -> Bool) -> (Int64, Int64)? {
    for d in 0..<(radius + 1) {
        // The ring's square, row by row, skipping the inside, which was
        // searched already.
        let side = d * 2 + 1;
        for i in 0..<(side * side) {
            let (dx, dz) = (i % side - d, i / side - d);
            if ring(dx, dz) == d and want(cx + dx, cz + dz) {
                return .Some((cx + dx, cz + dz));
            }
        }
    }
    .None
}

/// Which square ring around the player (dx, dz) is on.
func ring(dx: Int64, dz: Int64) -> Int64 {
    if dx.abs() > dz.abs() { dx.abs() } else { dz.abs() }
}

/// How many chunk columns around the player are drawn, by default and at
/// most; `AERIE_VIEW` picks within the range.
let defaultViewRadius: Int64 = 5;
let minViewRadius: Int64 = 2;
let maxViewRadius: Int64 = 16;
/// Milliseconds per frame spent generating and meshing.
let streamBudgetMs: Int64 = 6;

extend Game {
    /// Mesh the four chunks of column (cx, cz).
    mutating func meshColumn(cx: Int64, cz: Int64) {
        for cy in 0..<worldChunksY {
            let faces = if self.world.chunk(cx, cy, cz).isEmpty() { ChunkFaces() } else { self.world.padded(cx, cy, cz).mesh() };
            self.renderer.upload(cx, cy, cz, faces, self.world.lights(cx, cy, cz));
        }
        self.renderer.chunks.markColumn(cx, cz);
    }

    /// The nearest column still to generate, and the nearest one ready to
    /// mesh, around chunk column (pcx, pcz).
    func nextJobs(pcx: Int64, pcz: Int64) -> ((Int64, Int64)?, (Int64, Int64)?) {
        let radius = self.settings.viewRadius;
        let missing = nearestColumn(pcx, pcz, radius + 1, { (x: Int64, z: Int64) in not self.world.hasColumn(x, z) });
        let ready = nearestColumn(pcx, pcz, radius, { (x: Int64, z: Int64) in
            not self.renderer.chunks.hasColumn(x, z) and self.world.neighborsReady(x, z)
        });
        (missing, ready)
    }

    /// Generate and mesh the nearest missing columns for a few
    /// milliseconds, and forget far ones. Generation runs one ring further
    /// out than meshing, so every meshed chunk has all its neighbors.
    mutating func stream() {
        let (pcx, pcz) = columnOf(self.player.position);
        let start = unsafe monotonicMs();
        while unsafe monotonicMs() - start < streamBudgetMs {
            guard self.streamStep(pcx, pcz) else {
                break;
            }
        }
        self.world.unload(pcx, pcz, self.settings.viewRadius + 2);
        self.renderer.chunks.unload(pcx, pcz, keep: self.settings.viewRadius + 1);
    }

    /// Generate or mesh one column near chunk column (pcx, pcz): whichever
    /// is nearer, meshing on a tie, so land appears around the player first
    /// instead of after the whole area generates. False once there's
    /// nothing left to do.
    mutating func streamStep(pcx: Int64, pcz: Int64) -> Bool {
        match self.nextJobs(pcx, pcz) {
            (.None, .None) => { return false; },
            (.Some((x, z)), .None) => { self.world.generateColumn(x, z); },
            (.None, .Some((x, z))) => { self.meshColumn(x, z); },
            (.Some((gx, gz)), .Some((mx, mz))) => {
                if ring(mx - pcx, mz - pcz) <= ring(gx - pcx, gz - pcz) {
                    self.meshColumn(mx, mz);
                } else {
                    self.world.generateColumn(gx, gz);
                }
            }
        }
        true
    }

    /// Re-mesh every loaded chunk that block (x, y, z) shows up in: its own,
    /// and neighbors whose border or corner shading includes it. A glowing
    /// block changes the lights of every chunk it can reach.
    mutating func remeshAround(x: Int64, y: Int64, z: Int64, light light: Bool) {
        let spread = if light { (lightRadius.toInt64() ?? 10) + 1 } else { 1 };
        for (cx, cy, cz) in chunksAround(x, y, z, spread: spread) {
            if World.inRange(cy) and self.renderer.chunks.hasColumn(cx, cz) {
                self.renderer.upload(cx, cy, cz, self.world.padded(cx, cy, cz).mesh(), self.world.lights(cx, cy, cz));
            }
        }
    }
}

/// The chunks holding the 27 blocks `spread` or 0 blocks from (x, y, z)
/// along each axis, each once. `spread` is at most a chunk, so these are
/// all the chunks within `spread` of the block.
func chunksAround(x: Int64, y: Int64, z: Int64, spread spread: Int64) -> [(Int64, Int64, Int64)] {
    let offsets = [-spread, 0, spread];
    var seen: [Int64] = [];
    var found: [(Int64, Int64, Int64)] = [];
    for i in 0..<27 {
        let (cx, cy, cz) = (chunkOf(x + offsets(i % 3)), chunkOf(y + offsets(i / 3 % 3)), chunkOf(z + offsets(i / 9)));
        let key = chunkKey(cx, cy, cz);
        if not seen.contains(key) {
            seen.append(key);
            found.append((cx, cy, cz));
        }
    }
    found
}
