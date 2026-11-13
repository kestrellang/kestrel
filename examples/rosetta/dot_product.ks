// Rosetta Code: Dot product
// https://rosettacode.org/wiki/Dot_product
module Main

func dot(a: Array[Int64], b: Array[Int64]) -> Int64 {
    a.iter().zip(b.iter()).map { it.0 * it.1 }.sum()
}

@main
func main() {
    println(dot([1, 3, -5], [4, -2, -1]));
}
