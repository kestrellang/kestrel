// Rosetta Code: Evaluate binomial coefficients
// https://rosettacode.org/wiki/Evaluate_binomial_coefficients
module Main

// Multiplying before dividing keeps every intermediate value an integer.
func binomial(n: Int64, k: Int64) -> Int64 {
    if k < 0 or k > n { return 0 }
    let kk = if k > n - k { n - k } else { k };
    var result = 1;
    for i in 1..=kk {
        result = result * (n - kk + i) / i;
    }
    result
}

@main
func main() {
    println("C(5, 3) = \(binomial(5, 3))");
    println("C(60, 30) = \(binomial(60, 30))");
    for n in 0..=6 {
        println((0..=n).iter().map { binomial(n, it) }.collect().joined(by: " "));
    }
}
