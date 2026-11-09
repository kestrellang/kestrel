// Rosetta Code: Spiral matrix
// https://rosettacode.org/wiki/Spiral_matrix
module Main

func spiral(n: Int64) -> Array[Array[Int64]] {
    var m = Array(of: n, generatedBy: { (_) in Array[Int64](repeating: -1, count: n) });
    let dr = [0, 1, 0, -1];
    let dc = [1, 0, -1, 0];
    var (r, c, d) = (0, 0, 0);
    for k in 0..<(n * n) {
        m(r)(c) = k;
        let nr = r + dr(d);
        let nc = c + dc(d);
        // Turn right when the next cell is off the grid or already filled.
        if nr < 0 or nr >= n or nc < 0 or nc >= n or m(nr)(nc) != -1 {
            d = (d + 1) % 4;
        }
        r = r + dr(d);
        c = c + dc(d);
    }
    m
}

@main
func main() {
    for row in spiral(5) {
        println(row.map { "\(it:>3)" }.joined(by: ""));
    }
}
