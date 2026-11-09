// Rosetta Code: Nth root
// https://rosettacode.org/wiki/Nth_root
module Main

// Newton's method on f(x) = x^n - a.
func nthRoot(a: Float64, n: Int64, precision: Float64) -> Float64 {
    let nf = Float64(from: n);
    var x = a / nf;
    var delta = precision + 1.0;
    while delta >= precision {
        let next = ((nf - 1.0) * x + a / x.powi(n - 1)) / nf;
        delta = (next - x).abs();
        x = next;
    }
    x
}

@main
func main() {
    println(nthRoot(34.0, 5, 1.0e-12));
    println(nthRoot(2.0, 2, 1.0e-12));
    println(nthRoot(1024.0, 10, 1.0e-12));
    println("check: \(34.0.pow(1.0 / 5.0))");
}
