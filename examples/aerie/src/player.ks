module Aerie

import linalg.(Vec3)

/// Half the player's width, their height, and their eyes' height, in blocks.
let playerHalfWidth: Float32 = 0.3;
let playerHeight: Float32 = 1.8;
let eyeHeight: Float32 = 1.62;

let walkSpeed: Float32 = 5.0;
let flySpeed: Float32 = 12.0;
let jumpSpeed: Float32 = 8.0;
let gravity: Float32 = 26.0;
/// While gliding: the fastest you sink, and how fast you sail forward.
let glideSink: Float32 = 2.2;
let glideSpeed: Float32 = 10.0;

func floorInt(v: Float32) -> Int64 {
    v.floor().toInt64() ?? 0
}

/// Whether the player can't pass through (x, y, z). Unloaded chunks count
/// as solid, so nobody falls through land that hasn't streamed in yet.
func blocksPlayer(world: World, x: Int64, y: Int64, z: Int64) -> Bool {
    match world.blockAt(x, y, z) {
        .None => true,
        .Some(block) => match block.shape {
            .Solid => true,
            .Clear => true,
            _ => false
        }
    }
}

/// Whether any block in the box from `low` to `high` blocks the player.
func boxBlocked(world: World, low: Vec3, high: Vec3) -> Bool {
    let epsilon: Float32 = 0.0001;
    var y = floorInt(low.y);
    while y <= floorInt(high.y - epsilon) {
        var z = floorInt(low.z);
        while z <= floorInt(high.z - epsilon) {
            var x = floorInt(low.x);
            while x <= floorInt(high.x - epsilon) {
                if blocksPlayer(world, x, y, z) { return true; }
                x = x + 1;
            }
            z = z + 1;
        }
        y = y + 1;
    }
    false
}

/// The player: feet at `position`, looking along `yaw` and `pitch` the way
/// `Camera` does. Walking has gravity and collision; holding jump while
/// falling opens the wing-cloak and glides; flying (F) ignores both.
struct Player {
    var position: Vec3
    var velocity: Vec3
    var yaw: Float32
    var pitch: Float32
    var onGround: Bool
    var gliding: Bool
    var flying: Bool
    var spawn: Vec3

    init(spawn: Vec3) {
        self.position = spawn;
        self.velocity = Vec3.zero;
        self.yaw = 0.0;
        self.pitch = -0.2;
        self.onGround = false;
        self.gliding = false;
        self.flying = false;
        self.spawn = spawn;
    }

    var eye: Vec3 { self.position + Vec3(x: 0.0, y: eyeHeight, z: 0.0) }

    var camera: Camera { Camera(position: self.eye, yaw: self.yaw, pitch: self.pitch) }

    func low(at: Vec3) -> Vec3 { Vec3(x: at.x - playerHalfWidth, y: at.y, z: at.z - playerHalfWidth) }
    func high(at: Vec3) -> Vec3 { Vec3(x: at.x + playerHalfWidth, y: at.y + playerHeight, z: at.z + playerHalfWidth) }

    /// Whether the player's box would overlap block (x, y, z).
    func overlaps(x: Int64, y: Int64, z: Int64) -> Bool {
        let low = self.low(self.position);
        let high = self.high(self.position);
        Float32(from: x) < high.x and Float32(from: x + 1) > low.x
            and Float32(from: y) < high.y and Float32(from: y + 1) > low.y
            and Float32(from: z) < high.z and Float32(from: z + 1) > low.z
    }

    /// Move by `delta` one axis at a time, stopping flush against blocks.
    /// Returns which axes hit something, as (x, y, z).
    mutating func moveAndCollide(world: World, delta: Vec3) -> (Bool, Bool, Bool) {
        let gap: Float32 = 0.001;
        var p = self.position;
        var hits = (false, false, false);

        p = Vec3(x: p.x + delta.x, y: p.y, z: p.z);
        if boxBlocked(world, self.low(p), self.high(p)) {
            let edge = if delta.x > 0.0 { (p.x + playerHalfWidth).floor() - playerHalfWidth - gap } else { (p.x - playerHalfWidth).floor() + 1.0 + playerHalfWidth + gap };
            p = Vec3(x: edge, y: p.y, z: p.z);
            hits.0 = true;
        }
        p = Vec3(x: p.x, y: p.y + delta.y, z: p.z);
        if boxBlocked(world, self.low(p), self.high(p)) {
            let edge = if delta.y > 0.0 { (p.y + playerHeight).floor() - playerHeight - gap } else { p.y.floor() + 1.0 };
            p = Vec3(x: p.x, y: edge, z: p.z);
            hits.1 = true;
        }
        p = Vec3(x: p.x, y: p.y, z: p.z + delta.z);
        if boxBlocked(world, self.low(p), self.high(p)) {
            let edge = if delta.z > 0.0 { (p.z + playerHalfWidth).floor() - playerHalfWidth - gap } else { (p.z - playerHalfWidth).floor() + 1.0 + playerHalfWidth + gap };
            p = Vec3(x: p.x, y: p.y, z: edge);
            hits.2 = true;
        }
        self.position = p;
        hits
    }

    mutating func update(controls: Controls, world: World, dt dt: Float32) {
        self.yaw = self.yaw + axis(controls.turnRight, controls.turnLeft) * 1.8 * dt;
        let limit = 1.5;
        self.pitch = (self.pitch + axis(controls.lookUp, controls.lookDown) * 1.4 * dt).clamp(-limit, limit);
        let ahead = Vec3(x: self.yaw.sin(), y: 0.0, z: -self.yaw.cos());
        let side = ahead.cross(Vec3.unitY);
        var wish = ahead * axis(controls.forward, controls.back) + side * axis(controls.right, controls.left);
        if wish.length > 1.0 { wish = wish.normalized; }

        if self.flying {
            let lift = Vec3.unitY * axis(controls.up, controls.down);
            self.position = self.position + (wish + lift) * (flySpeed * dt);
            self.velocity = Vec3.zero;
            return;
        }

        // Jump from the ground; hold it while falling to glide.
        var vy = self.velocity.y;
        if self.onGround and controls.up {
            vy = jumpSpeed;
        }
        vy = vy - gravity * dt;
        self.gliding = not self.onGround and controls.up and vy < 0.0;
        var horizontal = wish * walkSpeed;
        if self.gliding {
            if vy < -glideSink { vy = -glideSink; }
            // The cloak carries you forward even without pressing W.
            horizontal = ahead * glideSpeed + side * (axis(controls.right, controls.left) * 3.0);
        }
        self.velocity = Vec3(x: horizontal.x, y: vy, z: horizontal.z);

        // Small steps, so a fast fall can't skip through a thin floor.
        let steps = 1 + floorInt((self.velocity * dt).length / 0.4);
        let stepDt = dt / Float32(from: steps);
        self.onGround = false;
        var i: Int64 = 0;
        while i < steps {
            let (_, hitY, _) = self.moveAndCollide(world, self.velocity * stepDt);
            if hitY {
                if self.velocity.y < 0.0 { self.onGround = true; }
                self.velocity = Vec3(x: self.velocity.x, y: 0.0, z: self.velocity.z);
            }
            i = i + 1;
        }

        // Fell into the cloud sea: back home.
        if self.position.y < -40.0 {
            self.position = self.spawn;
            self.velocity = Vec3.zero;
        }
    }
}

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

/// The first non-air block along the ray from `origin` in `direction`
/// (normalized), within `reach` blocks. Steps cell by cell (Amanatides and
/// Woo), so no block is skipped.
func raycast(world: World, origin: Vec3, direction: Vec3, reach: Float32) -> RayHit? {
    var x = floorInt(origin.x);
    var y = floorInt(origin.y);
    var z = floorInt(origin.z);
    let stepX: Int64 = if direction.x > 0.0 { 1 } else { -1 };
    let stepY: Int64 = if direction.y > 0.0 { 1 } else { -1 };
    let stepZ: Int64 = if direction.z > 0.0 { 1 } else { -1 };
    let far: Float32 = 1000000.0;
    // Ray distance between cell walls on each axis, and to the first wall.
    let deltaX = if direction.x == 0.0 { far } else { (1.0 / direction.x).abs() };
    let deltaY = if direction.y == 0.0 { far } else { (1.0 / direction.y).abs() };
    let deltaZ = if direction.z == 0.0 { far } else { (1.0 / direction.z).abs() };
    var nextX = if direction.x > 0.0 { (Float32(from: x) + 1.0 - origin.x) * deltaX } else { (origin.x - Float32(from: x)) * deltaX };
    var nextY = if direction.y > 0.0 { (Float32(from: y) + 1.0 - origin.y) * deltaY } else { (origin.y - Float32(from: y)) * deltaY };
    var nextZ = if direction.z > 0.0 { (Float32(from: z) + 1.0 - origin.z) * deltaZ } else { (origin.z - Float32(from: z)) * deltaZ };
    var travelled: Float32 = 0.0;
    while travelled <= reach {
        let (px, py, pz) = (x, y, z);
        if nextX < nextY and nextX < nextZ {
            x = x + stepX;
            travelled = nextX;
            nextX = nextX + deltaX;
        } else if nextY < nextZ {
            y = y + stepY;
            travelled = nextY;
            nextY = nextY + deltaY;
        } else {
            z = z + stepZ;
            travelled = nextZ;
            nextZ = nextZ + deltaZ;
        }
        if travelled > reach {
            return .None;
        }
        match world.blockAt(x, y, z) {
            .Some(.Air) => {},
            .Some(_) => {
                return .Some(RayHit(x: x, y: y, z: z, beforeX: px, beforeY: py, beforeZ: pz));
            },
            .None => { return .None; }
        }
    }
    .None
}
