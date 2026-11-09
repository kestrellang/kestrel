// Rosetta Code: Bitwise operations
// https://rosettacode.org/wiki/Bitwise_operations
module Main

@main
func main() {
    let a: Int64 = 255;
    let b: Int64 = 2;
    println("a AND b: \(a & b)");
    println("a OR b:  \(a | b)");
    println("a XOR b: \(a ^ b)");
    println("NOT a:   \(!a)");
    println("a << b:  \(a << b)");
    println("a >> b:  \(a >> b)   (arithmetic: -a >> b = \(-a >> b))");
    let u = UInt64(truncating: -a);
    println("logical right shift of -a by b: \(u >> b)");
    println("a rotl b: \(a.rotateLeft(by: b))");
    println("a rotr b: \(a.rotateRight(by: b))");
}
