module Life

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
        for i in 0..<self.cells.count {
            self.cells(i) = rng.nextInt(below: 10) < 3;
        }
    }
}
