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

import Sdl.(Key, Event)

func handleEvent(event: Event, mutating state s: GameState, config cfg: Config) {
    match event {
        .Quit => { s.running = false },
        .KeyDown(key) => {
            match key {
                .Space => { s.paused = not s.paused },
                .R => {
                    s.seedCounter = s.seedCounter + 1;
                    s.grid.randomize(seed: s.seedCounter);
                    s.paused = false;
                },
                .C => {
                    s.grid.clear();
                    s.paused = true;
                },
                .Digit1 => { s.selectedPattern = .Glider },
                .Digit2 => { s.selectedPattern = .Blinker },
                .Digit3 => { s.selectedPattern = .Lwss },
                .Digit4 => { s.selectedPattern = .Pulsar },
                .Digit5 => { s.selectedPattern = .GosperGun },
                .Escape => { s.running = false },
                _ => {}
            }
        },
        .KeyUp(_) => {},
        .MouseDown(px, py) => {
            let gx = px / cfg.cellSize;
            let gy = py / cfg.cellSize;
            s.selectedPattern.stamp(on: s.grid, centerX: gx, centerY: gy);
        }
    }
}

func gameLoop[I, R](mutating input i: I, mutating renderer r: R, config cfg: Config) where I: InputManager, R: GameRenderer {
    var state = GameState(
        grid: Grid(width: cfg.width, height: cfg.height),
        paused: false,
        running: true,
        selectedPattern: .Glider,
        seedCounter: 12648430,
        fps: 0,
        stepDelayMs: cfg.stepDelayMs,
        generation: 0,
        frameCount: 0,
        lastFpsMs: 0
    );
    state.grid.randomize(seed: state.seedCounter);

    var timer = Timer.start();

    while state.running {
        while let .Some(event) = i.getEvent() {
            handleEvent(event, state: state, config: cfg);
        }

        let dt = timer.tick();
        if not state.paused and (state.stepDelayMs == 0 or dt >= state.stepDelayMs) {
            state.grid.step();
            state.generation = state.generation + 1;
        }

        state.updateFps(timer.elapsed());
        r.render(state);
    }

    r.finish(state, elapsed: timer.elapsed());
}

func main() -> Int32 {
    let cfg = Config.fromArgs();
    if cfg.headlessIters > 0 {
        var input = HeadlessInputManager(cfg.headlessIters);
        var renderer = HeadlessRenderer(config: cfg);
        gameLoop(input: input, renderer: renderer, config: cfg);
    } else {
        var sdl = SdlGameRenderer(config: cfg);
        gameLoop(input: sdl, renderer: sdl, config: cfg);
    }
    0
}
