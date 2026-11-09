// Rosetta Code: Identity matrix
// https://rosettacode.org/wiki/Identity_matrix
module Main

func identity(n: Int64) -> Array[Array[Int64]] {
    Array(of: n, generatedBy: { (i) in
        Array(of: n, generatedBy: { (j) in if i == j { 1 } else { 0 } })
    })
}

@main
func main() {
    for row in identity(5) {
        println(row.joined(" "));
    }
}
