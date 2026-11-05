module Aerie

import wgpu.(WgpuError)
import sdl.(Event, Key, monotonicMs)
import linalg.(Vec3, Mat4)

/// Which movement keys are held.
struct Controls {
    var forward: Bool = false
    var back: Bool = false
    var left: Bool = false
    var right: Bool = false
    var up: Bool = false
    var down: Bool = false
    var turnLeft: Bool = false
    var turnRight: Bool = false
    var lookUp: Bool = false
    var lookDown: Bool = false

    init() {}

    mutating func setKey(key: Key, held held: Bool) {
        match key {
            .W => { self.forward = held },
            .S => { self.back = held },
            .A => { self.left = held },
            .D => { self.right = held },
            .Space => { self.up = held },
            .LeftShift => { self.down = held },
            .LeftArrow => { self.turnLeft = held },
            .RightArrow => { self.turnRight = held },
            .UpArrow => { self.lookUp = held },
            .DownArrow => { self.lookDown = held },
            _ => {}
        }
    }
}

func axis(positive: Bool, negative: Bool) -> Float32 {
    (if positive { 1.0 } else { 0.0 }) - (if negative { 1.0 } else { 0.0 })
}

/// A free-flying camera at `position`, turned `yaw` radians from looking
/// down -z and tilted `pitch` radians up. Milestone 4 replaces flying with
/// walking and gliding.
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

    mutating func update(controls: Controls, dt dt: Float32) {
        self.yaw = self.yaw + axis(controls.turnRight, controls.turnLeft) * 1.8 * dt;
        let limit = 1.5;
        self.pitch = (self.pitch + axis(controls.lookUp, controls.lookDown) * 1.4 * dt).clamp(-limit, limit);
        let ahead = Vec3(x: self.yaw.sin(), y: 0.0, z: -self.yaw.cos());
        let side = ahead.cross(Vec3.unitY);
        let move = ahead * axis(controls.forward, controls.back)
            + side * axis(controls.right, controls.left)
            + Vec3.unitY * axis(controls.up, controls.down);
        self.position = self.position + move * (10.0 * dt);
    }

    func view() -> Mat4 {
        Mat4.lookAt(eye: self.position, target: self.position + self.forward, up: Vec3.unitY)
    }
}

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

/// A spot a little above and behind the island nearest the origin.
func spawnPoint(generator: Generator) -> Vec3 {
    var r: Int64 = 0;
    while r < 2000 {
        let c = generator.column(r, r / 2);
        if c.top >= c.bottom and c.strength > 0.6 {
            return Vec3(x: Float32(from: r), y: Float32(from: c.top + 30), z: Float32(from: r / 2 + 60));
        }
        r = r + 4;
    }
    Vec3(x: 0.0, y: 90.0, z: 0.0)
}

/// How many chunk columns around the player are drawn.
let viewRadius: Int64 = 5;
/// Milliseconds per frame spent generating and meshing.
let streamBudgetMs: Int64 = 6;

/// The game, driven one event and one frame at a time. Nothing here owns
/// the main loop, so a browser build can call `frame` from
/// `requestAnimationFrame` instead of a `while` loop.
struct Game: not Copyable {
    var renderer: Renderer
    var world: World
    /// Columns whose chunks are meshed, by `chunkKey(cx, 0, cz)`.
    var meshed: Dictionary[Int64, Bool]
    var camera: Camera
    var controls: Controls
    var seconds: Float32
    var running: Bool

    init(consuming renderer: Renderer, seed seed: Int64) {
        self.renderer = renderer;
        self.renderer.viewDistance = Float32(from: viewRadius * chunkSize);
        self.world = World(seed);
        self.meshed = Dictionary[Int64, Bool]();
        self.camera = Camera(position: spawnPoint(self.world.generator), yaw: 0.0, pitch: -0.35);
        self.controls = Controls();
        self.seconds = 0.0;
        self.running = true;
    }

    mutating func event(event: Event) {
        match event {
            .Quit => { self.running = false },
            .KeyDown(.Escape) => { self.running = false },
            .KeyDown(key) => { self.controls.setKey(key, held: true) },
            .KeyUp(key) => { self.controls.setKey(key, held: false) },
            _ => {}
        }
    }

    /// Mesh the four chunks of column (cx, cz).
    mutating func meshColumn(cx: Int64, cz: Int64) {
        var cy: Int64 = 0;
        while cy < worldChunksY {
            let faces = if self.world.chunk(cx, cy, cz).isEmpty() { [] } else { meshChunk(self.world.padded(cx, cy, cz)) };
            self.renderer.upload(cx, cy, cz, faces);
            cy = cy + 1;
        }
        let _ = self.meshed.insert(chunkKey(cx, 0, cz), true);
    }

    /// The nearest column still to generate, and the nearest one ready to
    /// mesh, around chunk column (pcx, pcz).
    func nextJobs(pcx: Int64, pcz: Int64) -> ((Int64, Int64)?, (Int64, Int64)?) {
        let missing = nearestColumn(pcx, pcz, viewRadius + 1, { (x: Int64, z: Int64) in not self.world.hasColumn(x, z) });
        let ready = nearestColumn(pcx, pcz, viewRadius, { (x: Int64, z: Int64) in
            not self.meshed.contains(chunkKey(x, 0, z)) and self.world.neighborsReady(x, z)
        });
        (missing, ready)
    }

    /// Generate and mesh the nearest missing columns for a few
    /// milliseconds, and forget far ones. Generation runs one ring further
    /// out than meshing, so every meshed chunk has all its neighbors.
    mutating func stream() {
        let (pcx, pcz) = columnOf(self.camera.position);
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
        self.world.unload(pcx, pcz, viewRadius + 2);
        let keep = viewRadius + 1;
        self.renderer.remove(where: { (key: Int64, x: Int64, z: Int64) in (x - pcx).abs() > keep or (z - pcz).abs() > keep });
        self.meshed.retain(where: { (key: Int64, done: Bool) in
            let column = key / 16;
            let x = column / 2097152 - 1048576;
            let z = column % 2097152 - 1048576;
            (x - pcx).abs() <= keep and (z - pcz).abs() <= keep
        });
    }

    mutating func frame(dt: Float32) -> () throws WgpuError {
        self.seconds = self.seconds + dt;
        self.camera.update(self.controls, dt: dt);
        self.stream();
        // A low sun that slowly circles.
        let angle = 0.6 + self.seconds * 0.05;
        let sun = Vec3(x: angle.cos(), y: -0.7, z: angle.sin()).normalized;
        try self.renderer.draw(self.camera.view(), eye: self.camera.position, sun: sun);
        .Ok(())
    }
}
