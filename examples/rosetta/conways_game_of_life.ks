// Rosetta Code: Conway's Game of Life
// https://rosettacode.org/wiki/Conway%27s_Game_of_Life
module Main

struct Life {
    let width: Int64;
    let height: Int64;
    var cells: Array[Bool];

    func alive(x: Int64, y: Int64) -> Bool {
        // The board wraps around at the edges.
        let wx = (x + self.width) % self.width;
        let wy = (y + self.height) % self.height;
        self.cells(wy * self.width + wx)
    }

    func neighbours(x: Int64, y: Int64) -> Int64 {
        var count = 0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if (dx != 0 or dy != 0) and self.alive(x + dx, y + dy) { count = count + 1; }
            }
        }
        count
    }

    mutating func step() {
        var next = self.cells;
        for y in 0..<self.height {
            for x in 0..<self.width {
                let n = self.neighbours(x, y);
                next(y * self.width + x) = n == 3 or (n == 2 and self.alive(x, y));
            }
        }
        self.cells = next;
    }

    func show() {
        for y in 0..<self.height {
            let row = (0..<self.width).iter().map { if self.alive(it, y) { "#" } else { "." } }.collect();
            println(row.joined(by: ""));
        }
    }
}

@main
func main() {
    // A blinker and a glider on an 8x8 board.
    var life = Life(width: 8, height: 8, cells: Array[Bool](repeating: false, count: 64));
    for (x, y) in [(1, 0), (1, 1), (1, 2), (5, 4), (6, 5), (4, 6), (5, 6), (6, 6)] {
        life.cells(y * 8 + x) = true;
    }
    for generation in 0..<4 {
        println("Generation \(generation):");
        life.show();
        life.step();
    }
}
