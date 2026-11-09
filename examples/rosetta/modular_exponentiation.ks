// Rosetta Code: Modular exponentiation (with 64-bit operands)
// https://rosettacode.org/wiki/Modular_exponentiation
module Main

// (a * b) mod m without overflow, by doubling.
func mulMod(a: UInt64, b: UInt64, m: UInt64) -> UInt64 {
    var result: UInt64 = 0;
    var x = a % m;
    var y = b;
    while y > 0 {
        if y & 1 == 1 { result = (result + x) % m; }
        x = (x * 2) % m;
        y = y >> 1;
    }
    result
}

func powMod(base: UInt64, exponent: UInt64, modulus: UInt64) -> UInt64 {
    var result: UInt64 = 1;
    var b = base % modulus;
    var e = exponent;
    while e > 0 {
        if e & 1 == 1 { result = mulMod(result, b, modulus); }
        b = mulMod(b, b, modulus);
        e = e >> 1;
    }
    result
}

@main
func main() {
    println("4^13 mod 497 = \(powMod(4, 13, 497))");
    println("2^(2^62) mod 1000000007 = \(powMod(2, 4611686018427387904, 1000000007))");
    println("Fermat: 7^(1000000006) mod 1000000007 = \(powMod(7, 1000000006, 1000000007))");
}
