// Rosetta Code: Horner's rule for polynomial evaluation
// https://rosettacode.org/wiki/Horner%27s_rule_for_polynomial_evaluation
module Main

// Coefficients from the constant term up.
func horner(coefficients: Array[Int64], x: Int64) -> Int64 {
    coefficients.reversed.iter().fold(from: 0, by: { (acc, c) in acc * x + c })
}

@main
func main() {
    // -19 + 7x - 4x^2 + 6x^3 at x = 3
    println(horner([-19, 7, -4, 6], 3));
}
