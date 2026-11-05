// Aerie: a voxel sandbox on floating islands above a cloud sea.
//
// Walk with WASD, look with the arrow keys, jump with Space and hold it
// while falling to glide. Q breaks the block you look at, E places the
// selected one, 1 to 9 pick from the hotbar, F toggles flying (Space and
// Left Shift to rise and sink), T skips ahead a quarter day, P saves.
// Escape saves and quits. The title screen waits for Enter
// (`AERIE_PLAY=1` skips it).
//
// The world is saved to `world.aerie`, or the path in `AERIE_SAVE`
// (`AERIE_SAVE=none` turns saving off). `AERIE_FRAMES=n` quits after n
// frames; `AERIE_BENCH=n` times chunk generation and meshing over n runs
// instead of opening a window.
//
// Run from this folder, so `assets/blocks.png` resolves.

module Aerie

import sdl.(SDLWindow, Event, monotonicMs)
import deflate.(Zlib)
import std.io.file.(readFileBytes, writeFileBytes)

/// Where the world is saved, or `None` with `AERIE_SAVE=none`.
func savePath() -> String? {
    match getenv("AERIE_SAVE") {
        .Some(path) => if path == "none" { .None } else { .Some(path) },
        .None => .Some(defaultSavePath())
    }
}

/// The save at `path`, if there is a readable one. A missing file is a new
/// world; a damaged one is reported and ignored rather than overwritten
/// silently on the next save.
func loadSave(path: String) -> SaveState? {
    let packed = match readFileBytes(path) {
        .Ok(bytes) => bytes,
        .Err(_) => { return .None; }
    };
    let bytes = match Zlib.decompress(packed) {
        .Ok(bytes) => bytes,
        .Err(_) => {
            println("\(path) is not a readable save; starting a new world");
            return .None;
        }
    };
    let state = decodeSave(bytes);
    if state.isNone() {
        println("\(path) is from another version of Aerie; starting a new world");
    }
    state
}

/// Compress and write `state` to `path`, reporting how it went. True if
/// it was written.
func writeSave(path: String, state: SaveState) -> Bool {
    let bytes = encodeSave(state);
    let packed = Zlib.compress(bytes);
    match writeFileBytes(path, packed) {
        .Ok(_) => {
            persistSaves();
            println("saved \(state.chunks.count) changed chunks to \(path) (\(packed.count) bytes)");
            true
        },
        .Err(_) => {
            println("couldn't write \(path)");
            false
        }
    }
}

/// Time building and meshing the test island `runs` times, without a
/// window, and print the average per chunk.
func bench(runs: Int64) -> Int64 {
    // The densest chunk of the column under the spawn point.
    let generator = Generator(1);
    let (cx, cz) = columnOf(spawnPoint(generator));
    let start = unsafe monotonicMs();
    var world = World(1);
    var i = 0;
    while i < runs {
        world = World(1);
        var dz: Int64 = -1;
        while dz <= 1 {
            var dx: Int64 = -1;
            while dx <= 1 {
                world.generateColumn(cx + dx, cz + dz);
                dx = dx + 1;
            }
            dz = dz + 1;
        }
        i = i + 1;
    }
    let built = unsafe monotonicMs();
    var faces = 0;
    i = 0;
    while i < runs {
        faces = 0;
        var cy: Int64 = 0;
        while cy < worldChunksY {
            faces = faces + meshChunk(world.padded(cx, cy, cz)).count;
            cy = cy + 1;
        }
        i = i + 1;
    }
    let meshed = unsafe monotonicMs();
    // Per chunk: 9 columns of 4 chunks generated, 4 chunks meshed.
    let generated = Float64(from: built - start) / Float64(from: runs * 9 * worldChunksY);
    let mesh = Float64(from: meshed - built) / Float64(from: runs * worldChunksY);
    println("\(runs) runs, \(faces) faces in the spawn column");
    println("generate: \(generated) ms per chunk");
    println("mesh:     \(mesh) ms per chunk");
    return 0;
}

/// The desktop driver: an SDL window and a `while` loop feeding the game
/// events and frames. A browser driver would do the same from callbacks.
func run() -> Int64 {
    if let .Some(runs) = getenv("AERIE_BENCH") {
        return bench(Int64(parsing: runs) ?? 10);
    }
    var window = SDLWindow(title: "Aerie", width: 1280, height: 720);
    let (width, height) = window.drawableSize;
    let renderer = match Renderer.create(window.nativeWindow, width: width, height: height) {
        .Ok(r) => r,
        .Err(e) => {
            println("wgpu error: \(e.message)");
            return 1;
        }
    };
    let path = savePath();
    let save = match path { .Some(p) => loadSave(p), .None => .None };
    var game = Game(renderer, seed: 1, save: save);
    let maxFrames = match getenv("AERIE_FRAMES") {
        .Some(s) => Int64(parsing: s) ?? 0,
        .None => 0
    };
    var last = unsafe monotonicMs();
    var frames = 0;
    var captured = false;
    while game.running {
        while let .Some(event) = window.pollEvent() {
            match event {
                .Resized(_, _) => {
                    let (w, h) = window.drawableSize;
                    game.renderer.resize(w, h);
                },
                _ => { game.event(event) }
            }
        }
        if game.mouseCaptured != captured {
            captured = game.mouseCaptured;
            window.setRelativeMouseMode(captured);
        }
        let now = unsafe monotonicMs();
        let dt = Float32(from: now - last) / 1000.0;
        last = now;
        match game.frame(dt) {
            .Ok(_) => {},
            .Err(e) => {
                println("wgpu error: \(e.message)");
                return 1;
            }
        }
        if game.wantsSave {
            game.wantsSave = false;
            if let .Some(p) = path {
                game.notify(if writeSave(p, game.save()) { "World saved" } else { "Couldn't save" });
            }
        }
        frames = frames + 1;
        endFrame();
        if maxFrames > 0 and frames >= maxFrames {
            game.running = false;
        }
    }
    if let .Some(p) = path { let _ = writeSave(p, game.save()); }
    println("drew \(frames) frames; \(game.world.chunks.count) chunks loaded, \(game.renderer.meshes.count) meshes, \(game.renderer.drawnChunks) drawn, \(game.flock.birds.count) birds");
    let p = game.player.position;
    println("player at (\(p.x), \(p.y), \(p.z)), on ground: \(game.player.onGround)");
    0
}

@main
func main() -> lang.i64 {
    if run() == 0 { 0 } else { 1 }
}
