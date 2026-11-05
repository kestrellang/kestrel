module Aerie

import linalg.(Vec3)

/// What a ray hit: the block, and the empty cell it passed through just
/// before (where a placed block goes).
struct RayHit {
    var x: Int64
    var y: Int64
    var z: Int64
    var beforeX: Int64
    var beforeY: Int64
    var beforeZ: Int64
}

/// A ray's progress along one axis, cell wall by cell wall.
struct RayAxis {
    /// The cell the ray is in, and which way it moves: -1 or 1.
    var cell: Int64
    var step: Int64
    /// Distance along the ray between walls on this axis, and to the next.
    var delta: Float32
    var next: Float32

    /// Along `direction` from `origin`, one axis of each.
    init(origin origin: Float32, direction direction: Float32) {
        self.cell = origin.floor().toInt64() ?? 0;
        self.step = if direction > 0.0 { 1 } else { -1 };
        self.delta = if direction == 0.0 { 1000000.0 } else { (1.0 / direction).abs() };
        let wall = Float32(from: self.cell);
        self.next = if direction > 0.0 { (wall + 1.0 - origin) * self.delta } else { (origin - wall) * self.delta };
    }

    /// Cross the next wall; returns how far along the ray it is.
    mutating func advance() -> Float32 {
        let travelled = self.next;
        self.cell = self.cell + self.step;
        self.next = self.next + self.delta;
        travelled
    }
}

extend World {
    /// The first non-air block along the ray from `origin` in `direction`
    /// (normalized), within `reach` blocks. Steps cell by cell (Amanatides
    /// and Woo), so no block is skipped.
    func raycast(origin: Vec3, direction: Vec3, reach: Float32) -> RayHit? {
        var x = RayAxis(origin: origin.x, direction: direction.x);
        var y = RayAxis(origin: origin.y, direction: direction.y);
        var z = RayAxis(origin: origin.z, direction: direction.z);
        var travelled: Float32 = 0.0;
        while travelled <= reach {
            let (px, py, pz) = (x.cell, y.cell, z.cell);
            // Cross whichever wall comes first.
            travelled = if x.next < y.next and x.next < z.next { x.advance() } else if y.next < z.next { y.advance() } else { z.advance() };
            guard travelled <= reach, let .Some(block) = self.blockAt(x.cell, y.cell, z.cell) else {
                return .None;
            }
            if not block.isAir {
                return .Some(RayHit(x: x.cell, y: y.cell, z: z.cell, beforeX: px, beforeY: py, beforeZ: pz));
            }
        }
        .None
    }

    /// Whether the player can't pass through (x, y, z). Unloaded chunks
    /// count as solid, so nobody falls through land that hasn't streamed in
    /// yet.
    func blocksPlayer(x: Int64, y: Int64, z: Int64) -> Bool {
        match self.blockAt(x, y, z) {
            .None => true,
            .Some(block) => match block.shape {
                .Solid => true,
                .Clear => true,
                _ => false
            }
        }
    }

    /// Whether any block in the box from `low` to `high` blocks the player.
    func blocksBox(low: Vec3, high: Vec3) -> Bool {
        // Cells the box touches; a box ending exactly on a wall doesn't
        // reach into the next cell.
        let epsilon: Float32 = 0.0001;
        let first = (low.x.floor().toInt64() ?? 0, low.y.floor().toInt64() ?? 0, low.z.floor().toInt64() ?? 0);
        let last = ((high.x - epsilon).floor().toInt64() ?? 0, (high.y - epsilon).floor().toInt64() ?? 0, (high.z - epsilon).floor().toInt64() ?? 0);
        let (sx, sy, sz) = (last.0 - first.0 + 1, last.1 - first.1 + 1, last.2 - first.2 + 1);
        for i in 0..<(sx * sy * sz) {
            if self.blocksPlayer(first.0 + i % sx, first.1 + i / (sx * sz), first.2 + i / sx % sz) {
                return true;
            }
        }
        false
    }
}
