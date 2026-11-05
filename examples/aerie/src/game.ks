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
    var timePressed: Bool = false
    var savePressed: Bool = false

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
            .Digit9 => { if held { self.slotPressed = 8 } },
            .T => { if held { self.timePressed = true } },
            .P => { if held { self.savePressed = true } },
            _ => {}
        }
    }

    /// Forget presses once a frame has handled them.
    mutating func clearPresses() {
        self.breakPressed = false;
        self.placePressed = false;
        self.flyPressed = false;
        self.slotPressed = -1;
        self.timePressed = false;
        self.savePressed = false;
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
/// How far away blocks can be broken or placed.
let reach: Float32 = 6.0;

/// A plain overlay rectangle.
func rect(x: Float32, y: Float32, w: Float32, h: Float32, color: Vec4) -> OverlayQuad {
    OverlayQuad(rect: Vec4(x: x, y: y, z: w, w: h), color: color, tile: Vec4(x: -1.0, y: 0.0, z: 0.0, w: 0.0))
}

/// A four-pointed twinkle centered on (x, y), `size` pixels from tip to tip.
func sparkle(x: Float32, y: Float32, size: Float32, color: Vec4) -> [OverlayQuad] {
    let arm = (size * 0.5).floor();
    let t = (size / 8.0).ceil();
    [
        rect(x - arm, y - t * 0.5, arm * 2.0, t, color),
        rect(x - t * 0.5, y - arm, t, arm * 2.0, color),
        rect(x - t * 1.5, y - t * 1.5, t * 3.0, t * 3.0, color)
    ]
}

/// `text` shading from `color` to `bottom`, ringed by `rim` so pale
/// letters stay readable over a pale sky.
func outlinedQuads(text: String, x x: Float32, y y: Float32, scale scale: Float32, color color: Vec4, bottom bottom: Vec4, rim rim: Vec4) -> [OverlayQuad] {
    var quads: [OverlayQuad] = [];
    let edge = (scale * 0.3).ceil();
    for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
        let outline = textQuads(text, x: x + dx * edge, y: y + dy * edge, scale: scale, color: rim, bottom: rim);
        quads.append(contentsOf: outline);
    }
    let lit = textQuads(text, x: x, y: y, scale: scale, color: color, bottom: bottom);
    quads.append(contentsOf: lit);
    quads
}

/// The crosshair, the hotbar and the selected block's name for a `width`
/// by `height` screen, `time` seconds in.
func hud(width: Int64, height: Int64, selected: Int64, gliding: Bool, time: Float32) -> [OverlayQuad] {
    var quads: [OverlayQuad] = [];
    let w = Float32(from: width);
    let h = Float32(from: height);
    // Crosshair, on a soft indigo outline; it widens while gliding.
    let arm: Float32 = if gliding { 13.0 } else { 8.0 };
    let (cx, cy) = ((w * 0.5).floor(), (h * 0.5).floor());
    let outline = Vec4(x: 0.14, y: 0.14, z: 0.26, w: 0.45);
    let white = Vec4(x: 1.0, y: 0.97, z: 1.0, w: 0.9);
    quads.append(rect(cx - arm - 1.0, cy - 2.0, arm * 2.0 + 2.0, 4.0, outline));
    quads.append(rect(cx - 2.0, cy - arm - 1.0, 4.0, arm * 2.0 + 2.0, outline));
    quads.append(rect(cx - arm, cy - 1.0, arm * 2.0, 2.0, white));
    quads.append(rect(cx - 1.0, cy - arm, 2.0, arm * 2.0, white));

    // Hotbar: a frosted slate tray of slots. The selected slot rises,
    // ringed in gold, and its block's name floats above.
    let slot: Float32 = 56.0;
    let gap: Float32 = 6.0;
    let count = Float32(from: hotbar.count);
    let total = count * slot + (count - 1.0) * gap;
    let left = ((w - total) * 0.5).floor();
    let top = h - slot - 20.0;
    quads.append(rect(left - 10.0, top - 10.0, total + 20.0, slot + 20.0, Vec4(x: 0.26, y: 0.28, z: 0.42, w: 0.45)));
    quads.append(rect(left - 10.0, top - 10.0, total + 20.0, 2.0, Vec4(x: 0.92, y: 0.94, z: 1.0, w: 0.45)));
    quads.append(rect(left - 10.0, top + slot + 8.0, total + 20.0, 2.0, Vec4(x: 0.14, y: 0.14, z: 0.24, w: 0.45)));
    var i: Int64 = 0;
    while i < hotbar.count {
        let chosen = i == selected;
        let x = left + Float32(from: i) * (slot + gap);
        let y = if chosen { top - 6.0 } else { top };
        if chosen {
            let pulse = 0.8 + 0.2 * (time * 4.0).sin();
            quads.append(rect(x - 4.0, y - 4.0, slot + 8.0, slot + 8.0, Vec4(x: 1.0, y: 0.86, z: 0.56, w: pulse)));
            quads.append(rect(x - 2.0, y - 2.0, slot + 4.0, slot + 4.0, Vec4(x: 1.0, y: 0.97, z: 0.88, w: 1.0)));
        }
        let face = if chosen { Vec4(x: 0.96, y: 0.96, z: 1.0, w: 0.95) } else { Vec4(x: 0.90, y: 0.92, z: 1.0, w: 0.28) };
        quads.append(rect(x, y, slot, slot, face));
        // A recess for the block, a little shadow under it, then the block.
        quads.append(rect(x + 3.0, y + 3.0, slot - 6.0, slot - 6.0, Vec4(x: 0.16, y: 0.18, z: 0.30, w: 0.30)));
        let icon = slot - 20.0;
        quads.append(rect(x + 12.0, y + 13.0, icon, icon, Vec4(x: 0.10, y: 0.10, z: 0.20, w: 0.45)));
        let layer = Float32(from: Int64(from: hotbar(i).layer(.South).index));
        quads.append(OverlayQuad(
            rect: Vec4(x: x + 10.0, y: y + 10.0, z: icon, w: icon),
            color: Vec4(x: 1.0, y: 1.0, z: 1.0, w: 1.0),
            tile: Vec4(x: layer, y: 0.0, z: 0.0, w: 0.0)
        ));
        let number = labelQuads("\(i + 1)", x: x + 4.0, y: y + 4.0, scale: 2.0, color: Vec4(x: 1.0, y: 1.0, z: 1.0, w: if chosen { 0.95 } else { 0.7 }));
        quads.append(contentsOf: number);
        i = i + 1;
    }
    let name = hotbar(selected).name;
    let nameScale: Float32 = 3.0;
    let label = gradientLabelQuads(name, x: ((w - textWidth(name, nameScale)) * 0.5).floor(), y: top - 38.0, scale: nameScale,
        color: Vec4(x: 1.0, y: 1.0, z: 1.0, w: 0.95), bottom: Vec4(x: 0.84, y: 0.90, z: 1.0, w: 0.95));
    quads.append(contentsOf: label);
    quads
}

/// The title screen's overlay, `time` seconds in: a bobbing name
/// with twinkles around it, a pulsing prompt and the controls.
func titleScreen(width: Int64, height: Int64, resumed: Bool, time: Float32) -> [OverlayQuad] {
    let w = Float32(from: width);
    let h = Float32(from: height);
    var quads: [OverlayQuad] = [];
    // A soft indigo veil behind the text, so pastel letters read over a
    // pastel sky: stacked bands, densest in the middle, instead of a box.
    let middle = h * 0.47;
    var band: Int64 = 0;
    while band < 8 {
        let reach = h * 0.30 * (1.0 - Float32(from: band) / 8.0);
        quads.append(rect(0.0, (middle - reach).floor(), w, (reach * 2.0).floor(), Vec4(x: 0.20, y: 0.22, z: 0.40, w: 0.05)));
        band = band + 1;
    }
    let titleScale = (w / 60.0).floor().clamp(10.0, 24.0);
    let bob = (time * 1.3).sin() * titleScale * 0.3;
    let titleTop = (h * 0.24 + bob).floor();
    let title = "AERIE";
    let titleLeft = ((w - textWidth(title, titleScale)) * 0.5).floor();
    // A deep extrusion and an outline give the letters a puffy depth.
    let deep = Vec4(x: 0.30, y: 0.34, z: 0.56, w: 0.85);
    let depth = textQuads(title, x: titleLeft + titleScale * 0.6, y: titleTop + titleScale * 0.9, scale: titleScale, color: deep, bottom: Vec4(x: 0.20, y: 0.22, z: 0.40, w: 0.85));
    quads.append(contentsOf: depth);
    let name = outlinedQuads(title, x: titleLeft, y: titleTop, scale: titleScale,
        color: Vec4(x: 1.0, y: 1.0, z: 1.0, w: 1.0), bottom: Vec4(x: 0.78, y: 0.88, z: 1.0, w: 1.0), rim: Vec4(x: 0.36, y: 0.42, z: 0.66, w: 0.95));
    quads.append(contentsOf: name);
    // Twinkles around the name, each on its own beat.
    let titleWidth = textWidth(title, titleScale);
    let spots: [(Float32, Float32, Float32)] = [(-0.08, 0.1, 0.0), (1.06, 0.2, 1.7), (0.18, -0.25, 3.1), (0.86, -0.2, 4.4), (0.5, 1.35, 2.3), (-0.04, 1.1, 5.2), (1.1, 1.0, 0.9)];
    for (fx, fy, phase) in spots {
        let beat = ((time * 2.2 + phase).sin() * 0.5 + 0.5);
        let size = (titleScale * (0.6 + beat * 0.9)).floor();
        let glints = sparkle(titleLeft + titleWidth * fx, titleTop + titleScale * 5.0 * fy, size, Vec4(x: 1.0, y: 0.95, z: 1.0, w: 0.35 + beat * 0.6));
        quads.append(contentsOf: glints);
    }
    let subtitle = "ISLANDS ABOVE THE CLOUD SEA";
    let subScale = (titleScale * 0.22).floor().clamp(3.0, 6.0);
    let sub = outlinedQuads(subtitle, x: ((w - textWidth(subtitle, subScale)) * 0.5).floor(), y: (h * 0.24 + titleScale * 6.5).floor(), scale: subScale,
        color: Vec4(x: 0.90, y: 0.95, z: 1.0, w: 1.0), bottom: Vec4(x: 0.80, y: 0.84, z: 1.0, w: 1.0), rim: Vec4(x: 0.22, y: 0.26, z: 0.46, w: 0.85));
    quads.append(contentsOf: sub);
    // Bound first: an `if` directly inside an array literal crashes the
    // compiler (array_literal_if_expression_owned_element.ks).
    let prompt = if resumed { "PRESS ENTER TO CONTINUE" } else { "PRESS ENTER TO START A NEW WORLD" };
    let promptScale = subScale + 1.0;
    let glow = 0.75 + 0.25 * (time * 3.0).sin();
    let promptTop = (h * 0.66).floor();
    let pressed = outlinedQuads(prompt, x: ((w - textWidth(prompt, promptScale)) * 0.5).floor(), y: promptTop, scale: promptScale,
        color: Vec4(x: 1.0, y: 1.0, z: 1.0, w: glow), bottom: Vec4(x: 1.0, y: 0.94, z: 0.80, w: glow), rim: Vec4(x: 0.22, y: 0.26, z: 0.46, w: glow * 0.9));
    quads.append(contentsOf: pressed);
    // The controls, on a frosted strip along the bottom.
    let controls = "WASD MOVE  ARROWS LOOK  SPACE JUMP/GLIDE  Q BREAK  E PLACE  P SAVE";
    let controlScale: Float32 = 3.0;
    quads.append(rect(0.0, h - 48.0, w, 48.0, Vec4(x: 0.20, y: 0.22, z: 0.36, w: 0.45)));
    quads.append(rect(0.0, h - 48.0, w, 2.0, Vec4(x: 0.92, y: 0.94, z: 1.0, w: 0.40)));
    let help = labelQuads(controls, x: ((w - textWidth(controls, controlScale)) * 0.5).floor(), y: h - 32.0, scale: controlScale, color: Vec4(x: 0.96, y: 0.94, z: 1.0, w: 0.9));
    quads.append(contentsOf: help);
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
    /// 0 to 1 over a day; see `daylight`.
    var timeOfDay: Float32
    var running: Bool
    /// Set when the player asks to save; the driver writes `save()` out and
    /// clears it, since only it knows where saves live.
    var wantsSave: Bool
    /// Showing the title screen: the world turns behind it until Enter.
    var onTitle: Bool
    /// Whether the world came from a save, for the title's wording.
    var resumed: Bool
    /// A short message in the corner, shown while `noticeTime` > 0.
    var notice: String
    var noticeTime: Float32

    /// A new world from `seed`, or the one in `save`.
    init(consuming renderer: Renderer, seed seed: Int64, save save: SaveState?) {
        self.renderer = renderer;
        self.renderer.viewDistance = Float32(from: viewRadius * chunkSize);
        let worldSeed = match save { .Some(s) => s.seed, .None => seed };
        self.world = World(worldSeed);
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
        self.resumed = save.isSome();
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
            .KeyDown(.Escape) => { self.running = false },
            .KeyDown(.Return) => { self.onTitle = false },
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
            let glowed = match self.world.blockAt(hit.x, hit.y, hit.z) {
                .Some(b) => b.emits,
                .None => false
            };
            if self.world.setBlock(hit.x, hit.y, hit.z, .Air) {
                self.remeshAround(hit.x, hit.y, hit.z, light: glowed);
            }
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

    /// Advance the game by `dt` seconds and draw it.
    mutating func frame(dt: Float32) -> () throws WgpuError {
        self.seconds = self.seconds + dt;
        if not self.onTitle {
            self.play(dt);
        }
        self.stream();
        self.timeOfDay = self.timeOfDay + dt / dayLength;
        self.timeOfDay = self.timeOfDay - self.timeOfDay.floor();
        self.controls.clearPresses();
        self.noticeTime = self.noticeTime - dt;

        let (w, h) = (self.renderer.width, self.renderer.height);
        // The title screen turns slowly above where play starts.
        let view = if self.onTitle {
            Camera(position: self.player.eye + Vec3(x: 0.0, y: 6.0, z: 0.0), yaw: self.player.yaw + self.seconds * 0.05, pitch: -0.15)
        } else {
            self.player.camera
        };
        var overlay = if self.onTitle { titleScreen(w, h, self.resumed, self.seconds) } else { hud(w, h, self.selected, self.player.gliding, self.seconds) };
        if self.noticeTime > 0.0 {
            let notice = labelQuads(self.notice, x: 16.0, y: 16.0, scale: 4.0, color: Vec4(x: 1.0, y: 1.0, z: 1.0, w: 0.9));
            overlay.append(contentsOf: notice);
        }
        try self.renderer.draw(view.view(), eye: view.position, light: daylight(self.timeOfDay), time: self.seconds, overlay: overlay);
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
        self.useBlocks();
        if self.controls.savePressed {
            self.wantsSave = true;
        }
        // T skips ahead a quarter of a day.
        if self.controls.timePressed {
            self.timeOfDay = self.timeOfDay + 0.25;
        }
    }

    /// Show `text` in the corner for a few seconds.
    mutating func notify(text: String) {
        self.notice = text;
        self.noticeTime = 3.0;
    }
}
