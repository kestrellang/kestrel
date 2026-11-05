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
        for i in 0..<(side * side) {
            out.append(self.column(cx * chunkSize + i % side - treeReach, cz * chunkSize + i / side - treeReach));
        }
        out
    }

    /// The blocks of chunk (cx, cy, cz), given `columns(cx, cz)`.
    func chunk(cx: Int64, cy: Int64, cz: Int64, columns: [Column]) -> Chunk {
        var out = Chunk();
        let (x0, y0, z0) = (cx * chunkSize, cy * chunkSize, cz * chunkSize);
        let side = chunkSize + 2 * treeReach;
        for i in 0..<(chunkSize * chunkSize) {
            let (lx, lz) = (i % chunkSize, i / chunkSize);
            let c = columns((lz + treeReach) * side + lx + treeReach);
            out.fill(c, lx: lx, lz: lz, x: x0 + lx, y0: y0, z: z0 + lz);
        }
        // Trees and lair towers that reach into this chunk, clipped to it.
        // Both are decided per column, so one on a chunk's edge comes out
        // the same from both sides.
        for i in 0..<(side * side) {
            let (x, z) = (i % side, i / side);
            let c = columns(i);
            let (wx, wz) = (x0 + x - treeReach, z0 + z - treeReach);
            let (lx, ly, lz) = (x - treeReach, c.top + 1 - y0, z - treeReach);
            let height = c.treeHeight(wx, wz);
            if height > 0 {
                out.growTree(lx, ly, lz, height: height, lair: c.lair, wx: wx, wz: wz);
            }
            let tower = c.towerHeight(wx, wz);
            if tower > 0 {
                out.buildTower(lx, ly, lz, height: tower);
            }
        }
        out
    }

    /// A spot a little above and behind the island nearest the origin.
    func spawnPoint() -> Vec3 {
        for step in 0..<500 {
            let r = step * 4;
            let c = self.column(r, r / 2);
            if c.top >= c.bottom and c.strength > 0.6 {
                return Vec3(x: Float32(from: r) + 0.5, y: Float32(from: c.top + 2), z: Float32(from: r / 2) + 0.5);
            }
        }
        Vec3(x: 0.0, y: 90.0, z: 0.0)
    }
}

/// How far a tree's leaves, or a tower's walls, reach from its column.
let treeReach: Int64 = 3;

extend Column {
    var isLand: Bool { self.top >= self.bottom }

    /// The island's block `depth` below its surface at (x, y, z). Dreamy
    /// islands are moss over loam over driftstone, veined with skyiron;
    /// lairs are scorched soil, cracked with the odd molten pool, over
    /// ashrock.
    func block(depth: Int64, x: Int64, y: Int64, z: Int64) -> Block {
        if self.lair {
            if depth > 0 { return .Ashrock; }
            return if scatter(x, y, z) < 6 { .Embercore } else { .Scorchloam };
        }
        if depth == 0 { return .Mossloam; }
        if depth <= 3 { return .Loam; }
        if depth > 8 and scatter(x, y, z) < 30 { return .Skyiron; }
        .Driftstone
    }

    /// The height of the tree growing on this column at (x, z), or 0 for
    /// none.
    func treeHeight(x: Int64, z: Int64) -> Int64 {
        guard self.isLand and self.strength >= 0.35 else {
            return 0;
        }
        let h = scatter(x, 0, z);
        if h >= 3 { 0 } else { 4 + h % 3 }
    }

    /// The height of the lair tower standing on this column at (x, z), or
    /// 0 for none: rare, and only well inland on a lair island.
    func towerHeight(x: Int64, z: Int64) -> Int64 {
        guard self.lair and self.isLand and self.strength >= 0.7 else {
            return 0;
        }
        let h = scatter(x, 1, z);
        if h >= 2 { 0 } else { 10 + scatter(x, 2, z) % 7 }
    }
}

extend Chunk {
    /// Island blocks for `column`, at chunk-local (lx, lz) and world (x,
    /// z), in this chunk whose floor is at world height `y0`.
    mutating func fill(c: Column, lx lx: Int64, lz lz: Int64, x x: Int64, y0 y0: Int64, z z: Int64) {
        guard c.isLand and c.top >= y0 and c.bottom < y0 + chunkSize else {
            return;
        }
        let first = if c.bottom > y0 { c.bottom } else { y0 };
        let last = if c.top < y0 + chunkSize - 1 { c.top } else { y0 + chunkSize - 1 };
        for y in first..<(last + 1) {
            self.put(lx, y - y0, lz, c.block(c.top - y, x, y, z));
        }
        // A glowing plant here and there: glowcaps, or hexblooms in a lair.
        let above = c.top + 1;
        if above < y0 + chunkSize and above >= y0 and scatter(x, above, z) < 8 {
            self.put(lx, above - y0, lz, if c.lair { .Hexbloom } else { .Glowcap });
        }
    }

    /// A tree with its trunk's base at chunk-local (x, y, z); `put` clips
    /// whatever falls outside the chunk. Dreamy islands grow emberwood in
    /// blossom; lairs grow deadwood with sparse, ragged gloomleaf, decided
    /// per leaf from its world position (wx, wz are the trunk's).
    mutating func growTree(x: Int64, y: Int64, z: Int64, height height: Int64, lair lair: Bool, wx wx: Int64, wz wz: Int64) {
        let crown = y + height;
        // Leaves first, in a squashed ball, so the trunk overwrites the
        // middle: 7 across, 5 high.
        for i in 0..<(7 * 7 * 5) {
            let (dx, dz, dy) = (i % 7 - 3, i / 7 % 7 - 3, i / 49 - 2);
            let inBall = dx * dx + dz * dz + dy * dy * 2 <= 10;
            if inBall and (not lair or scatter(wx + dx, crown + dy, wz + dz) < 450) {
                self.put(x + dx, crown + dy, z + dz, if lair { .Gloomleaf } else { .Emberleaf });
            }
        }
        for t in y..<(crown + 1) {
            self.put(x, t, z, if lair { .Deadwood } else { .Emberwood });
        }
    }

    /// A dark brick spire, 5x5 and hollow, with its floor at chunk-local
    /// (x, y, z) and `height` storeys.
    mutating func buildTower(x: Int64, y: Int64, z: Int64, height height: Int64) {
        for i in 0..<(5 * 5 * (height + 2)) {
            let (dx, dz, dy) = (i % 5 - 2, i / 5 % 5 - 2, i / 25);
            if let .Some(block) = towerBlock(dx, dy, dz, height: height) {
                self.put(x + dx, y + dy, z + dz, block);
            }
        }
    }
}

/// What a tower `height` storeys tall has at (dx, dy, dz) from the middle
/// of its floor, or `None` to leave what's there: windows glowing with
/// embercore, a battlement with corner spikes, and a brazier burning on
/// its roof.
func towerBlock(dx: Int64, dy: Int64, dz: Int64, height height: Int64) -> Block? {
    let edge = dx.abs() == 2 or dz.abs() == 2;
    let corner = dx.abs() == 2 and dz.abs() == 2;
    // Spikes on the corners above the battlement.
    if dy == height + 1 {
        return if corner { .Some(.Darkbrick) } else { .None };
    }
    // The battlement: every other wall block, and the brazier.
    if dy == height {
        if dx == 0 and dz == 0 { return .Some(.Embercore); }
        return if edge and (dx + dz) % 2 == 0 { .Some(.Darkbrick) } else { .None };
    }
    // Floors at the bottom and under the roof; hollow inside.
    if dy == 0 or dy == height - 1 { return .Some(.Darkbrick); }
    if not edge { return .Some(.Air); }
    // Windows in the middle of each wall every four storeys.
    if dy % 4 == 2 and (dx == 0 or dz == 0) { .Some(.Embercore) } else { .Some(.Darkbrick) }
}
