module Aerie

/// One column of an island: solid from `bottom` to `top` inclusive, or
/// empty when `top < bottom`. `strength` is how far inside an island the
/// column is, 0 at the shore to 1 deep inland.
struct Column {
    var top: Int64
    var bottom: Int64
    var strength: Float32
}

/// The island generator. Islands come from a low-frequency mask: where it
/// rises past a threshold there's land, and the further past, the thicker
/// the island's keel. Each island also floats at its own altitude.
struct Generator {
    /// The seed everything else is derived from; saves store it.
    var seed: Int64
    var mask: Noise
    var detail: Noise

    init(seed: Int64) {
        self.seed = seed;
        self.mask = Noise(seed);
        self.detail = Noise(seed + 7919);
    }

    func column(x: Int64, z: Int64) -> Column {
        let fx = Float32(from: x);
        let fz = Float32(from: z);
        let m = self.mask.fractal(fx * 0.011, fz * 0.011, octaves: 4);
        let raw = (m - 0.12) * 2.5;
        if raw <= 0.0 {
            return Column(top: -1, bottom: 0, strength: 0.0);
        }
        let strength = if raw > 1.0 { 1.0 } else { raw };
        let altitude = 64.0 + 18.0 * self.detail.fractal(fx * 0.003 + 31.7, fz * 0.003 - 12.3, octaves: 2);
        let hills = 3.0 * self.detail.fractal(fx * 0.05, fz * 0.05, octaves: 2);
        let top = altitude + strength * 5.0 + hills;
        // A deep, rounded keel under the middle, ragged at the edges.
        let keel = strength * strength * 34.0 + 2.0 + 6.0 * self.mask.fractal(fx * 0.07 + 50.0, fz * 0.07 + 50.0, octaves: 2);
        Column(top: top.floor().toInt64() ?? 0, bottom: (top - keel).floor().toInt64() ?? 0, strength: strength)
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
        // Trees whose leaves reach into this chunk, clipped to it. Trees
        // are decided per column, so one on a chunk's edge comes out the
        // same from both sides.
        var z: Int64 = 0;
        while z < side {
            var x: Int64 = 0;
            while x < side {
                let c = columns(z * side + x);
                let height = treeHeight(x0 + x - treeReach, z0 + z - treeReach, c);
                if height > 0 {
                    out = placeTree(out, x - treeReach, c.top + 1 - y0, z - treeReach, height);
                }
                x = x + 1;
            }
            z = z + 1;
        }
        out
    }
}

/// How far a tree's leaves reach from its trunk.
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
        let depth = c.top - y;
        let block: Block = if depth == 0 {
            .Mossloam
        } else if depth <= 3 {
            .Loam
        } else if depth > 8 and scatter(x, y, z) < 30 {
            .Skyiron
        } else {
            .Driftstone
        };
        out.put(lx, y - y0, lz, block);
        y = y + 1;
    }
    if c.top + 1 <= y0 + chunkSize - 1 and c.top + 1 >= y0 and scatter(x, c.top + 1, z) < 8 {
        out.put(lx, c.top + 1 - y0, lz, .Glowcap);
    }
    out
}

/// Whether an emberwood tree grows on column (x, z), and its height.
func treeHeight(x: Int64, z: Int64, column: Column) -> Int64 {
    if column.top < column.bottom or column.strength < 0.35 {
        return 0;
    }
    let h = scatter(x, 0, z);
    if h >= 3 { 0 } else { 4 + h % 3 }
}

/// An emberwood tree with its trunk's base at chunk-local (x, y, z);
/// `put` clips whatever falls outside the chunk.
func placeTree(chunk: Chunk, x: Int64, y: Int64, z: Int64, height: Int64) -> Chunk {
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
                    out.put(x + dx, crown + dy, z + dz, .Emberleaf);
                }
                dx = dx + 1;
            }
            dz = dz + 1;
        }
        dy = dy + 1;
    }
    var t = y;
    while t <= crown {
        out.put(x, t, z, .Emberwood);
        t = t + 1;
    }
    out
}
