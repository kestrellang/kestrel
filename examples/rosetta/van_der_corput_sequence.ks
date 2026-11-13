// Rosetta Code: Van der Corput sequence
// https://rosettacode.org/wiki/Van_der_Corput_sequence
module Main

// Reflect the base-b digits of n about the radix point.
func vdc(n: Int64, base: Int64) -> Float64 {
    var result = 0.0;
    var denominator = 1.0;
    var rest = n;
    while rest > 0 {
        denominator = denominator * Float64(from: base);
        result = result + Float64(from: rest % base) / denominator;
        rest = rest / base;
    }
    result
}

@main
func main() {
    for base in 2..=5 {
        println("base \(base): \((0..<10).iter().map { "\(vdc(it, base):.4)" }.collect().joined(by: " "))");
    }
}
