// Rosetta Code: Sum multiples of 3 and 5
// https://rosettacode.org/wiki/Sum_multiples_of_3_and_5
module Main

// Brute force.
func sumMultiples(below n: Int64) -> Int64 {
    (1..<n).iter().filter { it % 3 == 0 or it % 5 == 0 }.sum()
}

// Closed form: inclusion-exclusion over arithmetic series.
func sumOfMultiples(of k: Int64, below n: Int64) -> Int64 {
    let m = (n - 1) / k;
    k * m * (m + 1) / 2
}

func sumFast(below n: Int64) -> Int64 {
    sumOfMultiples(of: 3, below: n) + sumOfMultiples(of: 5, below: n) - sumOfMultiples(of: 15, below: n)
}

@main
func main() {
    println("Below 1000: \(sumMultiples(below: 1000))");
    println("Below 1000 (closed form): \(sumFast(below: 1000))");
    println("Below 10^9 (closed form): \(sumFast(below: 1000000000))");
}
