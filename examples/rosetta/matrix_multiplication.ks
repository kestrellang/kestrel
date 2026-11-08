// Rosetta Code: Matrix multiplication
// https://rosettacode.org/wiki/Matrix_multiplication
module Main

func multiply(a: Array[Array[Int64]], b: Array[Array[Int64]]) -> Array[Array[Int64]] {
    let rows = a.count;
    let cols = b(0).count;
    let inner = b.count;
    Array(of: rows, generatedBy: { (i) in
        Array(of: cols, generatedBy: { (j) in
            (0..<inner).iter().map { a(i)(it) * b(it)(j) }.sum()
        })
    })
}

@main
func main() {
    let a = [[1, 2], [3, 4], [5, 6]];
    let b = [[7, 8, 9], [10, 11, 12]];
    for row in multiply(a, b) {
        println(row);
    }
}
