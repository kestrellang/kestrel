module Aerie

/// The blocks on the hotbar, keys 1 to 9.
let hotbar: [Block] = [.Driftstone, .Loam, .Mossloam, .Emberwood, .Planks, .Cloudglass, .Featherthatch, .Sailcloth, .Glowcap];
/// How far away blocks can be broken or placed.
let reach: Float32 = 6.0;

/// The block being broken, and how far along, 0 to 1: breaking takes
/// `Block.hardness` seconds of holding.
struct Breaking {
    var x: Int64
    var y: Int64
    var z: Int64
    var progress: Float32
}

extend Game {
    /// Break or place a block where the player is looking. Breaking
    /// takes holding: the block cracks over its `hardness` in seconds, and
    /// looking away or letting go starts over.
    mutating func useBlocks(dt: Float32) {
        let view = self.player.camera;
        let target = raycast(self.world, view.position, view.forward, reach);
        if self.controls.breakHeld {
            match target {
                .Some(hit) => { self.chip(hit.x, hit.y, hit.z, dt) },
                .None => { self.breaking = .None }
            }
            return;
        }
        self.breaking = .None;
        guard self.controls.placePressed, let .Some(hit) = target else {
            return;
        }
        // Don't wall the player in.
        if self.player.overlaps(hit.beforeX, hit.beforeY, hit.beforeZ) {
            return;
        }
        let block = hotbar(self.selected);
        if self.world.setBlock(hit.beforeX, hit.beforeY, hit.beforeZ, block) {
            self.remeshAround(hit.beforeX, hit.beforeY, hit.beforeZ, light: block.emits);
        }
    }

    /// Work at block (x, y, z) for `dt` seconds, knocking it out once
    /// its hardness is used up.
    mutating func chip(x: Int64, y: Int64, z: Int64, dt: Float32) {
        guard let .Some(block) = self.world.blockAt(x, y, z) else {
            return;
        }
        // Looking at a new block starts over.
        var b = match self.breaking {
            .Some(b) => if b.x == x and b.y == y and b.z == z { b } else { Breaking(x: x, y: y, z: z, progress: 0.0) },
            .None => Breaking(x: x, y: y, z: z, progress: 0.0)
        };
        b.progress = b.progress + dt / block.hardness;
        if b.progress < 1.0 {
            self.breaking = .Some(b);
            return;
        }
        self.breaking = .None;
        if self.world.setBlock(x, y, z, .Air) {
            self.remeshAround(x, y, z, light: block.emits);
        }
    }

    /// The birds, then the cracks on the block being broken (which cast no
    /// shadow), and how many of the boxes are birds.
    func models() -> ([ModelBox], Int64) {
        var boxes = self.flock.models(self.seconds);
        let birds = boxes.count;
        if let .Some(b) = self.breaking {
            boxes.append(crackBox(b.x, b.y, b.z, b.progress));
        }
        (boxes, birds)
    }
}
