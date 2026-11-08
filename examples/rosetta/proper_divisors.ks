// Rosetta Code: Proper divisors
// https://rosettacode.org/wiki/Proper_divisors
module Main

func properDivisors(n: Int64) -> Array[Int64] {
    (1..=(n / 2)).iter().filter { n % it == 0 }.collect()
}

@main
func main() {
    for n in 1..=10 {
        println("\(n:>2): \(properDivisors(n))");
    }
    var best: Int64 = 1;
    var bestCount: Int64 = 0;
    for n in 1..=20000 {
        let c = properDivisors(n).count;
        if c > bestCount {
            best = n;
            bestCount = c;
        }
    }
    println("\(best) has the most proper divisors below 20000: \(bestCount)");
}
