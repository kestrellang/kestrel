module Aerie

import wgpu.(WgpuError)
import sdl.(Event, Key, monotonicMs)
import linalg.(Vec3, Vec4, Mat4)

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
    /// Presses waiting for the next frame: break, place, toggle flying,
    /// and a hotbar slot (or -1).
    var breakPressed: Bool = false
    var placePressed: Bool = false
    var flyPressed: Bool = false
    var slotPressed: Int64 = -1

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
            .Q => { if held { self.breakPressed = true } },
            .E => { if held { self.placePressed = true } },
            .F => { if held { self.flyPressed = true } },
            .Digit1 => { if held { self.slotPressed = 0 } },
            .Digit2 => { if held { self.slotPressed = 1 } },
            .Digit3 => { if held { self.slotPressed = 2 } },
            .Digit4 => { if held { self.slotPressed = 3 } },
            .Digit5 => { if held { self.slotPressed = 4 } },
            .Digit6 => { if held { self.slotPressed = 5 } },
            .Digit7 => { if held { self.slotPressed = 6 } },
            .Digit8 => { if held { self.slotPressed = 7 } },
            _ => {}
        }
    }

    /// Forget presses once a frame has handled them.
    mutating func clearPresses() {
        self.breakPressed = false;
        self.placePressed = false;
        self.flyPressed = false;
        self.slotPressed = -1;
    }
}

func axis(positive: Bool, negative: Bool) -> Float32 {
    (if positive { 1.0 } else { 0.0 }) - (if negative { 1.0 } else { 0.0 })
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
            return Vec3(x: Float32(from: r) + 0.5, y: Float32(from: c.top + 2), z: Float32(from: r / 2) + 0.5);
        }
        r = r + 4;
    }
    Vec3(x: 0.0, y: 90.0, z: 0.0)
}

/// The blocks on the hotbar, keys 1 to 8.
let hotbar: [Block] = [.Driftstone, .Loam, .Mossloam, .Emberwood, .Planks, .Cloudglass, .Featherthatch, .Sailcloth];
/// How far away blocks can be broken or placed.
let reach: Float32 = 6.0;

/// The crosshair and hotbar for a `width` by `height` screen.
func hud(width: Int64, height: Int64, selected: Int64, gliding: Bool) -> [OverlayQuad] {
    var quads: [OverlayQuad] = [];
    let w = Float32(from: width);
    let h = Float32(from: height);
    let none = Vec4(x: -1.0, y: 0.0, z: 0.0, w: 0.0);
    let white = Vec4(x: 1.0, y: 1.0, z: 1.0, w: 0.85);
    // Crosshair; it widens while gliding.
    let arm: Float32 = if gliding { 14.0 } else { 9.0 };
    quads.append(OverlayQuad(rect: Vec4(x: w * 0.5 - arm, y: h * 0.5 - 1.0, z: arm * 2.0, w: 2.0), color: white, tile: none));
    quads.append(OverlayQuad(rect: Vec4(x: w * 0.5 - 1.0, y: h * 0.5 - arm, z: 2.0, w: arm * 2.0), color: white, tile: none));
    // Hotbar: a frame per slot, the selected one bright, with its block.
    let slot: Float32 = 52.0;
    let left = w * 0.5 - slot * 4.0;
    let top = h - slot - 14.0;
    quads.append(OverlayQuad(rect: Vec4(x: left - 4.0, y: top - 4.0, z: slot * 8.0 + 8.0, w: slot + 8.0), color: Vec4(x: 0.12, y: 0.14, z: 0.22, w: 0.45), tile: none));
    var i: Int64 = 0;
    while i < hotbar.count {
        let x = left + Float32(from: i) * slot;
        if i == selected {
            quads.append(OverlayQuad(rect: Vec4(x: x, y: top, z: slot, w: slot), color: Vec4(x: 1.0, y: 0.95, z: 0.8, w: 0.9), tile: none));
        }
        let layer = Float32(from: Int64(from: hotbar(i).layer(.South).index));
        quads.append(OverlayQuad(
            rect: Vec4(x: x + 6.0, y: top + 6.0, z: slot - 12.0, w: slot - 12.0),
            color: Vec4(x: 1.0, y: 1.0, z: 1.0, w: 1.0),
            tile: Vec4(x: layer, y: 0.0, z: 0.0, w: 0.0)
        ));
        i = i + 1;
    }
    quads
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
    var player: Player
    var selected: Int64
    var controls: Controls
    var seconds: Float32
    var running: Bool

    init(consuming renderer: Renderer, seed seed: Int64) {
        self.renderer = renderer;
        self.renderer.viewDistance = Float32(from: viewRadius * chunkSize);
        self.world = World(seed);
        self.meshed = Dictionary[Int64, Bool]();
        self.player = Player(spawnPoint(self.world.generator));
        self.selected = 0;
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

    /// Re-mesh every loaded chunk that block (x, y, z) shows up in: its own,
    /// and neighbors whose border or corner shading includes it.
    mutating func remeshAround(x: Int64, y: Int64, z: Int64) {
        var done: [Int64] = [];
        var dy: Int64 = -1;
        while dy <= 1 {
            var dz: Int64 = -1;
            while dz <= 1 {
                var dx: Int64 = -1;
                while dx <= 1 {
                    let (cx, cy, cz) = (chunkOf(x + dx), chunkOf(y + dy), chunkOf(z + dz));
                    let key = chunkKey(cx, cy, cz);
                    if World.inRange(cy) and not done.contains(key) and self.meshed.contains(chunkKey(cx, 0, cz)) {
                        done.append(key);
                        self.renderer.upload(cx, cy, cz, meshChunk(self.world.padded(cx, cy, cz)));
                    }
                    dx = dx + 1;
                }
                dz = dz + 1;
            }
            dy = dy + 1;
        }
    }

    /// Break or place a block where the player is looking.
    mutating func useBlocks() {
        if not self.controls.breakPressed and not self.controls.placePressed {
            return;
        }
        let view = self.player.camera;
        guard let .Some(hit) = raycast(self.world, view.position, view.forward, reach) else {
            return;
        }
        if self.controls.breakPressed {
            if self.world.setBlock(hit.x, hit.y, hit.z, .Air) {
                self.remeshAround(hit.x, hit.y, hit.z);
            }
            return;
        }
        // Don't wall the player in.
        if self.player.overlaps(hit.beforeX, hit.beforeY, hit.beforeZ) {
            return;
        }
        if self.world.setBlock(hit.beforeX, hit.beforeY, hit.beforeZ, hotbar(self.selected)) {
            self.remeshAround(hit.beforeX, hit.beforeY, hit.beforeZ);
        }
    }

    mutating func frame(dt: Float32) -> () throws WgpuError {
        self.seconds = self.seconds + dt;
        if self.controls.flyPressed {
            self.player.flying = not self.player.flying;
        }
        if self.controls.slotPressed >= 0 {
            self.selected = self.controls.slotPressed;
        }
        // Long frames (a stall, a debug build) mustn't launch the player
        // through the floor.
        let step = if dt > 0.05 { 0.05 } else { dt };
        self.player.update(self.controls, self.world, dt: step);
        self.useBlocks();
        self.controls.clearPresses();
        self.stream();
        // A low sun that slowly circles.
        let angle = 0.6 + self.seconds * 0.05;
        let sun = Vec3(x: angle.cos(), y: -0.7, z: angle.sin()).normalized;
        let view = self.player.camera;
        let overlay = hud(self.renderer.width, self.renderer.height, self.selected, self.player.gliding);
        try self.renderer.draw(view.view(), eye: view.position, sun: sun, overlay: overlay);
        .Ok(())
    }
}
