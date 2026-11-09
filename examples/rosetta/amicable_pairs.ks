// Rosetta Code: Amicable pairs
// https://rosettacode.org/wiki/Amicable_pairs
module Main

@main
func main() {
    let limit = 20000;
    // Sieve the sums of proper divisors.
    var sums = Array[Int64](repeating: 0, count: limit + 1);
    for d in 1..=(limit / 2) {
        var m = 2 * d;
        while m <= limit {
            sums(m) = sums(m) + d;
            m = m + d;
        }
    }
    for n in 1..=limit {
        let m = sums(n);
        if m > n and m <= limit and sums(m) == n {
            println("\(n) \(m)");
        }
    }
}
