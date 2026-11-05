module Aerie

import wgpu.(WgpuError)
import sdl.(Event, Key, monotonicMs)
import linalg.(Vec3, Vec4, Mat4)

/// Seconds in one day.
let dayLength: Float32 = 480.0;

/// How often the world saves itself during play, in seconds.
let autosaveSeconds: Float32 = 60.0;

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
    /// Frames per second, shown in the corner when Z turns it on.
    var fps: FpsCounter
    var breaking: Breaking?
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
        self.untilAutosave = autosaveSeconds;
        self.fps = FpsCounter();
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
        self.fps.count(dt, toggle: self.controls.fpsPressed);
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
        if self.fps.shown and not self.onTitle {
            overlay.append(contentsOf: fpsQuads(self.fps.rate));
        }
        if self.noticeTime > 0.0 {
            let row: Int64 = if self.fps.shown and not self.onTitle { 1 } else { 0 };
            let notice = noticeQuads(self.notice, row: row, fade: self.noticeTime.clamp(0.0, 1.0));
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

/// Frames per second, measured over half a second at a time.
struct FpsCounter {
    var shown: Bool = false
    /// The last measured rate, and frames and seconds toward the next.
    var rate: Int64 = 0
    var frames: Int64 = 0
    var seconds: Float32 = 0.0

    init() {}

    /// Count a frame of `dt` seconds; `toggle` shows or hides the readout.
    mutating func count(dt: Float32, toggle toggle: Bool) {
        if toggle {
            self.shown = not self.shown;
        }
        self.frames = self.frames + 1;
        self.seconds = self.seconds + dt;
        if self.seconds < 0.5 {
            return;
        }
        self.rate = (Float32(from: self.frames) / self.seconds + 0.5).toInt64() ?? 0;
        self.frames = 0;
        self.seconds = 0.0;
    }
}
