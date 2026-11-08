// Rosetta Code: Modular inverse
// https://rosettacode.org/wiki/Modular_inverse
module Main

// Extended Euclid; null when a and m aren't coprime.
func modInverse(a: Int64, m: Int64) -> Int64? {
    var (oldR, r) = (a, m);
    var (oldS, s) = (1, 0);
    while r != 0 {
        let q = oldR / r;
        let nextR = oldR - q * r;
        oldR = r;
        r = nextR;
        let nextS = oldS - q * s;
        oldS = s;
        s = nextS;
    }
    if oldR != 1 { return null }
    oldS.modulo(by: m)
}

@main
func main() {
    println("42^-1 mod 2017 = \(modInverse(42, 2017) ?? -1)");
    println("40^-1 mod 1 = \(modInverse(40, 1) ?? -1)");
    match modInverse(52, 26) {
        some x => println("52^-1 mod 26 = \(x)"),
        null => println("52 has no inverse mod 26"),
    }
}
