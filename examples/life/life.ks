// Conway's Game of Life — an SDL example.
//
// The world is a toroidal 80x60 grid (cells wrap at the edges) drawn with
// 10px squares into the 800x600 window provided by `Sdl.SDLApp`. Each tick
// applies the standard B3/S23 rules to every cell using a double-buffered
// step so reads always see the previous generation.
//
// Controls:
//   Space  pause / resume
//   R      reseed with random noise
//   C      clear the grid (auto-pauses so you can plant patterns)
//   G      drop a glider near the top-left corner
//   Esc    quit

module Life

import Sdl.(Color, Rectangle, Milliseconds, Key, Event, Renderer, SDLApp)

struct Config {
    static var width: Int64 { 80 }
    static var height: Int64 { 60 }
    static var cellSize: Int64 { 10 }
    static var stepDelayMs: Int64 { 80 }
}

struct Grid : not Copyable {
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
    // doesn't immediately collapse to overpopulation.
    mutating func randomize(mutating using rng: Lcg64) {
        var i: Int64 = 0;
        while i < self.cells.count {
            self.cells(i) = rng.nextInt(below: 10) < 3;
            i = i + 1;
        }
    }

    // Classic 5-cell glider, travels south-east one step per 4 generations.
    mutating func seedGlider(x x: Int64, y y: Int64) {
        self.setCell(x: x + 1, y: y + 0, alive: true);
        self.setCell(x: x + 2, y: y + 1, alive: true);
        self.setCell(x: x + 0, y: y + 2, alive: true);
        self.setCell(x: x + 1, y: y + 2, alive: true);
        self.setCell(x: x + 2, y: y + 2, alive: true);
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

func main() -> Int32 {
    var app = SDLApp();
    var grid = Grid();
    var rng = Lcg64(seed: UInt64(intLiteral: 12648430));
    var paused = false;
    var running = true;

    grid.randomize(using: rng);

    while running {
        while let .Some(event) = app.pollEvent() {
            match event {
                .Quit => { running = false },
                .KeyDown(key) => {
                    match key {
                        .Space => { paused = not paused },
                        .R => {
                            grid.randomize(using: rng);
                            paused = false;
                        },
                        .C => {
                            grid.clear();
                            paused = true;
                        },
                        .G => { grid.seedGlider(x: 5, y: 5) },
                        .Escape => { running = false },
                        _ => {}
                    }
                },
                .KeyUp(_) => {}
            }
        }

        if not paused {
            grid.step();
        }

        app.render { (renderer) in
            renderer.clear(Color.black());
            grid.render(renderer);
        };

        app.delay(Milliseconds(Config.stepDelayMs));
    }

    0
}
