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

import Sdl.(Color, Rectangle, Milliseconds, Key, Event, Renderer, SDLApp)
import clutch.command.(Command)
import clutch.arg.(Arg)
import clutch.matches.(ArgMatches)
import clutch.os.(getArgv)

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

func buildCommand() -> Command {
    var cmd = Command(name: "life");
    cmd.setAbout(text: "Conway's Game of Life — an SDL example");

    var width = Arg(name: "width");
    width.short(flag: "W");
    width.help(text: "Board width (5..2000)");
    width.defaultsTo(value: "80");
    cmd.addArg(arg: width);

    var height = Arg(name: "height");
    height.short(flag: "H");
    height.help(text: "Board height (5..2000)");
    height.defaultsTo(value: "60");
    cmd.addArg(arg: height);

    var cellSize = Arg(name: "cell-size");
    cellSize.short(flag: "c");
    cellSize.help(text: "Cell size in pixels (1..40, auto if omitted)");
    cmd.addArg(arg: cellSize);

    var headless = Arg(name: "headless");
    headless.asFlag();
    headless.help(text: "Run headless benchmark (no window)");
    cmd.addArg(arg: headless);

    var iters = Arg(name: "iters");
    iters.short(flag: "n");
    iters.help(text: "Number of generations for headless mode");
    iters.defaultsTo(value: "1000");
    cmd.addArg(arg: iters);

    cmd
}

func clampParse(value: Optional[String], min lo: Int64, max hi: Int64, default fallback: Int64) -> Int64 {
    guard let .Some(s) = value else {
        return fallback;
    }

    guard let .Some(n) = Int64.parse(s) else {
        return fallback;
    }

    if n < lo or n > hi {
        return fallback
    }

    return n;
}

func parseConfig() -> Config {
    let cmd = buildCommand();
    let args = getArgv();

    guard let .Ok(matches) = cmd.parse(tokens: args) else {
        return Config(
            width: 80,
            height: 60,
            cellSize: autoCellSize(width: 80, height: 60),
            stepDelayMs: 80,
            headlessIters: 0
        )
    }

    let width = clampParse(matches.getValue(name: "width"), min: 5, max: 2000, default: 80);
    let height = clampParse(matches.getValue(name: "height"), min: 5, max: 2000, default: 60);

    let headlessIters = if matches.hasFlag(name: "headless") {
        clampParse(matches.getValue(name: "iters"), min: 1, max: 999999999, default: 1000)
    } else {
        0
    };

    let cellSize = if headlessIters > 0 {
        1
    } else {
        clampParse(matches.getValue(name: "cell-size"), min: 1, max: 40, default: 0)
    };

    let finalCell = if cellSize > 0 { cellSize } else { autoCellSize(width: width, height: height) };

    Config(
        width: width,
        height: height,
        cellSize: finalCell,
        stepDelayMs: 80,
        headlessIters: headlessIters
    )
}

struct GameState {
    var grid: Grid
    var paused: Bool
    var running: Bool
    var selectedPattern: Pattern
    var seedCounter: UInt64
    var fps: Int64
}

protocol GameRenderer {
    mutating func render(state: GameState)
}

func renderGrid(grid: Grid, renderer: Renderer, cellSize cell: Int64) {
    renderer.setColor(Color.green());
    let gutter = if cell >= 4 { 1 } else { 0 };
    var y: Int64 = 0;
    while y < grid.height {
        var x: Int64 = 0;
        while x < grid.width {
            if grid.cellAt(x: x, y: y) {
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

struct HeadlessRenderer: GameRenderer {
    init(config cfg: Config) {}

    mutating func render(state: GameState) {}
}

enum Pattern: Formattable {
    case Glider
    case Blinker
    case Lwss
    case Pulsar
    case GosperGun

    func format(options: FormatOptions = FormatOptions.default()) -> String {
        match self {
            .Glider => "GLIDER",
            .Blinker => "BLINKER",
            .Lwss => "LWSS",
            .Pulsar => "PULSAR",
            .GosperGun => "GOSPER GUN"
        }
    }
}

struct Grid {
    var width: Int64
    var height: Int64
    var cells: Array[Bool]
    var next: Array[Bool]

    init(width w: Int64, height h: Int64) {
        let n = w * h;
        self.width = w;
        self.height = h;
        self.cells = Array[Bool](repeating: false, count: n);
        self.next = Array[Bool](repeating: false, count: n);
    }

    // Toroidal wrap so a glider that walks off one edge re-enters from the
    // opposite side.
    func index(x x: Int64, y y: Int64) -> Int64 {
        let w = self.width;
        let h = self.height;
        let xx = (x % w + w) % w;
        let yy = (y % h + h) % h;
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
        for y in 0..<self.height {
            for x in 0..<self.width {
                let alive = self.cellAt(x: x, y: y);
                let n = self.neighborCount(x: x, y: y);
                // B3/S23: birth on exactly 3 live neighbours; an already-live
                // cell survives with 2 or 3.
                let nextAlive = if alive { n == 2 or n == 3 } else { n == 3 };
                self.next(self.index(x: x, y: y)) = nextAlive;
            }
        }
        let tmp = self.cells;
        self.cells = self.next;
        self.next = tmp;
    }

    mutating func clear() {
        for i in 0..<self.cells.count {
            self.cells(i) = false;
        }
    }

    mutating func randomize(seed seed: UInt64) {
        var rng = Lcg64(seed: seed);
        var i: Int64 = 0;
        while i < self.cells.count {
            self.cells(i) = rng.nextInt(below: 10) < 3;
            i = i + 1;
        }
    }

    // === Patterns ===

    mutating func stampGlider(centerX cx: Int64, centerY cy: Int64) {
        let x = cx - 1; let y = cy - 1;
        self.setCell(x: x + 1, y: y + 0, alive: true);
        self.setCell(x: x + 2, y: y + 1, alive: true);
        self.setCell(x: x + 0, y: y + 2, alive: true);
        self.setCell(x: x + 1, y: y + 2, alive: true);
        self.setCell(x: x + 2, y: y + 2, alive: true);
    }

    mutating func stampBlinker(centerX cx: Int64, centerY cy: Int64) {
        self.setCell(x: cx - 1, y: cy, alive: true);
        self.setCell(x: cx,     y: cy, alive: true);
        self.setCell(x: cx + 1, y: cy, alive: true);
    }

    mutating func stampLwss(centerX cx: Int64, centerY cy: Int64) {
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
        let x = cx - 6; let y = cy - 6;
        self.setCell(x: x + 2, y: y + 0, alive: true);
        self.setCell(x: x + 3, y: y + 0, alive: true);
        self.setCell(x: x + 4, y: y + 0, alive: true);
        self.setCell(x: x + 8, y: y + 0, alive: true);
        self.setCell(x: x + 9, y: y + 0, alive: true);
        self.setCell(x: x + 10, y: y + 0, alive: true);
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
        self.setCell(x: x + 2, y: y + 12, alive: true);
        self.setCell(x: x + 3, y: y + 12, alive: true);
        self.setCell(x: x + 4, y: y + 12, alive: true);
        self.setCell(x: x + 8, y: y + 12, alive: true);
        self.setCell(x: x + 9, y: y + 12, alive: true);
        self.setCell(x: x + 10, y: y + 12, alive: true);
    }

    mutating func stampGosperGun(centerX cx: Int64, centerY cy: Int64) {
        let x = cx - 18; let y = cy - 4;
        self.setCell(x: x + 0, y: y + 4, alive: true);
        self.setCell(x: x + 1, y: y + 4, alive: true);
        self.setCell(x: x + 0, y: y + 5, alive: true);
        self.setCell(x: x + 1, y: y + 5, alive: true);
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
        self.setCell(x: x + 34, y: y + 2, alive: true);
        self.setCell(x: x + 34, y: y + 3, alive: true);
        self.setCell(x: x + 35, y: y + 2, alive: true);
        self.setCell(x: x + 35, y: y + 3, alive: true);
    }

    mutating func stamp(pattern pattern: Pattern, centerX cx: Int64, centerY cy: Int64) {
        match pattern {
            .Glider => self.stampGlider(centerX: cx, centerY: cy),
            .Blinker => self.stampBlinker(centerX: cx, centerY: cy),
            .Lwss => self.stampLwss(centerX: cx, centerY: cy),
            .Pulsar => self.stampPulsar(centerX: cx, centerY: cy),
            .GosperGun => self.stampGosperGun(centerX: cx, centerY: cy)
        }
    }

}

func runHeadless(cfg: Config) -> Int32 {
    var state = GameState(
        grid: Grid(width: cfg.width, height: cfg.height),
        paused: false,
        running: true,
        selectedPattern: .Glider,
        seedCounter: 12648430,
        fps: 0
    );
    state.grid.randomize(seed: state.seedCounter);
    let renderer = HeadlessRenderer(config: cfg);

    let start = monotonicMs();
    var i: Int64 = 0;
    while i < cfg.headlessIters {
        state.grid.step();
        renderer.render(state);
        i = i + 1;
    }
    let elapsedMs = monotonicMs() - start;
    let denom = if elapsedMs > 0 { elapsedMs } else { 1 };
    let gensPerSec = cfg.headlessIters * 1000 / denom;

    println("\(cfg.width)x\(cfg.height)  gens=\(cfg.headlessIters)  elapsed_ms=\(elapsedMs)  gens_per_sec=\(gensPerSec)");
    0
}

func runInteractive(cfg: Config) -> Int32 {
    let windowW = cfg.width * cfg.cellSize;
    let windowH = cfg.height * cfg.cellSize;
    var app = SDLApp(title: "Game of Life", width: windowW, height: windowH);
    let cellSize = cfg.cellSize;
    var state = GameState(
        grid: Grid(width: cfg.width, height: cfg.height),
        paused: false,
        running: true,
        selectedPattern: .Glider,
        seedCounter: 12648430,
        fps: 0
    );
    state.grid.randomize(seed: state.seedCounter);

    var frameCount: Int64 = 0;
    var fpsLast: UInt32 = sdlGetTicks();

    let simStepMs = UInt32(from: cfg.stepDelayMs);
    var simLast: UInt32 = sdlGetTicks();

    while state.running {
        while let .Some(event) = app.pollEvent() {
            match event {
                .Quit => { state.running = false },
                .KeyDown(key) => {
                    match key {
                        .Space => { state.paused = not state.paused },
                        .R => {
                            state.seedCounter = state.seedCounter + 1;
                            state.grid.randomize(seed: state.seedCounter);
                            state.paused = false;
                        },
                        .C => {
                            state.grid.clear();
                            state.paused = true;
                        },
                        .Digit1 => { state.selectedPattern = .Glider },
                        .Digit2 => { state.selectedPattern = .Blinker },
                        .Digit3 => { state.selectedPattern = .Lwss },
                        .Digit4 => { state.selectedPattern = .Pulsar },
                        .Digit5 => { state.selectedPattern = .GosperGun },
                        .Escape => { state.running = false },
                        _ => {}
                    }
                },
                .KeyUp(_) => {},
                .MouseDown(px, py) => {
                    let gx = px / cfg.cellSize;
                    let gy = py / cfg.cellSize;
                    state.grid.stamp(pattern: state.selectedPattern, centerX: gx, centerY: gy);
                }
            }
        }

        let now = sdlGetTicks();
        if not state.paused and (now - simLast) >= simStepMs {
            state.grid.step();
            simLast = now;
        }

        frameCount = frameCount + 1;
        let elapsed = Int64(from: now - fpsLast);
        if elapsed >= 500 {
            state.fps = frameCount * 1000 / elapsed;
            frameCount = 0;
            fpsLast = now;
        }

        let grid = state.grid;
        let patternText = "PATTERN: " + state.selectedPattern.format();
        let fpsText = "FPS: " + state.fps.format();
        app.render { (renderer) in
            renderer.clear(Color.black());
            renderGrid(grid, renderer, cellSize: cellSize);
            renderer.drawText(patternText, 8, 8, 2);
            renderer.drawText(fpsText, 8, 28, 2);
        };
    }

    0
}

func main() -> Int32 {
    let cfg = parseConfig();
    if cfg.headlessIters > 0 {
        return runHeadless(cfg);
    }
    runInteractive(cfg)
}
