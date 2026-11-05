module Aerie

import wgpu.(WgpuError)
import sdl.(Event)
import linalg.(Vec3)

/// The game, driven one event and one frame at a time. Nothing here owns
/// the main loop, so a browser build can call `frame` from
/// `requestAnimationFrame` instead of a `while` loop. It touches no files
/// or windows: it sets `wantsSave` and `mouseCaptured`, and the driver acts
/// on them.
struct Game: not Copyable {
    var renderer: Renderer
    var settings: Settings
    var world: World
    var player: Player
    var flock: Flock
    var controls: Controls
    var hotbar: Hotbar
    /// The block being broken, if any.
    var breaking: Breaking?
    var sky: Sky
    var autosave: Autosave
    var fps: FpsCounter
    var notice: Notice
    /// Seconds since the game started.
    var seconds: Float32
    /// Showing the title screen: the world turns behind it until Enter.
    var onTitle: Bool
    /// Whether the mouse should be captured for mouse look.
    var mouseCaptured: Bool
    var running: Bool
    /// Set when it's time to save; the driver writes `save()` out and
    /// clears it, since only it knows where saves live.
    var wantsSave: Bool

    /// A new world from `seed`, or the one in `save`.
    init(consuming renderer: Renderer, settings settings: Settings, seed seed: Int64, save save: SaveState?) {
        self.renderer = renderer;
        self.renderer.viewDistance = settings.viewDistance;
        self.settings = settings;
        let worldSeed = match save { .Some(s) => s.seed, .None => seed };
        self.world = World(worldSeed);
        self.player = Player(self.world.generator.spawnPoint());
        self.flock = Flock(seed: worldSeed);
        self.controls = Controls();
        self.hotbar = Hotbar();
        self.breaking = .None;
        self.sky = Sky(timeOfDay: settings.startTime ?? 0.1);
        self.autosave = Autosave();
        self.fps = FpsCounter();
        self.notice = Notice();
        self.seconds = 0.0;
        self.onTitle = not settings.skipTitle;
        self.mouseCaptured = false;
        self.running = true;
        self.wantsSave = false;
        if let .Some(s) = save {
            self.restore(s);
        }
    }

    /// Pick up where `state` left off. A time of day set at startup wins.
    mutating func restore(state: SaveState) {
        for (key, chunk) in state.chunks {
            let _ = self.world.stored.insert(key, chunk);
        }
        self.player.position = state.position;
        self.player.yaw = state.yaw;
        self.player.pitch = state.pitch;
        if self.settings.startTime.isNone() {
            self.sky.timeOfDay = state.timeOfDay;
        }
    }

    /// Everything a save file keeps.
    func save() -> SaveState {
        SaveState(
            seed: self.world.generator.seed,
            timeOfDay: self.sky.timeOfDay,
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
            // A click starts play or recaptures the mouse.
            .MouseDown(_, _, _) => {
                if self.onTitle or not self.mouseCaptured { self.start() } else { self.controls.handle(event) }
            },
            // Mouse movement only looks around while captured.
            .MouseMove(_, _, _, _) => {
                if self.mouseCaptured { self.controls.handle(event) }
            },
            _ => { self.controls.handle(event) }
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
        let view = self.view();
        self.widenView(dt);
        self.sky.update(dt, lairness: self.world.generator.lairness(view.position.x, view.position.z));
        self.fps.count(dt, toggle: self.controls.fpsPressed);
        self.notice.update(dt);
        self.controls.clearPresses();
        let (models, casters) = self.models();
        let hud = Hud(self.renderer.width, self.renderer.height);
        let icons = if self.onTitle { [] } else { hud.icons(self.hotbar, time: self.seconds) };
        try self.renderer.draw(Scene(
            view: view.view(), eye: view.position, light: self.sky.daylight, time: self.seconds,
            models: models, casters: casters, overlay: self.overlay(), icons: icons
        ));
        .Ok(())
    }

    /// Widen the view as you soar faster, up to 25 degrees, easing in and
    /// out, so speed reads as speed.
    mutating func widenView(dt: Float32) {
        let fast = if self.player.gliding { ((self.player.velocity.length - 8.0) * 0.8).clamp(0.0, 25.0) } else { 0.0 };
        let fov = self.renderer.fieldOfView;
        self.renderer.fieldOfView = fov + (baseFieldOfView + fast - fov) * (dt * 3.0).clamp(0.0, 1.0);
    }

    /// Through the player's eyes; on the title screen, turning slowly
    /// above where play starts.
    func view() -> Camera {
        guard self.onTitle else {
            return self.player.camera;
        }
        Camera(position: self.player.eye + Vec3(x: 0.0, y: 6.0, z: 0.0), yaw: self.player.yaw + self.seconds * 0.05, pitch: -0.15)
    }

    /// The interface over the world: the title screen, or the HUD with the
    /// frame rate and any notice in the corner.
    func overlay() -> [OverlayQuad] {
        let hud = Hud(self.renderer.width, self.renderer.height);
        if self.onTitle {
            var quads = hud.title(self.seconds);
            quads.append(contentsOf: self.notice.quads(row: 0));
            return quads;
        }
        var quads = hud.play(self.hotbar, gliding: self.player.gliding, time: self.seconds);
        quads.append(contentsOf: self.fps.quads());
        quads.append(contentsOf: self.notice.quads(row: if self.fps.shown { 1 } else { 0 }));
        quads
    }

    /// One frame of play: the player's input, movement and edits.
    mutating func play(dt: Float32) {
        if self.controls.flyPressed {
            self.player.flying = not self.player.flying;
        }
        // Once per press, before the physics steps, so the frame's own
        // jump off the ground isn't taken for a double jump.
        if self.controls.jumpPressed {
            self.player.boost();
        }
        self.hotbar.update(self.controls);
        self.player.look(yaw: self.controls.mouseX * mouseSensitivity, pitch: -self.controls.mouseY * mouseSensitivity);
        self.movePlayer(dt);
        self.useBlocks(dt);
        // Saved every minute of play, and on quitting.
        if self.autosave.due(dt) {
            self.wantsSave = true;
        }
    }

    /// Physics in steps of at most 50 ms, so a slow frame can't launch the
    /// player through the floor; a long stall is dropped. Unloaded chunks
    /// count as solid, so the player waits, frozen, until the columns
    /// around them exist rather than being shoved out of them.
    mutating func movePlayer(dt: Float32) {
        let (pcx, pcz) = columnOf(self.player.position);
        guard self.world.neighborsReady(pcx, pcz) else {
            return;
        }
        var remaining = dt.clamp(0.0, 0.25);
        while remaining > 0.0 {
            let step = remaining.clamp(0.0, 0.05);
            self.player.update(self.controls, self.world, dt: step);
            remaining = remaining - step;
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
        self.notice.show(text);
    }
}
