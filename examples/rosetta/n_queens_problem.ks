// Rosetta Code: N-queens problem
// https://rosettacode.org/wiki/N-queens_problem
module Main

// Backtracking: queens(r) is the column of the queen on row r.
struct Solver {
    let n: Int64;
    var queens: Array[Int64];
    var solutions: Int64;
    var first: Array[Int64];

    init(size n: Int64) {
        self.n = n;
        self.queens = Array[Int64](repeating: 0, count: n);
        self.solutions = 0;
        self.first = [];
    }

    func safe(row: Int64, col: Int64) -> Bool {
        for r in 0..<row {
            let c = self.queens(r);
            if c == col or (c - col).abs() == row - r { return false }
        }
        true
    }

    mutating func place(row: Int64) {
        if row == self.n {
            if self.solutions == 0 { self.first = self.queens; }
            self.solutions = self.solutions + 1;
            return
        }
        for col in 0..<self.n {
            if self.safe(row, col) {
                self.queens(row) = col;
                self.place(row + 1);
            }
        }
    }
}

@main
func main() {
    var eight = Solver(size: 8);
    eight.place(0);
    println("First solution for 8 queens:");
    for row in 0..<8 {
        let line = (0..<8).iter().map { if it == eight.first(row) { "Q" } else { "." } }.collect();
        println(line.joined(" "));
    }
    for n in 1..=10 {
        var s = Solver(size: n);
        s.place(0);
        println("\(n) queens: \(s.solutions) solutions");
    }
}
