// Conway's Game of Life — an SDL example.
//
// The world is a toroidal grid (cells wrap at the edges) drawn with square
// cells into a window sized to fit. Each tick applies the standard B3/S23
// rules to every cell using a double-buffered step so reads always see the
// previous generation.
//
// Usage:
//   life [width] [height] [cellPx]
//     interactive SDL window. Bounds: 5..2000 board, 1..40 cellPx.
//   life --headless ITERS [width] [height]
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

import Sdl.(Color, Rectangle, Milliseconds, Key, Event, Renderer, SDLApp)

@extern(.C, mangleName: "Kestrel_Argc")
func cliArgc() -> Int32

@extern(.C, mangleName: "Kestrel_GetArg")
func cliGetArg(idx: Int32) -> CString

@extern(.C, mangleName: "Kestrel_GetTicks")
func sdlGetTicks() -> UInt32

@extern(.C, mangleName: "Kestrel_MonotonicMs")
func monotonicMs() -> Int64

struct Config {
    var width: Int64
    var height: Int64
    var cellSize: Int64
    var stepDelayMs: Int64
    // 0 means interactive; >0 runs N headless generations and exits.
    var headlessIters: Int64
}

// Reads argv slot `idx` into a String, returning the empty string if missing
// or null. Centralises the CString → String dance so the parser body stays
// readable.
func argString(idx: Int32) -> String {
    let cstr = cliGetArg(idx);
    if cstr.isNull { "" } else { String(from: cstr) }
}

// Picks the largest cell size (in px) that keeps the board inside ~800x600,
// clamped to [2, 40]. Smaller boards stay readable, bigger ones still fit.
func autoCellSize(width w: Int64, height h: Int64) -> Int64 {
    let byW = 800 / w;
    let byH = 600 / h;
    var c = if byW < byH { byW } else { byH };
    if c < 1 { c = 1; }
    if c > 40 { c = 40; }
    c
}

// Reads CLI args. Two layouts:
//   life [width] [height] [cellPx]
//   life --headless ITERS [width] [height]
// Out-of-range or unparseable values are silently ignored so the binary
// still launches. `cellPx` is auto-fitted unless the user overrides it.
func parseConfig() -> Config {
    var width: Int64 = 80;
    var height: Int64 = 60;
    var cellOverride: Int64 = 0;
    var headlessIters: Int64 = 0;
    let argc = cliArgc();

    // Detect headless mode: argv[1] == "--headless", argv[2] == iterations.
    // Width/height shift to argv[3]/argv[4] in that case.
    var posStart: Int32 = Int32(intLiteral: 1);
    if argc >= Int32(intLiteral: 3) and argString(Int32(intLiteral: 1)).equals("--headless") {
        if let .Some(n) = Int64.parse(argString(Int32(intLiteral: 2))) {
            if n > 0 {
                headlessIters = n;
            }
        }
        posStart = Int32(intLiteral: 3);
    }

    if argc >= posStart + Int32(intLiteral: 1) {
        if let .Some(w) = Int64.parse(argString(posStart)) {
            if w >= 5 and w <= 2000 {
                width = w;
            }
        }
    }
    if argc >= posStart + Int32(intLiteral: 2) {
        if let .Some(h) = Int64.parse(argString(posStart + Int32(intLiteral: 1))) {
            if h >= 5 and h <= 2000 {
                height = h;
            }
        }
    }
    // cellPx only applies in interactive mode.
    if headlessIters == 0 and argc >= posStart + Int32(intLiteral: 3) {
        if let .Some(c) = Int64.parse(argString(posStart + Int32(intLiteral: 2))) {
            if c >= 1 and c <= 40 {
                cellOverride = c;
            }
        }
    }
    let cellSize = if cellOverride > 0 {
        cellOverride
    } else {
        autoCellSize(width: width, height: height)
    };
    Config(
        width: width,
        height: height,
        cellSize: cellSize,
        stepDelayMs: 80,
        headlessIters: headlessIters
    )
}

struct Grid {
    var width: Int64
    var height: Int64
    var cellSize: Int64
    var cells: Array[Bool]
    var next: Array[Bool]

    init(width w: Int64, height h: Int64, cellSize c: Int64) {
        let n = w * h;
        self.width = w;
        self.height = h;
        self.cellSize = c;
        self.cells = Array[Bool](repeating: false, count: n);
        self.next = Array[Bool](repeating: false, count: n);
    }

    // Toroidal wrap so a glider that walks off one edge re-enters from the
    // opposite side. The double-modulo handles negative inputs from the
    // neighbour scan.
    func index(x x: Int64, y y: Int64) -> Int64 {
        let w = self.width;
        let h = self.height;
        let xx = (x.modulo(w) + w).modulo(w);
        let yy = (y.modulo(h) + h).modulo(h);
        yy * w + xx
    }

    func cellAt(x x: Int64, y y: Int64) -> Bool {
        self.cells(self.index(x: x, y: y))
    }

    mutating func setCell(x x: Int64, y y: Int64, alive alive: Bool) {
        let i = self.index(x: x, y: y);
        self.cells(i) = alive;
    }

    func neighborCount(x x: Int64, y y: Int64) -> Int64 {
        var count: Int64 = 0;
        var dy: Int64 = 0 - 1;
        while dy <= 1 {
            var dx: Int64 = 0 - 1;
            while dx <= 1 {
                if not (dx == 0 and dy == 0) {
                    if self.cellAt(x: x + dx, y: y + dy) { count = count + 1; }
                }
                dx = dx + 1;
            }
            dy = dy + 1;
        }
        count
    }

    mutating func step() {
        var y: Int64 = 0;
        while y < self.height {
            var x: Int64 = 0;
            while x < self.width {
                let alive = self.cellAt(x: x, y: y);
                let n = self.neighborCount(x: x, y: y);
                // B3/S23: birth on exactly 3 live neighbours; an already-live
                // cell survives with 2 or 3.
                let nextAlive = if alive { n == 2 or n == 3 } else { n == 3 };
                self.next(self.index(x: x, y: y)) = nextAlive;
                x = x + 1;
            }
            y = y + 1;
        }
        // Swap buffers so the next read sees the new generation. The old
        // `cells` array becomes the scratch buffer for the following step.
        let tmp = self.cells;
        self.cells = self.next;
        self.next = tmp;
    }

    mutating func clear() {
        var i: Int64 = 0;
        while i < self.cells.count {
            self.cells(i) = false;
            i = i + 1;
        }
    }

    // ~30% density gives a busy field that takes a while to settle but
    // doesn't immediately collapse to overpopulation. The rng is owned
    // locally so we don't depend on `mutating`-mode argument plumbing.
    mutating func randomize(seed seed: UInt64) {
        var rng = Lcg64(seed: seed);
        var i: Int64 = 0;
        while i < self.cells.count {
            self.cells(i) = rng.nextInt(below: 10) < 3;
            i = i + 1;
        }
    }

    // === Patterns ===
    //
    // Each `stamp*` method draws the pattern centred at `(cx, cy)`. Bounds
    // wrap toroidally via `setCell`, so it doesn't matter if the click was
    // close to the edge.

    mutating func stampGlider(centerX cx: Int64, centerY cy: Int64) {
        let x = cx - 1; let y = cy - 1;
        self.setCell(x: x + 1, y: y + 0, alive: true);
        self.setCell(x: x + 2, y: y + 1, alive: true);
        self.setCell(x: x + 0, y: y + 2, alive: true);
        self.setCell(x: x + 1, y: y + 2, alive: true);
        self.setCell(x: x + 2, y: y + 2, alive: true);
    }

    mutating func stampBlinker(centerX cx: Int64, centerY cy: Int64) {
        // 3x1 horizontal — period-2 oscillator.
        self.setCell(x: cx - 1, y: cy, alive: true);
        self.setCell(x: cx,     y: cy, alive: true);
        self.setCell(x: cx + 1, y: cy, alive: true);
    }

    mutating func stampLwss(centerX cx: Int64, centerY cy: Int64) {
        // Light-Weight Spaceship, 5x4. Travels left-to-right.
        let x = cx - 2; let y = cy - 2;
        self.setCell(x: x + 1, y: y + 0, alive: true);
        self.setCell(x: x + 4, y: y + 0, alive: true);
        self.setCell(x: x + 0, y: y + 1, alive: true);
        self.setCell(x: x + 0, y: y + 2, alive: true);
        self.setCell(x: x + 4, y: y + 2, alive: true);
        self.setCell(x: x + 0, y: y + 3, alive: true);
        self.setCell(x: x + 1, y: y + 3, alive: true);
        self.setCell(x: x + 2, y: y + 3, alive: true);
        self.setCell(x: x + 3, y: y + 3, alive: true);
    }

    mutating func stampPulsar(centerX cx: Int64, centerY cy: Int64) {
        // 13x13 period-3 oscillator. Built from four mirrored arms; we
        // unroll all 48 lit cells rather than trying to share code with
        // arithmetic loops.
        let x = cx - 6; let y = cy - 6;
        // top horizontals
        self.setCell(x: x + 2, y: y + 0, alive: true);
        self.setCell(x: x + 3, y: y + 0, alive: true);
        self.setCell(x: x + 4, y: y + 0, alive: true);
        self.setCell(x: x + 8, y: y + 0, alive: true);
        self.setCell(x: x + 9, y: y + 0, alive: true);
        self.setCell(x: x + 10, y: y + 0, alive: true);
        // upper sides
        self.setCell(x: x + 0, y: y + 2, alive: true);
        self.setCell(x: x + 5, y: y + 2, alive: true);
        self.setCell(x: x + 7, y: y + 2, alive: true);
        self.setCell(x: x + 12, y: y + 2, alive: true);
        self.setCell(x: x + 0, y: y + 3, alive: true);
        self.setCell(x: x + 5, y: y + 3, alive: true);
        self.setCell(x: x + 7, y: y + 3, alive: true);
        self.setCell(x: x + 12, y: y + 3, alive: true);
        self.setCell(x: x + 0, y: y + 4, alive: true);
        self.setCell(x: x + 5, y: y + 4, alive: true);
        self.setCell(x: x + 7, y: y + 4, alive: true);
        self.setCell(x: x + 12, y: y + 4, alive: true);
        // middle horizontals
        self.setCell(x: x + 2, y: y + 5, alive: true);
        self.setCell(x: x + 3, y: y + 5, alive: true);
        self.setCell(x: x + 4, y: y + 5, alive: true);
        self.setCell(x: x + 8, y: y + 5, alive: true);
        self.setCell(x: x + 9, y: y + 5, alive: true);
        self.setCell(x: x + 10, y: y + 5, alive: true);
        self.setCell(x: x + 2, y: y + 7, alive: true);
        self.setCell(x: x + 3, y: y + 7, alive: true);
        self.setCell(x: x + 4, y: y + 7, alive: true);
        self.setCell(x: x + 8, y: y + 7, alive: true);
        self.setCell(x: x + 9, y: y + 7, alive: true);
        self.setCell(x: x + 10, y: y + 7, alive: true);
        // lower sides
        self.setCell(x: x + 0, y: y + 8, alive: true);
        self.setCell(x: x + 5, y: y + 8, alive: true);
        self.setCell(x: x + 7, y: y + 8, alive: true);
        self.setCell(x: x + 12, y: y + 8, alive: true);
        self.setCell(x: x + 0, y: y + 9, alive: true);
        self.setCell(x: x + 5, y: y + 9, alive: true);
        self.setCell(x: x + 7, y: y + 9, alive: true);
        self.setCell(x: x + 12, y: y + 9, alive: true);
        self.setCell(x: x + 0, y: y + 10, alive: true);
        self.setCell(x: x + 5, y: y + 10, alive: true);
        self.setCell(x: x + 7, y: y + 10, alive: true);
        self.setCell(x: x + 12, y: y + 10, alive: true);
        // bottom horizontals
        self.setCell(x: x + 2, y: y + 12, alive: true);
        self.setCell(x: x + 3, y: y + 12, alive: true);
        self.setCell(x: x + 4, y: y + 12, alive: true);
        self.setCell(x: x + 8, y: y + 12, alive: true);
        self.setCell(x: x + 9, y: y + 12, alive: true);
        self.setCell(x: x + 10, y: y + 12, alive: true);
    }

    mutating func stampGosperGun(centerX cx: Int64, centerY cy: Int64) {
        // Gosper glider gun: 36x9. Emits a glider every 30 generations.
        let x = cx - 18; let y = cy - 4;
        // Left block
        self.setCell(x: x + 0, y: y + 4, alive: true);
        self.setCell(x: x + 1, y: y + 4, alive: true);
        self.setCell(x: x + 0, y: y + 5, alive: true);
        self.setCell(x: x + 1, y: y + 5, alive: true);
        // Left arm
        self.setCell(x: x + 10, y: y + 4, alive: true);
        self.setCell(x: x + 10, y: y + 5, alive: true);
        self.setCell(x: x + 10, y: y + 6, alive: true);
        self.setCell(x: x + 11, y: y + 3, alive: true);
        self.setCell(x: x + 11, y: y + 7, alive: true);
        self.setCell(x: x + 12, y: y + 2, alive: true);
        self.setCell(x: x + 12, y: y + 8, alive: true);
        self.setCell(x: x + 13, y: y + 2, alive: true);
        self.setCell(x: x + 13, y: y + 8, alive: true);
        self.setCell(x: x + 14, y: y + 5, alive: true);
        self.setCell(x: x + 15, y: y + 3, alive: true);
        self.setCell(x: x + 15, y: y + 7, alive: true);
        self.setCell(x: x + 16, y: y + 4, alive: true);
        self.setCell(x: x + 16, y: y + 5, alive: true);
        self.setCell(x: x + 16, y: y + 6, alive: true);
        self.setCell(x: x + 17, y: y + 5, alive: true);
        // Right arm
        self.setCell(x: x + 20, y: y + 2, alive: true);
        self.setCell(x: x + 20, y: y + 3, alive: true);
        self.setCell(x: x + 20, y: y + 4, alive: true);
        self.setCell(x: x + 21, y: y + 2, alive: true);
        self.setCell(x: x + 21, y: y + 3, alive: true);
        self.setCell(x: x + 21, y: y + 4, alive: true);
        self.setCell(x: x + 22, y: y + 1, alive: true);
        self.setCell(x: x + 22, y: y + 5, alive: true);
        self.setCell(x: x + 24, y: y + 0, alive: true);
        self.setCell(x: x + 24, y: y + 1, alive: true);
        self.setCell(x: x + 24, y: y + 5, alive: true);
        self.setCell(x: x + 24, y: y + 6, alive: true);
        // Right block
        self.setCell(x: x + 34, y: y + 2, alive: true);
        self.setCell(x: x + 34, y: y + 3, alive: true);
        self.setCell(x: x + 35, y: y + 2, alive: true);
        self.setCell(x: x + 35, y: y + 3, alive: true);
    }

    mutating func stamp(kind kind: Int64, centerX cx: Int64, centerY cy: Int64) {
        if kind == 0 {
            self.stampGlider(centerX: cx, centerY: cy);
        } else if kind == 1 {
            self.stampBlinker(centerX: cx, centerY: cy);
        } else if kind == 2 {
            self.stampLwss(centerX: cx, centerY: cy);
        } else if kind == 3 {
            self.stampPulsar(centerX: cx, centerY: cy);
        } else if kind == 4 {
            self.stampGosperGun(centerX: cx, centerY: cy);
        }
    }

    func render(renderer: Renderer) {
        let cell = self.cellSize;
        // Set the cell color once so the inner loop only issues fillRect
        // calls. With dense boards (e.g. 200x150) this halves the SDL
        // syscalls per frame.
        renderer.setColor(Color.green());
        // Drop the 1px gutter once cells get small — it eats the cell.
        let gutter = if cell >= 4 { 1 } else { 0 };
        var y: Int64 = 0;
        while y < self.height {
            var x: Int64 = 0;
            while x < self.width {
                if self.cellAt(x: x, y: y) {
                    let rect = Rectangle(
                        x: x * cell,
                        y: y * cell,
                        width: cell - gutter,
                        height: cell - gutter
                    );
                    renderer.fillRect(rect);
                }
                x = x + 1;
            }
            y = y + 1;
        }
    }
}

func patternName(kind kind: Int64) -> String {
    if kind == 0 {
        "GLIDER"
    } else if kind == 1 {
        "BLINKER"
    } else if kind == 2 {
        "LWSS"
    } else if kind == 3 {
        "PULSAR"
    } else {
        "GOSPER GUN"
    }
}

// Headless mode: no window, no SDL. Times N generations on a randomized
// board and prints `gens / ms / gens-per-sec`. Exits 0.
func runHeadless(cfg: Config) -> Int32 {
    var grid = Grid(width: cfg.width, height: cfg.height, cellSize: 1);
    grid.randomize(seed: UInt64(intLiteral: 12648430));

    let start = monotonicMs();
    var i: Int64 = 0;
    while i < cfg.headlessIters {
        grid.step();
        i = i + 1;
    }
    let elapsedMs = monotonicMs() - start;
    // Avoid divide-by-zero on instant runs.
    let denom = if elapsedMs > 0 { elapsedMs } else { 1 };
    let gensPerSec = cfg.headlessIters * 1000 / denom;

    let _ = std.io.stdio.println(
        cfg.width.format() + "x" + cfg.height.format() +
        "  gens=" + cfg.headlessIters.format() +
        "  elapsed_ms=" + elapsedMs.format() +
        "  gens_per_sec=" + gensPerSec.format()
    );
    0
}

func main() -> Int32 {
    let cfg = parseConfig();
    if cfg.headlessIters > 0 {
        return runHeadless(cfg);
    }
    let windowW = cfg.width * cfg.cellSize;
    let windowH = cfg.height * cfg.cellSize;
    var app = SDLApp(title: "Game of Life", width: windowW, height: windowH);
    var grid = Grid(width: cfg.width, height: cfg.height, cellSize: cfg.cellSize);
    var paused = false;
    var running = true;
    // 0..4 — see `patternName` for the labels.
    var selectedPattern: Int64 = 0;
    // Bumped each time we reseed so successive R-presses give a different
    // pattern instead of replaying the same starting field.
    var seedCounter: UInt64 = UInt64(intLiteral: 12648430);

    grid.randomize(seed: seedCounter);

    // FPS sampling: count frames between ticks, refresh the displayed value
    // once per ~500ms so it doesn't flicker.
    var frameCount: Int64 = 0;
    var fpsLast: UInt32 = sdlGetTicks();
    var displayedFps: Int64 = 0;

    // Sim is paced independently of render: render runs vsync-capped (so the
    // FPS counter is meaningful), the simulation only advances when at least
    // `stepDelayMs` has elapsed since the last generation. Without this, a
    // 30k-cell board at vsync would step 60+ generations/sec — too fast to
    // watch.
    let simStepMs = UInt32(from: cfg.stepDelayMs);
    var simLast: UInt32 = sdlGetTicks();

    while running {
        while let .Some(event) = app.pollEvent() {
            match event {
                .Quit => { running = false },
                .KeyDown(key) => {
                    match key {
                        .Space => { paused = not paused },
                        .R => {
                            seedCounter = seedCounter + UInt64(intLiteral: 1);
                            grid.randomize(seed: seedCounter);
                            paused = false;
                        },
                        .C => {
                            grid.clear();
                            paused = true;
                        },
                        .Digit1 => { selectedPattern = 0 },
                        .Digit2 => { selectedPattern = 1 },
                        .Digit3 => { selectedPattern = 2 },
                        .Digit4 => { selectedPattern = 3 },
                        .Digit5 => { selectedPattern = 4 },
                        .Escape => { running = false },
                        _ => {}
                    }
                },
                .KeyUp(_) => {},
                .MouseDown(px, py) => {
                    let gx = px / cfg.cellSize;
                    let gy = py / cfg.cellSize;
                    grid.stamp(kind: selectedPattern, centerX: gx, centerY: gy);
                }
            }
        }

        let now = sdlGetTicks();
        if not paused and (now - simLast) >= simStepMs {
            grid.step();
            simLast = now;
        }

        // Refresh the FPS readout every ~500ms.
        frameCount = frameCount + 1;
        let elapsed = Int64(from: now - fpsLast);
        if elapsed >= 500 {
            displayedFps = frameCount * 1000 / elapsed;
            frameCount = 0;
            fpsLast = now;
        }

        app.render { (renderer) in
            renderer.clear(Color.black());
            grid.render(renderer);
            // HUD: current pattern in the top-left, FPS just below.
            renderer.drawText("PATTERN: " + patternName(kind: selectedPattern), 8, 8, 2);
            renderer.drawText("FPS: " + displayedFps.format(), 8, 28, 2);
        };
    }

    0
}
