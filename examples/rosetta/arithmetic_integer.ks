// Rosetta Code: Arithmetic/Integer
// https://rosettacode.org/wiki/Arithmetic/Integer
module Main

@main
func main() {
    let a: Int64 = -17;
    let b: Int64 = 5;
    println("a + b = \(a + b)");
    println("a - b = \(a - b)");
    println("a * b = \(a * b)");
    // Division truncates toward zero; the remainder takes the sign of a.
    println("a / b = \(a / b)");
    println("a % b = \(a % b)");
    // modulo(by:) is the Euclidean, always non-negative remainder.
    println("a mod b = \(a.modulo(by: b))");
    println("a ^ 3 = \(a.pow(3))");
}
