// Rosetta Code: Magic squares of odd order
// https://rosettacode.org/wiki/Magic_squares_of_odd_order
module Main

// The Siamese method, as a closed formula for each cell.
func magicSquare(n: Int64) -> Array[Array[Int64]] {
    Array(of: n, generatedBy: { (i) in
        Array(of: n, generatedBy: { (j) in
            n * ((i + j + 1 + n / 2) % n) + ((i + 2 * j + 2 * n - 2) % n) + 1
        })
    })
}

@main
func main() {
    let n = 5;
    let square = magicSquare(n);
    for row in square {
        println(row.map { "\(it:>3)" }.joined(""));
    }
    println("Magic constant: \(square(0).iter().sum())");
    // Check every row, column and both diagonals.
    let target = n * (n * n + 1) / 2;
    var ok = (0..<n).iter().all { square(it).iter().sum() == target };
    ok = ok and (0..<n).iter().all { (j) in square.iter().map { it(j) }.sum() == target };
    ok = ok and (0..<n).iter().map { square(it)(it) }.sum() == target;
    ok = ok and (0..<n).iter().map { square(it)(n - 1 - it) }.sum() == target;
    println("Valid: \(ok)");
}
