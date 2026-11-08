// Rosetta Code: Sudoku
// https://rosettacode.org/wiki/Sudoku
module Main

func canPlace(grid: Array[Int64], cell: Int64, digit: Int64) -> Bool {
    let row = cell / 9;
    let col = cell % 9;
    let boxRow = row / 3 * 3;
    let boxCol = col / 3 * 3;
    for k in 0..<9 {
        if grid(row * 9 + k) == digit { return false }
        if grid(k * 9 + col) == digit { return false }
        if grid((boxRow + k / 3) * 9 + boxCol + k % 3) == digit { return false }
    }
    true
}

// Backtracking over empty cells in order.
func solve(mutating grid: Array[Int64]) -> Bool {
    let empty = grid.firstIndex(of: 0);
    guard let .Some(cell) = empty else { return true }
    for digit in 1..=9 {
        if canPlace(grid, cell, digit) {
            grid(cell) = digit;
            if solve(grid) { return true }
            grid(cell) = 0;
        }
    }
    false
}

func show(grid: Array[Int64]) {
    for row in 0..<9 {
        if row % 3 == 0 and row > 0 { println("------+-------+------"); }
        var line = StringBuilder();
        for col in 0..<9 {
            if col % 3 == 0 and col > 0 { line.append("| "); }
            line.append("\(grid(row * 9 + col)) ");
        }
        println(line.build().trimmedEnd().toOwned());
    }
}

@main
func main() {
    let puzzle = "850002400720000009004000000000107002305000900040000000000080070017000000000036040";
    var grid = puzzle.chars.iter().map { Int64(from: it.digitValue() ?? 0) }.collect();
    if solve(grid) { show(grid); } else { println("no solution"); }
}
