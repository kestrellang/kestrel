// Rosetta Code: Primality by trial division
// https://rosettacode.org/wiki/Primality_by_trial_division
module Main

func isPrime(n: Int64) -> Bool {
    if n < 2 { return false }
    if n % 2 == 0 { return n == 2 }
    var d: Int64 = 3;
    while d * d <= n {
        if n % d == 0 { return false }
        d = d + 2;
    }
    true
}

@main
func main() {
    println((0..<60).iter().filter { isPrime(it) }.collect());
    println("isPrime(2147483647) = \(isPrime(2147483647))");
    println("isPrime(3000000021) = \(isPrime(3000000021))");
}
