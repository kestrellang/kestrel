// Rosetta Code: Gaussian elimination
// https://rosettacode.org/wiki/Gaussian_elimination
module Main

// Solves a x = b with partial pivoting.
func solve(matrix: Array[Array[Float64]], rhs: Array[Float64]) -> Array[Float64] {
    let n = rhs.count;
    var a = matrix;
    var b = rhs;
    for col in 0..<n {
        var pivot = col;
        for r in (col + 1)..<n {
            if a(r)(col).abs() > a(pivot)(col).abs() { pivot = r; }
        }
        a.swap(at: col, with: pivot);
        b.swap(at: col, with: pivot);
        for r in (col + 1)..<n {
            let factor = a(r)(col) / a(col)(col);
            for c in col..<n { a(r)(c) = a(r)(c) - factor * a(col)(c); }
            b(r) = b(r) - factor * b(col);
        }
    }
    var x = Array[Float64](repeating: 0.0, count: n);
    var i = n - 1;
    while i >= 0 {
        var s = b(i);
        for j in (i + 1)..<n { s = s - a(i)(j) * x(j); }
        x(i) = s / a(i)(i);
        i = i - 1;
    }
    x
}

@main
func main() {
    let a = [
        [1.00, 0.00, 0.00, 0.00, 0.00, 0.00],
        [1.00, 0.63, 0.39, 0.25, 0.16, 0.10],
        [1.00, 1.26, 1.58, 1.98, 2.49, 3.13],
        [1.00, 1.88, 3.55, 6.70, 12.62, 23.80],
        [1.00, 2.51, 6.32, 15.88, 39.90, 100.28],
        [1.00, 3.14, 9.87, 31.01, 97.41, 306.02],
    ];
    let b = [-0.01, 0.61, 0.91, 0.99, 0.60, 0.02];
    for x in solve(a, b) {
        println("\(x:.6)");
    }
}
