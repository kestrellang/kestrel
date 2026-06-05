// Conway's Game of Life — an SDL example.
//
// The world is a toroidal grid (cells wrap at the edges) drawn with square
// cells into a window sized to fit. Each tick applies the standard B3/S23
// rules to every cell using a double-buffered step so reads always see the
// previous generation.
//
// Usage:
//   life --width 80 --height 60 --cell-size 10
//     interactive SDL window. Bounds: 5..2000 board, 1..40 cellPx.
//   life --headless --iters 1000 --width 80 --height 60
//     no window; runs ITERS generations from a randomized board, prints
//     wall time and gens/sec, exits. Useful for benchmarking the solver.
//
// Controls:
//   1..5   pick a pattern: glider, blinker, LWSS, pulsar, Gosper gun
//   click  stamp the selected pattern centered on the clicked cell
//   Space  pause / resume
//   R      reseed with random noise
//   C      clear the grid (auto-pauses so you can plant patterns)
//   Esc    quit

module Life

import sdl.(Event)

// `gameLoop` drives a single object that both produces input events and renders.
// SdlGameRenderer owns the window and naturally does both, so it is passed once;
// the headless path wraps its two separate helpers in a HeadlessApp adapter.
//
// A single `mutating app` (rather than two params bound to the same `sdl`) keeps
// us from aliasing one resource-owning value into two mutable params, which would
// risk a double drop of the SDL handles at exit. The `A: not Copyable` bound is
// required: a generic protocol-bound param defaults to a `Copyable` requirement,
// so a non-Copyable concrete type (SdlGameRenderer wraps SDL pointers) only fits
// when the param explicitly opts out of copyability.
func gameLoop[A](mutating app: A, config cfg: Config) where A: InputManager, A: GameRenderer, A: not Copyable {
    var state = GameState(fromConfig: cfg);

    var timer = Timer.start();
    var simAccum: Int64 = 0;

    while state.running {
        while let .Some(event) = app.getEvent() {
            match event {
                .Quit => { state.running = false },
                .KeyDown(.Space) => { state.paused = not state.paused },
                .KeyDown(.R) => { state.randomize() },
                .KeyDown(.C) => { state.clear() },
                .KeyDown(.Digit1) => { state.selectedPattern = .Glider },
                .KeyDown(.Digit2) => { state.selectedPattern = .Blinker },
                .KeyDown(.Digit3) => { state.selectedPattern = .Lwss },
                .KeyDown(.Digit4) => { state.selectedPattern = .Pulsar },
                .KeyDown(.Digit5) => { state.selectedPattern = .GosperGun },
                .KeyDown(.Escape) => { state.running = false },
                .MouseDown(px, py) => { state.stamp(px, py, cfg.cellSize) },
                _ => {}
            }
        }

        // Quit/Escape (or headless exhausting its iters) stops us before the
        // generation step, so `--iters N` runs exactly N generations and an
        // interactive quit doesn't burn one more step/render frame.
        if not state.running { break; }

        simAccum = simAccum + timer.tick();
        if not state.paused and (state.stepDelayMs == 0 or simAccum >= state.stepDelayMs) {
            state.grid.step();
            state.generation = state.generation + 1;
            simAccum = 0;
        }

        state.updateFps(timer.elapsed());
        app.render(state);
    }

    app.finish(state, elapsed: timer.elapsed());
}

@main
func main() {
    let cfg = Config.fromArgs();
    if cfg.headlessIters > 0 {
        var app = HeadlessApp(config: cfg);
        gameLoop(app, config: cfg);
    } else {
        var sdl = SdlGameRenderer(config: cfg);
        gameLoop(sdl, config: cfg);
    }
}
