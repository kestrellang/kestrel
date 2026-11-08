// Rosetta Code: Lucas-Lehmer test
// https://rosettacode.org/wiki/Lucas-Lehmer_test
// 2^p - 1 is prime iff s(p-2) == 0 mod 2^p - 1, where s(0) = 4, s(k+1) = s(k)^2 - 2.
module Main

func mulMod(a: UInt64, b: UInt64, m: UInt64) -> UInt64 {
    var result: UInt64 = 0;
    var x = a % m;
    var y = b;
    while y > 0 {
        if y & 1 == 1 { result = (result + x) % m; }
        x = (x + x) % m;
        y = y >> 1;
    }
    result
}

func isMersennePrime(p: Int64) -> Bool {
    if p == 2 { return true }
    let one: UInt64 = 1;
    let m = (one << p) - 1;
    var s: UInt64 = 4;
    for _ in 0..<(p - 2) {
        s = (mulMod(s, s, m) + m - 2) % m;
    }
    s == 0
}

@main
func main() {
    let exponents = (2..<63).iter().filter { (p) in
        (2..<p).iter().all { p % it != 0 } and isMersennePrime(p)
    }.collect();
    println("Mersenne primes 2^p - 1 for p in: \(exponents.joined(" "))");
}
