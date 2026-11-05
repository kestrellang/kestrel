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
    /// Breaking: the left mouse button, held.
    var breakHeld: Bool = false
    /// Presses waiting for the next frame: place, toggle flying, and a
    /// hotbar slot (or -1).
    var placePressed: Bool = false
    var flyPressed: Bool = false
    var slotPressed: Int64 = -1
    var fpsPressed: Bool = false
    /// Mouse movement since the last frame, in pixels, and wheel clicks.
    var mouseX: Float32 = 0.0
    var mouseY: Float32 = 0.0
    var wheel: Int64 = 0

    init() {}

    mutating func setKey(key: Key, held held: Bool) {
        match key {
            .W => { self.forward = held },
            .S => { self.back = held },
            .A => { self.left = held },
            .D => { self.right = held },
            .Space => { self.up = held },
            .LeftShift => { self.down = held },
            .F => { if held { self.flyPressed = true } },
            .Digit1 => { if held { self.slotPressed = 0 } },
            .Digit2 => { if held { self.slotPressed = 1 } },
            .Digit3 => { if held { self.slotPressed = 2 } },
            .Digit4 => { if held { self.slotPressed = 3 } },
            .Digit5 => { if held { self.slotPressed = 4 } },
            .Digit6 => { if held { self.slotPressed = 5 } },
            .Digit7 => { if held { self.slotPressed = 6 } },
            .Digit8 => { if held { self.slotPressed = 7 } },
            .Digit9 => { if held { self.slotPressed = 8 } },
            .Z => { if held { self.fpsPressed = true } },
            _ => {}
        }
    }

    /// Forget presses once a frame has handled them.
    mutating func clearPresses() {
        self.placePressed = false;
        self.flyPressed = false;
        self.slotPressed = -1;
        self.fpsPressed = false;
        self.mouseX = 0.0;
        self.mouseY = 0.0;
        self.wheel = 0;
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

/// Seconds in one day.
let dayLength: Float32 = 480.0;

/// The blocks on the hotbar, keys 1 to 9.
let hotbar: [Block] = [.Driftstone, .Loam, .Mossloam, .Emberwood, .Planks, .Cloudglass, .Featherthatch, .Sailcloth, .Glowcap];
/// Radians of turn per pixel of mouse movement.
let mouseSensitivity: Float32 = 0.0025;
/// How far away blocks can be broken or placed.
let reach: Float32 = 6.0;

/// How often the world saves itself during play, in seconds.
let autosaveSeconds: Float32 = 60.0;

/// How many chunk columns around the player are drawn, by default and at
/// most; `AERIE_VIEW` picks within the range.
let defaultViewRadius: Int64 = 5;
let minViewRadius: Int64 = 2;
let maxViewRadius: Int64 = 16;
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
    /// 0 to 1 over a day; see `daylight`.
    var timeOfDay: Float32
    var running: Bool
    /// Set when the player asks to save; the driver writes `save()` out and
    /// clears it, since only it knows where saves live.
    var wantsSave: Bool
    /// Showing the title screen: the world turns behind it until Enter.
    var onTitle: Bool
    /// A short message in the corner, shown while `noticeTime` > 0.
    var notice: String
    var noticeTime: Float32
    /// How many chunk columns around the player are loaded and drawn.
    var viewRadius: Int64
    /// Whether the mouse should be captured for mouse look; the driver
    /// applies it to the window.
    var mouseCaptured: Bool
    /// Seconds until the next autosave.
    var untilAutosave: Float32
    /// Z shows frames per second in the corner: the count shown, and
    /// frames and seconds counted toward the next one.
    var showFps: Bool
    var fps: Int64
    var fpsFrames: Int64
    var fpsSeconds: Float32
    /// The block being broken and how far along, 0 to 1; breaking takes
    /// `Block.hardness` seconds of holding.
    var breaking: (Int64, Int64, Int64)?
    var breakProgress: Float32
    var flock: Flock
    /// How much the lairs' crimson sky shows, easing toward how deep in
    /// lair country the camera is.
    var gloom: Float32

    /// A new world from `seed`, or the one in `save`.
    init(consuming renderer: Renderer, seed seed: Int64, save save: SaveState?) {
        self.renderer = renderer;
        // `AERIE_VIEW` sets the view distance in chunks.
        let view = match getenv("AERIE_VIEW") {
            .Some(v) => Int64(parsing: v) ?? defaultViewRadius,
            .None => defaultViewRadius
        };
        self.viewRadius = view.clamp(minViewRadius, maxViewRadius);
        self.mouseCaptured = false;
        self.gloom = 0.0;
        self.breaking = .None;
        self.breakProgress = 0.0;
        self.untilAutosave = autosaveSeconds;
        self.showFps = false;
        self.fps = 0;
        self.fpsFrames = 0;
        self.fpsSeconds = 0.0;
        self.renderer.viewDistance = Float32(from: self.viewRadius * chunkSize);
        let worldSeed = match save { .Some(s) => s.seed, .None => seed };
        self.world = World(worldSeed);
        self.flock = Flock(seed: worldSeed);
        self.meshed = Dictionary[Int64, Bool]();
        self.player = Player(spawnPoint(self.world.generator));
        self.selected = 0;
        self.controls = Controls();
        self.seconds = 0.0;
        // `AERIE_TIME` sets the starting time of day, 0 to 1.
        self.timeOfDay = match getenv("AERIE_TIME") {
            .Some(t) => Float32(parsing: t) ?? 0.1,
            .None => 0.1
        };
        self.running = true;
        self.wantsSave = false;
        // `AERIE_PLAY=1` skips the title screen.
        self.onTitle = getenv("AERIE_PLAY").isNone();
        self.notice = "";
        self.noticeTime = 0.0;
        if let .Some(s) = save {
            for (key, chunk) in s.chunks {
                let _ = self.world.stored.insert(key, chunk);
            }
            self.player.position = s.position;
            self.player.yaw = s.yaw;
            self.player.pitch = s.pitch;
            if getenv("AERIE_TIME").isNone() { self.timeOfDay = s.timeOfDay; }
        }
    }

    /// Everything a save file keeps.
    func save() -> SaveState {
        SaveState(
            seed: self.world.generator.seed,
            timeOfDay: self.timeOfDay,
            position: self.player.position,
            yaw: self.player.yaw,
            pitch: self.player.pitch,
            chunks: self.world.editedChunks()
        )
    }

    mutating func event(event: Event) {
        match event {
            .Quit => { self.running = false },
            // Escape frees a captured mouse first, then saves and quits.
            .KeyDown(.Escape) => {
                if self.mouseCaptured { self.release() } else { self.running = false }
            },
            .KeyDown(.Return) => { self.start() },
            // A click starts play or recaptures the mouse; once captured,
            // left breaks and right places.
            .MouseDown(button, _, _) => {
                if self.onTitle or not self.mouseCaptured {
                    self.start();
                } else {
                    match button {
                        .Left => { self.controls.breakHeld = true },
                        .Right => { self.controls.placePressed = true },
                        _ => {}
                    }
                }
            },
            .MouseUp(.Left, _, _) => { self.controls.breakHeld = false },
            .MouseMove(_, _, dx, dy) => {
                if self.mouseCaptured {
                    self.controls.mouseX = self.controls.mouseX + Float32(from: dx);
                    self.controls.mouseY = self.controls.mouseY + Float32(from: dy);
                }
            },
            .MouseWheel(clicks) => { self.controls.wheel = self.controls.wheel + clicks },
            .KeyDown(key) => { self.controls.setKey(key, held: true) },
            .KeyUp(key) => { self.controls.setKey(key, held: false) },
            _ => {}
        }
    }

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

    /// Break or place a block where the player is looking. Breaking
    /// takes holding: the block cracks over its `hardness` in seconds, and
    /// looking away or letting go starts over.
    mutating func useBlocks(dt: Float32) {
        let view = self.player.camera;
        let target = raycast(self.world, view.position, view.forward, reach);
        if not self.controls.breakHeld {
            self.breaking = .None;
        }
        if self.controls.breakHeld {
            if let .Some(hit) = target {
                self.chip(hit.x, hit.y, hit.z, dt);
            } else {
                self.breaking = .None;
            }
            return;
        }
        if not self.controls.placePressed {
            return;
        }
        guard let .Some(hit) = target else {
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
        let same = match self.breaking {
            .Some((bx, by, bz)) => bx == x and by == y and bz == z,
            .None => false
        };
        if not same {
            self.breaking = .Some((x, y, z));
            self.breakProgress = 0.0;
        }
        self.breakProgress = self.breakProgress + dt / block.hardness;
        if self.breakProgress < 1.0 {
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
        if let .Some((x, y, z)) = self.breaking {
            boxes.append(crackBox(x, y, z, self.breakProgress));
        }
        (boxes, birds)
    }

    /// Advance the game by `dt` seconds and draw it.
    mutating func frame(dt: Float32) -> () throws WgpuError {
        self.seconds = self.seconds + dt;
        if not self.onTitle {
            self.play(dt);
        }
        self.stream();
        self.flock.update(dt, center: self.player.position, reach: self.renderer.viewDistance.clamp(32.0, 96.0), generator: self.world.generator, time: self.seconds);
        self.timeOfDay = self.timeOfDay + dt / dayLength;
        self.timeOfDay = self.timeOfDay - self.timeOfDay.floor();
        self.countFps(dt);
        self.controls.clearPresses();
        self.noticeTime = self.noticeTime - dt;

        let (w, h) = (self.renderer.width, self.renderer.height);
        // The title screen turns slowly above where play starts.
        let view = if self.onTitle {
            Camera(position: self.player.eye + Vec3(x: 0.0, y: 6.0, z: 0.0), yaw: self.player.yaw + self.seconds * 0.05, pitch: -0.15)
        } else {
            self.player.camera
        };
        // Ease the sky toward the realm underfoot, over a couple of seconds.
        let target = self.world.generator.lairness(view.position.x, view.position.z);
        self.gloom = self.gloom + (target - self.gloom) * (dt * 0.6).clamp(0.0, 1.0);
        var overlay = if self.onTitle {
            titleScreen(w, h, self.seconds)
        } else {
            hud(w, h, self.selected, self.player.gliding, self.seconds)
        };
        if self.showFps and not self.onTitle {
            overlay.append(contentsOf: fpsQuads(self.fps));
        }
        if self.noticeTime > 0.0 {
            let notice = noticeQuads(self.notice, fade: self.noticeTime.clamp(0.0, 1.0));
            overlay.append(contentsOf: notice);
        }
        let (models, casters) = self.models();
        try self.renderer.draw(view.view(), eye: view.position, light: daylight(self.timeOfDay, gloom: self.gloom), time: self.seconds, models: models, casters: casters, overlay: overlay);
        .Ok(())
    }

    /// One frame of play: the player's input, movement and edits.
    mutating func play(dt: Float32) {
        if self.controls.flyPressed {
            self.player.flying = not self.player.flying;
        }
        if self.controls.slotPressed >= 0 {
            self.selected = self.controls.slotPressed;
        }
        // The wheel steps through the hotbar, wrapping at the ends; away
        // from you moves left.
        let slots = hotbar.count;
        self.selected = ((self.selected - self.controls.wheel) % slots + slots) % slots;
        self.player.look(yaw: self.controls.mouseX * mouseSensitivity, pitch: -self.controls.mouseY * mouseSensitivity);
        // Physics in steps of at most 50 ms, so a slow frame can't launch
        // the player through the floor; a long stall is dropped. Unloaded
        // chunks count as solid, so the player waits, frozen, until the
        // columns around them exist rather than being shoved out of them.
        let (pcx, pcz) = columnOf(self.player.position);
        var remaining = if not self.world.neighborsReady(pcx, pcz) { 0.0 } else if dt > 0.25 { 0.25 } else { dt };
        while remaining > 0.0 {
            let step = if remaining > 0.05 { 0.05 } else { remaining };
            self.player.update(self.controls, self.world, dt: step);
            remaining = remaining - step;
        }
        self.useBlocks(dt);
        // Saved every minute of play, and on quitting.
        self.untilAutosave = self.untilAutosave - dt;
        if self.untilAutosave <= 0.0 {
            self.wantsSave = true;
            self.untilAutosave = autosaveSeconds;
        }
    }

    /// Count this frame toward the FPS readout, which updates twice a
    /// second; Z shows or hides it.
    mutating func countFps(dt: Float32) {
        if self.controls.fpsPressed {
            self.showFps = not self.showFps;
        }
        self.fpsFrames = self.fpsFrames + 1;
        self.fpsSeconds = self.fpsSeconds + dt;
        if self.fpsSeconds >= 0.5 {
            self.fps = (Float32(from: self.fpsFrames) / self.fpsSeconds + 0.5).toInt64() ?? 0;
            self.fpsFrames = 0;
            self.fpsSeconds = 0.0;
        }
    }

    /// Leave the title screen and capture the mouse.
    mutating func start() {
        self.onTitle = false;
        self.mouseCaptured = true;
    }

    /// Free the mouse; a button held as it goes is let go.
    mutating func release() {
        self.mouseCaptured = false;
        self.controls.breakHeld = false;
    }

    /// Show `text` in the corner for a few seconds.
    mutating func notify(text: String) {
        self.notice = text;
        self.noticeTime = 3.0;
    }
}
