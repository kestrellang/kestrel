// Rosetta Code: Greatest common divisor
// https://rosettacode.org/wiki/Greatest_common_divisor
module Main

// Euclid's algorithm.
func gcd(a: Int64, b: Int64) -> Int64 {
    var x = a.abs();
    var y = b.abs();
    while y != 0 {
        let r = x % y;
        x = y;
        y = r;
    }
    x
}

@main
func main() {
    println("gcd(1071, 1029) = \(gcd(1071, 1029))");
    println("gcd(40902, 24140) = \(gcd(40902, 24140))");
    println("gcd(-36, 48) = \(gcd(-36, 48))");
    // The standard library has one too.
    println("1071.gcd(1029) = \(1071.gcd(1029))");
}
