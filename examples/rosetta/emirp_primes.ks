// Rosetta Code: Emirp primes
// https://rosettacode.org/wiki/Emirp_primes
module Main

func isPrime(n: Int64) -> Bool {
    if n < 2 { return false }
    if n % 2 == 0 { return n == 2 }
    var d = 3;
    while d * d <= n {
        if n % d == 0 { return false }
        d = d + 2;
    }
    true
}

func reversed(n: Int64) -> Int64 {
    var rest = n;
    var r = 0;
    while rest > 0 {
        r = r * 10 + rest % 10;
        rest = rest / 10;
    }
    r
}

// A prime whose reversal is a different prime.
func isEmirp(n: Int64) -> Bool {
    let r = reversed(n);
    r != n and isPrime(n) and isPrime(r)
}

@main
func main() {
    println("First 20: \((1..).iter().filter { isEmirp(it) }.take(20).collect())");
    println("Between 7700 and 8000: \((7700..=8000).iter().filter { isEmirp(it) }.collect())");
    println("10000th: \((1..).iter().filter { isEmirp(it) }.nth(9999) ?? 0)");
}
