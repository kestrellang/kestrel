// Self-contained Game of Life solver benchmark (no SDL, no clutch).
// Mirrors examples/life/src/grid.ks exactly so the comparison against
// life_bench.c measures codegen, not a different algorithm.

module LifeBench

@extern(.C, mangleName: "bench_now_ns")
func benchNowNs() -> Int64

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

    func cellAt(x x: Int64, y y: Int64) -> Bool {
        let w = self.width;
        let h = self.height;
        var xx = x; if xx < 0 { xx = xx + w; } else { if xx >= w { xx = xx - w; } }
        var yy = y; if yy < 0 { yy = yy + h; } else { if yy >= h { yy = yy - h; } }
        self.cells(unchecked: yy * w + xx)
    }

    func neighborCount(x x: Int64, y y: Int64) -> Int64 {
        var count: Int64 = 0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if not (dx == 0 and dy == 0) {
                    if self.cellAt(x: x + dx, y: y + dy) { count = count + 1; }
                }
            }
        }
        count
    }

    mutating func step() {
        for y in 0..<self.height {
            for x in 0..<self.width {
                let alive = self.cellAt(x: x, y: y);
                let n = self.neighborCount(x: x, y: y);
                let nextAlive = if alive { n == 2 or n == 3 } else { n == 3 };
                self.next(unchecked: y * self.width + x) = nextAlive;
            }
        }
        let tmp = self.cells;
        self.cells = self.next;
        self.next = tmp;
    }

    mutating func randomize(seed seed: UInt64) {
        var rng = Lcg64(seed: seed);
        for i in 0..<self.cells.count {
            self.cells(i) = rng.nextInt(below: 10) < 3;
        }
    }
}

@main
func main() {
    let W: Int64 = 512;
    let H: Int64 = 512;
    let ITERS: Int64 = 2000;

    var grid = Grid(width: W, height: H);
    grid.randomize(seed: 12648430);

    let t0 = benchNowNs();
    for _ in 0..<ITERS {
        grid.step();
    }
    let t1 = benchNowNs();

    var live: Int64 = 0;
    for i in 0..<grid.cells.count {
        if grid.cells(i) { live = live + 1; }
    }

    let ns = t1 - t0;
    let ms = ns / 1000000;
    let gps = if ms > 0 { ITERS * 1000 / ms } else { 0 };
    println("kestrel W=\(W) H=\(H) iters=\(ITERS) ms=\(ms) gens_per_sec=\(gps) live=\(live)");
}
