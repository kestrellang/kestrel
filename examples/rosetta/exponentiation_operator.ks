// Rosetta Code: Exponentiation operator
// https://rosettacode.org/wiki/Exponentiation_operator
module Main

// Exponentiation by squaring, for integer and floating-point bases.
// (Overloads must differ in labels, not just parameter types.)
func power(base: Int64, exponent: Int64) -> Int64 {
    var result: Int64 = 1;
    var b = base;
    var e = exponent;
    while e > 0 {
        if e & 1 == 1 { result = result * b; }
        b = b * b;
        e = e >> 1;
    }
    result
}

func powerReal(base: Float64, exponent: Int64) -> Float64 {
    if exponent < 0 { return 1.0 / powerReal(base, -exponent) }
    var result = 1.0;
    var b = base;
    var e = exponent;
    while e > 0 {
        if e & 1 == 1 { result = result * b; }
        b = b * b;
        e = e >> 1;
    }
    result
}

@main
func main() {
    println("2 ^ 10 = \(power(2, 10))");
    println("-3 ^ 3 = \(power(-3, 3))");
    println("1.5 ^ 3 = \(powerReal(1.5, 3))");
    println("2.0 ^ -2 = \(powerReal(2.0, -2))");
    println("std: 2.pow(10) = \(2.pow(10)), 1.5.powi(3) = \(1.5.powi(3))");
}
