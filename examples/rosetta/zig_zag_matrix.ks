// Rosetta Code: Zig-zag matrix
// https://rosettacode.org/wiki/Zig-zag_matrix
module Main

func zigzag(n: Int64) -> Array[Array[Int64]] {
    var m = Array(of: n, generatedBy: { (_) in Array[Int64](repeating: 0, count: n) });
    var (row, col) = (0, 0);
    for k in 0..<(n * n) {
        m(row)(col) = k;
        // Even diagonals run up-right, odd ones down-left.
        if (row + col) % 2 == 0 {
            if col == n - 1 { row = row + 1; }
            else if row == 0 { col = col + 1; }
            else { row = row - 1; col = col + 1; }
        } else {
            if row == n - 1 { col = col + 1; }
            else if col == 0 { row = row + 1; }
            else { row = row + 1; col = col - 1; }
        }
    }
    m
}

@main
func main() {
    for row in zigzag(5) {
        println(row.map { "\(it:>3)" }.joined(by: ""));
    }
}
