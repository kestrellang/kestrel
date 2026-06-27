module Life

struct Grid: Cloneable {
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

    // Reads a cell with toroidal wrap. Every caller passes coordinates at most
    // one step outside the grid (neighbour offsets are -1..1; the self-read and
    // renderer pass in-bounds coordinates), so a single add/subtract wraps each
    // axis — far cheaper than `index`'s general `%` in the per-cell hot loop,
    // where a signed modulo also drags in divide-by-zero / minValue-÷-1 guards.
    func cellAt(x x: Int64, y y: Int64) -> Bool {
        let w = self.width;
        let h = self.height;
        var xx = x; if xx < 0 { xx = xx + w; } else { if xx >= w { xx = xx - w; } }
        var yy = y; if yy < 0 { yy = yy + h; } else { if yy >= h { yy = yy - h; } }
        // Safety: the wrap above leaves `xx` in `[0, w)` and `yy` in `[0, h)`
        // (callers are at most one step out, so a single add/subtract suffices),
        // so `yy * w + xx` is always in `[0, w*h)` — the bounds check would be
        // redundant. `unchecked:` skips it; this is the hot per-cell read.
        self.cells(unchecked: yy * w + xx)
    }

    mutating func setCell(x x: Int64, y y: Int64, alive alive: Bool) {
        let i = self.index(x: x, y: y);
        self.cells(i) = alive;
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
                // B3/S23: birth on exactly 3 live neighbours; an already-live
                // cell survives with 2 or 3.
                let nextAlive = if alive { n == 2 or n == 3 } else { n == 3 };
                // x,y are in-bounds here (loop bounds `0..<width`/`0..<height`),
                // so the index is direct — no wrap — and provably in `[0, w*h)`,
                // making the bounds check redundant. `unchecked:` skips it.
                self.next(unchecked: y * self.width + x) = nextAlive;
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
        for i in 0..<self.cells.count {
            self.cells(i) = rng.nextInt(below: 10) < 3;
        }
    }

    func clone() -> Grid {
        var copy = Grid(width: self.width, height: self.height);
        copy.cells = self.cells.clone();
        copy.next = self.next.clone();
        copy
    }
}
