// Conway's Game of Life — an SDL example.
//
// The world is a toroidal 80x60 grid (cells wrap at the edges) drawn with
// 10px squares into the 800x600 window provided by `Sdl.SDLApp`. Each tick
// applies the standard B3/S23 rules to every cell using a double-buffered
// step so reads always see the previous generation.
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

struct Config {
    static var width: Int64 { 80 }
    static var height: Int64 { 60 }
    static var cellSize: Int64 { 10 }
    static var stepDelayMs: Int64 { 80 }
}

struct Grid {
    var cells: Array[Bool]
    var next: Array[Bool]

    init() {
        let n = Config.width * Config.height;
        self.cells = Array[Bool](repeating: false, count: n);
        self.next = Array[Bool](repeating: false, count: n);
    }

    // Toroidal wrap so a glider that walks off one edge re-enters from the
    // opposite side. The double-modulo handles negative inputs from the
    // neighbour scan.
    func index(x x: Int64, y y: Int64) -> Int64 {
        let w = Config.width;
        let h = Config.height;
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
        while y < Config.height {
            var x: Int64 = 0;
            while x < Config.width {
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
        let cell = Config.cellSize;
        var y: Int64 = 0;
        while y < Config.height {
            var x: Int64 = 0;
            while x < Config.width {
                if self.cellAt(x: x, y: y) {
                    // Subtract 1 from the rect size so a thin black gutter
                    // separates neighbouring cells visually.
                    let rect = Rectangle(
                        x: x * cell,
                        y: y * cell,
                        width: cell - 1,
                        height: cell - 1
                    );
                    renderer.fill(rect, Color.green());
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

func main() -> Int32 {
    var app = SDLApp(title: "Game of Life");
    var grid = Grid();
    var paused = false;
    var running = true;
    // 0..4 — see `patternName` for the labels.
    var selectedPattern: Int64 = 0;
    // Bumped each time we reseed so successive R-presses give a different
    // pattern instead of replaying the same starting field.
    var seedCounter: UInt64 = UInt64(intLiteral: 12648430);

    grid.randomize(seed: seedCounter);

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
                    let gx = px / Config.cellSize;
                    let gy = py / Config.cellSize;
                    grid.stamp(kind: selectedPattern, centerX: gx, centerY: gy);
                }
            }
        }

        if not paused {
            grid.step();
        }

        app.render { (renderer) in
            renderer.clear(Color.black());
            grid.render(renderer);
            // HUD: current pattern selection in the top-left, scale 2.
            renderer.drawText("PATTERN: " + patternName(kind: selectedPattern), 8, 8, 2);
        };

        app.delay(Milliseconds(Config.stepDelayMs));
    }

    0
}
