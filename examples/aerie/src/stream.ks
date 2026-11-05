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
    var d: Int64 = 0;
    while d <= radius {
        var dz = -d;
        while dz <= d {
            var dx = -d;
            while dx <= d {
                // Only the ring's edge; the inside was searched already.
                if (dx.abs() == d or dz.abs() == d) and want(cx + dx, cz + dz) {
                    return .Some((cx + dx, cz + dz));
                }
                dx = dx + 1;
            }
            dz = dz + 1;
        }
        d = d + 1;
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
        var cy: Int64 = 0;
        while cy < worldChunksY {
            let faces = if self.world.chunk(cx, cy, cz).isEmpty() { ChunkFaces() } else { meshChunk(self.world.padded(cx, cy, cz)) };
            self.renderer.upload(cx, cy, cz, faces, self.world.lights(cx, cy, cz));
            cy = cy + 1;
        }
        let _ = self.meshed.insert(chunkKey(cx, 0, cz), true);
    }

    /// The nearest column still to generate, and the nearest one ready to
    /// mesh, around chunk column (pcx, pcz).
    func nextJobs(pcx: Int64, pcz: Int64) -> ((Int64, Int64)?, (Int64, Int64)?) {
        let missing = nearestColumn(pcx, pcz, self.viewRadius + 1, { (x: Int64, z: Int64) in not self.world.hasColumn(x, z) });
        let ready = nearestColumn(pcx, pcz, self.viewRadius, { (x: Int64, z: Int64) in
            not self.meshed.contains(chunkKey(x, 0, z)) and self.world.neighborsReady(x, z)
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
            let (missing, ready) = self.nextJobs(pcx, pcz);
            // Whichever is nearer, meshing on a tie, so land appears around
            // the player first instead of after the whole area generates.
            match (missing, ready) {
                (.None, .None) => { break; },
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
        }
        self.world.unload(pcx, pcz, self.viewRadius + 2);
        let keep = self.viewRadius + 1;
        self.renderer.remove(where: { (key: Int64, x: Int64, z: Int64) in (x - pcx).abs() > keep or (z - pcz).abs() > keep });
        self.meshed.retain(where: { (key: Int64, done: Bool) in
            let (x, z) = columnOfKey(key);
            (x - pcx).abs() <= keep and (z - pcz).abs() <= keep
        });
    }

    /// Re-mesh every loaded chunk that block (x, y, z) shows up in: its own,
    /// and neighbors whose border or corner shading includes it. A glowing
    /// block changes the lights of every chunk it can reach.
    mutating func remeshAround(x: Int64, y: Int64, z: Int64, light light: Bool) {
        let spread = if light { (lightRadius.toInt64() ?? 10) + 1 } else { 1 };
        var done: [Int64] = [];
        var dy = -spread;
        while dy <= spread {
            var dz = -spread;
            while dz <= spread {
                var dx = -spread;
                while dx <= spread {
                    let (cx, cy, cz) = (chunkOf(x + dx), chunkOf(y + dy), chunkOf(z + dz));
                    let key = chunkKey(cx, cy, cz);
                    if World.inRange(cy) and not done.contains(key) and self.meshed.contains(chunkKey(cx, 0, cz)) {
                        done.append(key);
                        self.renderer.upload(cx, cy, cz, meshChunk(self.world.padded(cx, cy, cz)), self.world.lights(cx, cy, cz));
                    }
                    dx = dx + spread;
                }
                dz = dz + spread;
            }
            dy = dy + spread;
        }
    }
}
