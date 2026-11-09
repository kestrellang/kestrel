// Rosetta Code: Almost prime
// https://rosettacode.org/wiki/Almost_prime
module Main

// Number of prime factors counted with multiplicity.
func bigOmega(n: Int64) -> Int64 {
    var rest = n;
    var count: Int64 = 0;
    var d: Int64 = 2;
    while d * d <= rest {
        while rest % d == 0 {
            rest = rest / d;
            count = count + 1;
        }
        d = d + 1;
    }
    if rest > 1 { count + 1 } else { count }
}

@main
func main() {
    for k in 1..=5 {
        let first = (2..).iter().filter { bigOmega(it) == k }.take(10).collect();
        println("k = \(k): \(first.joined(" "))");
    }
}
