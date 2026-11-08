// Rosetta Code: Arrays
// https://rosettacode.org/wiki/Arrays
module Main

@main
func main() {
    // Fixed contents from a literal.
    var a = [1, 2, 3];
    a(0) = 10;
    println("a = \(a), a(1) = \(a(1)), count = \(a.count)");
    // Growable.
    var b = Array[String]();
    b.append("x");
    b.append("y");
    b.insert("w", at: 0);
    println("b = \(b)");
    // Pre-sized.
    let zeros = Array[Float64](repeating: 0.0, count: 4);
    println("zeros = \(zeros)");
    // Out-of-range access with an Optional instead of a trap.
    println("a(checked: 7) = \(a(checked: 7) ?? -1)");
    // Copies are independent values.
    var copy = a;
    copy.append(4);
    println("a = \(a), copy = \(copy)");
}
