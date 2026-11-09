// Rosetta Code: Sum of squares
// https://rosettacode.org/wiki/Sum_of_squares
module Main

@main
func main() {
    let xs = [3.0, 1.0, 4.0, 1.0, 5.0, 9.0];
    println(xs.iter().map { it * it }.sum());
    println(Array[Float64]().iter().map { it * it }.sum());
}
