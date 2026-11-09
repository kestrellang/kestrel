// Rosetta Code: Prime decomposition
// https://rosettacode.org/wiki/Prime_decomposition
module Main

func primeFactors(n: Int64) -> Array[Int64] {
    var factors = Array[Int64]();
    var rest = n;
    var d: Int64 = 2;
    while d * d <= rest {
        while rest % d == 0 {
            factors.append(d);
            rest = rest / d;
        }
        d = d + 1;
    }
    if rest > 1 { factors.append(rest); }
    factors
}

@main
func main() {
    for n in [12, 360, 97, 1001, 2147483647, 600851475143, 9007199254740993] {
        println("\(n) = \(primeFactors(n).joined(" x "))");
    }
}
