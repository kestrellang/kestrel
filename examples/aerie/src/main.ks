// Kestrelcraft: a voxel sandbox on floating islands above a cloud sea.
//
// Walk with WASD, look with the mouse, jump with Space; Space again in
// midair launches you up on the wing-cloak to soar (look down to dive and
// gain speed, up to climb; Space boosts, Shift folds the cloak). Hold the left button to break the block you look at,
// right click to place the selected one; the wheel or 1 to 9 pick from the
// hotbar, F toggles flying (Space and Left Shift to rise and sink), Z shows
// frames per second. The world saves every minute; Escape frees the mouse,
// then saves and quits. The title screen waits for Enter or a click
// (`AERIE_PLAY=1` skips it).
//
// The world is saved to `world.aerie`, or the path in `AERIE_SAVE`
// (`AERIE_SAVE=none` turns saving off). `AERIE_FRAMES=n` quits after n
// frames and prints the frame rate and where frame time went;
// `AERIE_VSYNC=0` draws without waiting for the display. `AERIE_BENCH=n`
// times chunk generation and meshing over n runs instead of opening a
// window.
//
// Run from this folder, so `assets/blocks.png` resolves.

module Aerie

import sdl.(SDLWindow, Event, monotonicUs)

/// The desktop driver: an SDL window and a `while` loop feeding the game
/// events and frames. In a browser the same loop runs, handing control
/// back to the page at the end of each frame (`endFrame`).
func run() -> Int64 {
    let settings = Settings.fromEnvironment();
    if let .Some(runs) = settings.benchRuns {
        Bench.run(runs);
        return 0;
    }
    var window = SDLWindow(title: "Kestrelcraft", width: 1280, height: 720);
    let (width, height) = window.drawableSize;
    let renderer = match Renderer.create(window.nativeWindow, width: width, height: height, graphics: settings.graphics) {
        .Ok(r) => r,
        .Err(e) => {
            println("wgpu error: \(e.message)");
            return 1;
        }
    };
    let file = settings.savePath.map({ (path: String) in SaveFile(path: path) });
    let save = match file { .Some(f) => f.load(), .None => .None };
    var game = Game(renderer, settings: settings, seed: 1, save: save);
    let start = unsafe monotonicUs();
    var last = start;
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
        let now = unsafe monotonicUs();
        if let .Err(e) = game.frame(Float32(from: now - last) / 1000000.0) {
            println("wgpu error: \(e.message)");
            return 1;
        }
        last = now;
        if game.wantsSave {
            game.wantsSave = false;
            // Quietly: only a failure is worth a notice.
            if let .Some(f) = file {
                if not f.write(game.save()) { game.notify("Couldn't save"); }
            }
        }
        frames = frames + 1;
        endFrame();
        if settings.maxFrames > 0 and frames >= settings.maxFrames {
            game.running = false;
        }
    }
    if let .Some(f) = file { let _ = f.write(game.save()); }
    report(game, frames: frames, seconds: milliseconds(unsafe monotonicUs() - start) / 1000.0);
    0
}

/// What a run of `seconds` did, for checking a headless or scripted run.
func report(game: Game, frames frames: Int64, seconds seconds: Float64) {
    let meshes = game.renderer.chunks.meshes.count;
    let (drawn, draws) = (game.renderer.drawnChunks, game.renderer.chunkDraws);
    println("drew \(frames) frames; \(game.world.chunks.count) chunks loaded, \(meshes) meshes, \(drawn) drawn in \(draws) draws, \(game.flock.birds.count) birds");
    let p = game.player.position;
    println("\(Float64(from: frames) / seconds) fps; \(game.profile.summary())");
    println(game.renderer.gpuTimer.summary());
    println("player at (\(p.x), \(p.y), \(p.z)), on ground: \(game.player.onGround)");
}

@main
func main() -> lang.i64 {
    if run() == 0 { 0 } else { 1 }
}
