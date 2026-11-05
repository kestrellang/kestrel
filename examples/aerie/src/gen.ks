module Aerie

import linalg.(Vec3)

/// One column of an island: solid from `bottom` to `top` inclusive, or
/// empty when `top < bottom`. `strength` is how far inside an island the
/// column is, 0 at the shore to 1 deep inland. `lair` is whether the column
/// lies in an evil lair region: true for about half the archipelago.
struct Column {
    var top: Int64
    var bottom: Int64
    var strength: Float32
    var lair: Bool
}

/// The island generator. Islands come from a low-frequency mask: where it
/// rises past a threshold there's land, and the further past, the thicker
/// the island's keel. Each island also floats at its own altitude. An even
/// lower-frequency theme field splits the archipelago into two realms:
/// the dreamy islands, and evil lairs of ash, dark brick and ember.
struct Generator {
    /// The seed everything else is derived from; saves store it.
    var seed: Int64
    var mask: Noise
    var detail: Noise
    var theme: Noise

    init(seed: Int64) {
        self.seed = seed;
        self.mask = Noise(seed);
        self.detail = Noise(seed + 7919);
        self.theme = Noise(seed + 104729);
    }

    /// How deep into lair country (x, z) is: 0 well inside the dreamy
    /// realm to 1 well inside a lair region, ramping across a narrow border.
    /// The field's scale is a few islands wide, so most islands fall wholly
    /// on one side.
    func lairness(x: Float32, z: Float32) -> Float32 {
        let n = self.theme.fractal(x * 0.0035 + 11.1, z * 0.0035 - 7.7, octaves: 2);
        (n * 8.0 + 0.5).clamp(0.0, 1.0)
    }

    func column(x: Int64, z: Int64) -> Column {
        let fx = Float32(from: x);
        let fz = Float32(from: z);
        let m = self.mask.fractal(fx * 0.011, fz * 0.011, octaves: 4);
        let raw = (m - 0.12) * 2.5;
        let lair = self.lairness(fx, fz) >= 0.5;
        if raw <= 0.0 {
            return Column(top: -1, bottom: 0, strength: 0.0, lair: lair);
        }
        let strength = if raw > 1.0 { 1.0 } else { raw };
        let altitude = 64.0 + 18.0 * self.detail.fractal(fx * 0.003 + 31.7, fz * 0.003 - 12.3, octaves: 2);
        let hills = 3.0 * self.detail.fractal(fx * 0.05, fz * 0.05, octaves: 2);
        let top = altitude + strength * 5.0 + hills;
        // A deep, rounded keel under the middle, ragged at the edges. Lairs
        // hang jagged spikes from theirs.
        var keel = strength * strength * 34.0 + 2.0 + 6.0 * self.mask.fractal(fx * 0.07 + 50.0, fz * 0.07 + 50.0, octaves: 2);
        if lair {
            let spike = self.detail.fractal(fx * 0.31 + 5.0, fz * 0.31 - 9.0, octaves: 1);
            keel = keel + strength * 22.0 * (spike * 2.0).clamp(0.0, 1.0);
        }
        Column(top: top.floor().toInt64() ?? 0, bottom: (top - keel).floor().toInt64() ?? 0, strength: strength, lair: lair)
    }

    /// The columns covering chunk column (cx, cz) plus a `treeReach`
    /// border, x fastest. Shared by all its chunks, so each column's noise
    /// is evaluated once.
    func columns(cx: Int64, cz: Int64) -> [Column] {
        let side = chunkSize + 2 * treeReach;
        var out: [Column] = [];
        out.reserveCapacity(side * side);
        var z: Int64 = 0;
        while z < side {
            var x: Int64 = 0;
            while x < side {
                out.append(self.column(cx * chunkSize + x - treeReach, cz * chunkSize + z - treeReach));
                x = x + 1;
            }
            z = z + 1;
        }
        out
    }

    /// The blocks of chunk (cx, cy, cz), given `columns(cx, cz)`.
    func chunk(cx: Int64, cy: Int64, cz: Int64, columns: [Column]) -> Chunk {
        var out = Chunk();
        let x0 = cx * chunkSize;
        let y0 = cy * chunkSize;
        let z0 = cz * chunkSize;
        let side = chunkSize + 2 * treeReach;
        var lz: Int64 = 0;
        while lz < chunkSize {
            var lx: Int64 = 0;
            while lx < chunkSize {
                let c = columns((lz + treeReach) * side + lx + treeReach);
                out = fillColumn(out, c, lx, lz, x0 + lx, y0, z0 + lz);
                lx = lx + 1;
            }
            lz = lz + 1;
        }
        // Trees and lair towers that reach into this chunk, clipped to it.
        // Both are decided per column, so one on a chunk's edge comes out
        // the same from both sides.
        var z: Int64 = 0;
        while z < side {
            var x: Int64 = 0;
            while x < side {
                let c = columns(z * side + x);
                let (wx, wz) = (x0 + x - treeReach, z0 + z - treeReach);
                let height = treeHeight(wx, wz, c);
                if height > 0 {
                    out = placeTree(out, x - treeReach, c.top + 1 - y0, z - treeReach, height, lair: c.lair, wx: wx, wz: wz);
                }
                let tower = towerHeight(wx, wz, c);
                if tower > 0 {
                    out = placeTower(out, x - treeReach, c.top + 1 - y0, z - treeReach, tower);
                }
                x = x + 1;
            }
            z = z + 1;
        }
        out
    }
}

/// How far a tree's leaves, or a tower's walls, reach from its column.
let treeReach: Int64 = 3;

/// Island blocks for one column of a chunk whose floor is at `y0`.
func fillColumn(chunk: Chunk, c: Column, lx: Int64, lz: Int64, x: Int64, y0: Int64, z: Int64) -> Chunk {
    var out = chunk;
    if c.top < c.bottom or c.top < y0 or c.bottom >= y0 + chunkSize {
        return out;
    }
    var y = if c.bottom > y0 { c.bottom } else { y0 };
    let last = if c.top < y0 + chunkSize - 1 { c.top } else { y0 + chunkSize - 1 };
    while y <= last {
        let block = if c.lair { lairBlock(c.top - y, x, y, z) } else { dreamBlock(c.top - y, x, y, z) };
        out.put(lx, y - y0, lz, block);
        y = y + 1;
    }
    // A glowing plant here and there: glowcaps, or hexblooms in a lair.
    if c.top + 1 <= y0 + chunkSize - 1 and c.top + 1 >= y0 and scatter(x, c.top + 1, z) < 8 {
        out.put(lx, c.top + 1 - y0, lz, if c.lair { .Hexbloom } else { .Glowcap });
    }
    out
}

/// A dreamy island's block `depth` below its surface at (x, y, z).
func dreamBlock(depth: Int64, x: Int64, y: Int64, z: Int64) -> Block {
    if depth == 0 { return .Mossloam; }
    if depth <= 3 { return .Loam; }
    if depth > 8 and scatter(x, y, z) < 30 { return .Skyiron; }
    .Driftstone
}

/// A lair island's block `depth` below its surface: scorched soil cracked
/// with the odd molten pool, over ashrock.
func lairBlock(depth: Int64, x: Int64, y: Int64, z: Int64) -> Block {
    if depth == 0 {
        return if scatter(x, y, z) < 6 { .Embercore } else { .Scorchloam };
    }
    .Ashrock
}

/// Whether an emberwood tree grows on column (x, z), and its height.
func treeHeight(x: Int64, z: Int64, column: Column) -> Int64 {
    if column.top < column.bottom or column.strength < 0.35 {
        return 0;
    }
    let h = scatter(x, 0, z);
    if h >= 3 { 0 } else { 4 + h % 3 }
}

/// A tree with its trunk's base at chunk-local (x, y, z); `put` clips
/// whatever falls outside the chunk. Dreamy islands grow emberwood in
/// blossom; lairs grow deadwood with sparse, ragged gloomleaf, decided per
/// leaf from its world position (wx, wz are the trunk's).
func placeTree(chunk: Chunk, x: Int64, y: Int64, z: Int64, height: Int64, lair lair: Bool, wx wx: Int64, wz wz: Int64) -> Chunk {
    var out = chunk;
    let crown = y + height;
    // Leaves first, so the trunk overwrites the middle.
    var dy: Int64 = -2;
    while dy <= 2 {
        var dz: Int64 = -3;
        while dz <= 3 {
            var dx: Int64 = -3;
            while dx <= 3 {
                if dx * dx + dz * dz + dy * dy * 2 <= 10 {
                    if not lair {
                        out.put(x + dx, crown + dy, z + dz, .Emberleaf);
                    } else if scatter(wx + dx, crown + dy, wz + dz) < 450 {
                        out.put(x + dx, crown + dy, z + dz, .Gloomleaf);
                    }
                }
                dx = dx + 1;
            }
            dz = dz + 1;
        }
        dy = dy + 1;
    }
    var t = y;
    while t <= crown {
        out.put(x, t, z, if lair { .Deadwood } else { .Emberwood });
        t = t + 1;
    }
    out
}

/// Whether a lair tower stands on column (x, z), and its height: rare, and
/// only well inland on a lair island.
func towerHeight(x: Int64, z: Int64, column: Column) -> Int64 {
    if not column.lair or column.top < column.bottom or column.strength < 0.7 {
        return 0;
    }
    let h = scatter(x, 1, z);
    if h >= 2 { 0 } else { 10 + scatter(x, 2, z) % 7 }
}

/// A dark brick spire, 5x5 and hollow, with its floor at chunk-local
/// (x, y, z) and `height` storeys: windows glowing with embercore, a
/// battlement with corner spikes, and a brazier burning on its roof.
func placeTower(chunk: Chunk, x: Int64, y: Int64, z: Int64, height: Int64) -> Chunk {
    var out = chunk;
    var dy: Int64 = 0;
    while dy <= height + 1 {
        var dz: Int64 = -2;
        while dz <= 2 {
            var dx: Int64 = -2;
            while dx <= 2 {
                let edge = dx.abs() == 2 or dz.abs() == 2;
                let corner = dx.abs() == 2 and dz.abs() == 2;
                let block: Block? = if dy == height + 1 {
                    // Spikes on the corners above the battlement.
                    if corner { .Some(.Darkbrick) } else { .None }
                } else if dy == height {
                    // Battlement: every other wall block, and the brazier.
                    if dx == 0 and dz == 0 { .Some(.Embercore) } else if edge and (dx + dz) % 2 == 0 { .Some(.Darkbrick) } else { .None }
                } else if dy == 0 or dy == height - 1 {
                    .Some(.Darkbrick)
                } else if edge {
                    // Windows in the middle of each wall every four storeys.
                    if dy % 4 == 2 and (dx == 0 or dz == 0) { .Some(.Embercore) } else { .Some(.Darkbrick) }
                } else {
                    .Some(.Air)
                };
                if let .Some(b) = block {
                    out.put(x + dx, y + dy, z + dz, b);
                }
                dx = dx + 1;
            }
            dz = dz + 1;
        }
        dy = dy + 1;
    }
    out
}

/// A spot a little above and behind the island nearest the origin.
func spawnPoint(generator: Generator) -> Vec3 {
    var r: Int64 = 0;
    while r < 2000 {
        let c = generator.column(r, r / 2);
        if c.top >= c.bottom and c.strength > 0.6 {
            return Vec3(x: Float32(from: r) + 0.5, y: Float32(from: c.top + 2), z: Float32(from: r / 2) + 0.5);
        }
        r = r + 4;
    }
    Vec3(x: 0.0, y: 90.0, z: 0.0)
}
