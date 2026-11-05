// Aerie: a voxel sandbox on floating islands above a cloud sea.
//
// Milestone 1 draws a hand-built test island from the block atlas with a
// fly camera: WASD, Space and Left Shift to move, arrow keys to look,
// Escape to quit. `AERIE_FRAMES=n` quits after n frames; `AERIE_BENCH=n`
// times chunk generation and meshing over n runs instead of opening a window.
//
// Run from this folder, so `assets/blocks.png` resolves.

module Aerie

import sdl.(SDLWindow, Event, monotonicMs)

/// Time building and meshing the test island `runs` times, without a
/// window, and print the average per chunk.
func bench(runs: Int64) -> Int64 {
    var start = unsafe monotonicMs();
    var island = testIsland();
    var i = 1;
    while i < runs {
        island = testIsland();
        i = i + 1;
    }
    let built = unsafe monotonicMs();
    var faces = 0;
    i = 0;
    while i < runs {
        faces = meshChunk(island).count;
        i = i + 1;
    }
    let meshed = unsafe monotonicMs();
    println("\(runs) runs, \(faces) faces per chunk");
    println("generate: \(Float64(from: built - start) / Float64(from: runs)) ms per chunk");
    println("mesh:     \(Float64(from: meshed - built) / Float64(from: runs)) ms per chunk");
    0
}

/// The desktop driver: an SDL window and a `while` loop feeding the game
/// events and frames. A browser driver would do the same from callbacks.
func run() -> Int64 {
    if let .Some(runs) = getenv("AERIE_BENCH") {
        return bench(Int64(parsing: runs) ?? 10);
    }
    var window = SDLWindow(title: "Aerie", width: 1280, height: 720);
    let (width, height) = window.drawableSize;
    let renderer = match Renderer.create(window.nativeWindow, width: width, height: height, atlasPath: "assets/blocks.png") {
        .Ok(r) => r,
        .Err(e) => {
            println("wgpu error: \(e.message)");
            return 1;
        }
    };
    var game = Game(renderer);
    let maxFrames = match getenv("AERIE_FRAMES") {
        .Some(s) => Int64(parsing: s) ?? 0,
        .None => 0
    };
    var last = unsafe monotonicMs();
    var frames = 0;
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
        frames = frames + 1;
        if maxFrames > 0 and frames >= maxFrames {
            game.running = false;
        }
    }
    println("drew \(frames) frames");
    0
}

@main
func main() -> lang.i64 {
    if run() == 0 { 0 } else { 1 }
}
