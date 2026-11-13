// Rosetta Code: Sum of a series
// https://rosettacode.org/wiki/Sum_of_a_series
module Main

@main
func main() {
    // sum of 1/k^2 for k = 1..1000, which approaches pi^2 / 6.
    let s = (1..=1000).iter().map { 1.0 / Float64(from: it * it) }.sum();
    println(s);
    println("pi^2 / 6 = \(Float64.pi * Float64.pi / 6.0)");
}
