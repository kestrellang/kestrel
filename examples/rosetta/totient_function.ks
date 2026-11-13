// Rosetta Code: Totient function
// https://rosettacode.org/wiki/Totient_function
module Main

func totient(n: Int64) -> Int64 {
    var result = n;
    var rest = n;
    var p = 2;
    while p * p <= rest {
        if rest % p == 0 {
            while rest % p == 0 { rest = rest / p; }
            result = result - result / p;
        }
        p = p + 1;
    }
    if rest > 1 { result = result - result / rest; }
    result
}

@main
func main() {
    println(" n  phi  prime");
    for n in 1..=25 {
        let phi = totient(n);
        println("\(n:>2) \(phi:>4)  \(phi == n - 1)");
    }
    for limit in [100, 1000, 10000, 100000] {
        let primes = (2..=limit).iter().filter { totient(it) == it - 1 }.count();
        println("primes up to \(limit): \(primes)");
    }
}
