module Aerie

import linalg.(Vec3, Mat4)

/// Half the player's width, their height, and their eyes' height, in blocks.
let playerHalfWidth: Float32 = 0.3;
let playerHeight: Float32 = 1.8;
let eyeHeight: Float32 = 1.62;

/// Speeds in blocks a second, and gravity in blocks a second squared.
let walkSpeed: Float32 = 5.0;
let flySpeed: Float32 = 12.0;
let jumpSpeed: Float32 = 8.0;
let gravity: Float32 = 26.0;
/// Soaring on the wing-cloak, like an elytra. A double jump launches you
/// `launchSpeed` blocks a second straight up (about 12 blocks high); each
/// jump while soaring pushes `boostSpeed` more along your view.
let launchSpeed: Float32 = 25.0;
let boostSpeed: Float32 = 12.0;
/// How fast diving gains speed and climbing loses it (gravity along your
/// path), and the air's drag, per second.
let soarGravity: Float32 = 16.0;
let soarDrag: Float32 = 0.12;
let maxSoarSpeed: Float32 = 45.0;
/// Below this speed the cloak loses lift and you sink.
let stallSpeed: Float32 = 7.0;
/// How quickly your path turns to follow your view, per second.
let soarTurn: Float32 = 3.0;
/// Below this height you've fallen into the cloud sea.
let seaLevel: Float32 = -40.0;

/// The player: feet at `position`, looking along `yaw` and `pitch` the way
/// `Camera` does. Walking has gravity and collision; a jump in midair opens
/// the wing-cloak and soars (see `boost`); flying (F) ignores both.
struct Player {
    var position: Vec3
    var velocity: Vec3
    var yaw: Float32
    var pitch: Float32
    var onGround: Bool
    /// Soaring on the wing-cloak, from a double jump until landing.
    var gliding: Bool
    /// Rising from the double jump's launch; the cloak catches the air at
    /// the top.
    var launching: Bool
    var flying: Bool
    var spawn: Vec3

    init(spawn: Vec3) {
        self.position = spawn;
        self.velocity = Vec3.zero;
        self.yaw = 0.0;
        self.pitch = -0.2;
        self.onGround = false;
        self.gliding = false;
        self.launching = false;
        self.flying = false;
        self.spawn = spawn;
    }

    var eye: Vec3 { self.position + Vec3(x: 0.0, y: eyeHeight, z: 0.0) }

    var camera: Camera { Camera(position: self.eye, yaw: self.yaw, pitch: self.pitch) }

    /// Level directions: where the player faces, and to their right.
    var ahead: Vec3 { Vec3(x: self.yaw.sin(), y: 0.0, z: -self.yaw.cos()) }
    var side: Vec3 { self.ahead.cross(Vec3.unitY) }

    /// The corners of the player's box with their feet at `at`.
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

    /// Turn by `yaw` and tilt by `pitch` radians, never past straight up
    /// or down.
    mutating func look(yaw yaw: Float32, pitch pitch: Float32) {
        let limit = 1.5;
        self.yaw = self.yaw + yaw;
        self.pitch = (self.pitch + pitch).clamp(-limit, limit);
    }

    /// Move for `dt` seconds as `controls` say, through `world`.
    mutating func update(controls: Controls, world: World, dt dt: Float32) {
        if self.flying {
            self.fly(controls, dt);
            return;
        }
        // Shift folds the cloak to drop.
        if self.gliding and controls.down {
            self.gliding = false;
            self.launching = false;
        }
        if self.gliding { self.soar(dt) } else { self.walk(controls, dt) }
        self.moveThrough(world, dt);
        // Fell into the cloud sea: back home.
        if self.position.y < seaLevel {
            self.position = self.spawn;
            self.velocity = Vec3.zero;
            self.gliding = false;
        }
    }

    /// A jump in midair: the first launches you high and opens the cloak;
    /// once soaring, each one pushes you along your view.
    mutating func boost() {
        guard not self.onGround and not self.flying else {
            return;
        }
        if self.gliding {
            self.velocity = self.velocity + self.camera.forward * boostSpeed;
        } else {
            self.velocity = Vec3(x: self.velocity.x, y: launchSpeed, z: self.velocity.z);
            self.gliding = true;
            self.launching = true;
        }
    }

    /// Where the movement keys point, along the ground, at most 1 long.
    func wish(controls: Controls) -> Vec3 {
        let wish = self.ahead * controls.forwardAxis + self.side * controls.strafeAxis;
        if wish.length > 1.0 { wish.normalized } else { wish }
    }

    /// Fly freely, through blocks, with no gravity.
    mutating func fly(controls: Controls, dt: Float32) {
        let lift = Vec3.unitY * controls.liftAxis;
        self.position = self.position + (self.wish(controls) + lift) * (flySpeed * dt);
        self.velocity = Vec3.zero;
    }

    /// Walk and jump under gravity.
    mutating func walk(controls: Controls, dt: Float32) {
        let vy = if self.onGround and controls.up { jumpSpeed } else { self.velocity.y };
        let horizontal = self.wish(controls) * walkSpeed;
        self.velocity = Vec3(x: horizontal.x, y: vy - gravity * dt, z: horizontal.z);
    }

    /// Soar like an elytra: your path turns to follow your view; diving
    /// gains speed and climbing spends it; too slow and you sink.
    mutating func soar(dt: Float32) {
        if self.launching {
            self.velocity.y = self.velocity.y - gravity * dt;
            self.launching = self.velocity.y > 0.0;
            return;
        }
        let look = self.camera.forward;
        let heading = if self.velocity.length > 0.01 { self.velocity.normalized } else { look };
        var speed = self.velocity.length;
        speed = speed - soarGravity * heading.y * dt - speed * soarDrag * dt;
        speed = speed.clamp(0.0, maxSoarSpeed);
        let turned = heading + (look - heading) * (soarTurn * dt).clamp(0.0, 1.0);
        var velocity = turned.normalized * speed;
        // Lift fades below stall speed, and gravity takes over.
        let stall = (1.0 - speed / stallSpeed).clamp(0.0, 1.0);
        velocity.y = velocity.y - gravity * stall * dt;
        self.velocity = velocity;
    }

    /// Move by the velocity for `dt` seconds, in small steps so a fast fall
    /// can't skip through a thin floor.
    mutating func moveThrough(world: World, dt: Float32) {
        let steps = 1 + (Int64(exactly: ((self.velocity * dt).length / 0.4).floor()) ?? 0);
        self.onGround = false;
        for _ in 0..<steps {
            self.step(world, self.velocity * (dt / Float32(from: steps)));
        }
    }

    /// Move by `delta`, landing on or bumping into whatever is in the way.
    mutating func step(world: World, delta: Vec3) {
        let hitY = self.moveAndCollide(world, delta);
        guard hitY else {
            return;
        }
        // Landing folds the cloak.
        if self.velocity.y < 0.0 {
            self.onGround = true;
            self.gliding = false;
            self.launching = false;
        }
        self.velocity = Vec3(x: self.velocity.x, y: 0.0, z: self.velocity.z);
    }

    /// Move by `delta` one axis at a time, stopping flush against blocks.
    /// Returns whether the move along y hit something.
    mutating func moveAndCollide(world: World, delta: Vec3) -> Bool {
        let gap: Float32 = 0.001;
        var p = Vec3(x: self.position.x + delta.x, y: self.position.y, z: self.position.z);
        if world.blocksBox(self.low(p), self.high(p)) {
            p.x = if delta.x > 0.0 { (p.x + playerHalfWidth).floor() - playerHalfWidth - gap } else { (p.x - playerHalfWidth).floor() + 1.0 + playerHalfWidth + gap };
        }
        p.y = p.y + delta.y;
        let hitY = world.blocksBox(self.low(p), self.high(p));
        if hitY {
            p.y = if delta.y > 0.0 { (p.y + playerHeight).floor() - playerHeight - gap } else { p.y.floor() + 1.0 };
        }
        p.z = p.z + delta.z;
        if world.blocksBox(self.low(p), self.high(p)) {
            p.z = if delta.z > 0.0 { (p.z + playerHalfWidth).floor() - playerHalfWidth - gap } else { (p.z - playerHalfWidth).floor() + 1.0 + playerHalfWidth + gap };
        }
        self.position = p;
        hitY
    }
}

/// A view from `position`, turned `yaw` radians from looking down -z and
/// tilted `pitch` radians up.
struct Camera {
    var position: Vec3
    var yaw: Float32
    var pitch: Float32

    var forward: Vec3 {
        Vec3(
            x: self.yaw.sin() * self.pitch.cos(),
            y: self.pitch.sin(),
            z: -self.yaw.cos() * self.pitch.cos()
        )
    }

    func view() -> Mat4 {
        Mat4.lookAt(eye: self.position, target: self.position + self.forward, up: Vec3.unitY)
    }
}
