// Rosetta Code: Matrix transposition
// https://rosettacode.org/wiki/Matrix_transposition
module Main

func transpose(m: Array[Array[Float64]]) -> Array[Array[Float64]] {
    if m.isEmpty { return [] }
    Array(of: m(0).count, generatedBy: { (j) in m.map { it(j) } })
}

@main
func main() {
    let m = [[1.0, 1.0, 1.0, 1.0], [2.0, 4.0, 8.0, 16.0], [3.0, 9.0, 27.0, 81.0],
             [4.0, 16.0, 64.0, 256.0], [5.0, 25.0, 125.0, 625.0]];
    for row in transpose(m) {
        println(row.map { "\(it:>5)" }.joined(by: " "));
    }
}
