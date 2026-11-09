// Rosetta Code: Determinant and permanent
// https://rosettacode.org/wiki/Determinant_and_permanent
module Main

// Laplace expansion along the first row; `sign` is -1 for the determinant
// and +1 for the permanent.
func expand(m: Array[Array[Int64]], sign: Int64) -> Int64 {
    let n = m.count;
    if n == 1 { return m(0)(0) }
    var total: Int64 = 0;
    var s: Int64 = 1;
    for col in 0..<n {
        let minor = m.drop(first: 1).map { (row) in
            row.iter().enumerate().filter(where: { it.0 != col }).map(as: { it.1 }).collect()
        };
        total = total + s * m(0)(col) * expand(minor, sign);
        s = s * sign;
    }
    total
}

@main
func main() {
    let matrices = [
        [[1, 2], [3, 4]],
        [[-2, 2, -3], [-1, 1, 3], [2, 0, -1]],
        [[1, 2, 3, 4], [4, 5, 6, 7], [7, 8, 9, 10], [10, 11, 12, 13]],
        [[0, 1, 2, 3, 4], [5, 6, 7, 8, 9], [10, 11, 12, 13, 14], [15, 16, 17, 18, 19], [20, 21, 22, 23, 24]],
    ];
    for m in matrices {
        println("\(m)");
        println("  determinant: \(expand(m, -1)), permanent: \(expand(m, 1))");
    }
}
