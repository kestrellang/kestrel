// Rosetta Code: Integer overflow
// https://rosettacode.org/wiki/Integer_overflow
// Plain arithmetic traps on overflow in debug builds; the explicit
// wrapping, checked and saturating forms say what you want instead.
module Main

@main
func main() {
    let big: Int64 = 9223372036854775807;
    println("max + 1 wrapping:   \(big.addWrapping(1))");
    println("max + 1 checked:    \(big.addChecked(1) == .None)");
    println("max + 1 saturating: \(big.addSaturating(1))");
    let small: Int64 = -9223372036854775807 - 1;
    println("-min checked:       \(small.negateChecked() == .None)");
    println("min / -1 checked:   \(small.divideChecked(-1) == .None)");
    println("4000000000 * 3000000000 checked: \(4000000000.multiplyChecked(3000000000) == .None)");
    let u: UInt32 = 4294967295;
    println("UInt32 max + 1 wrapping: \(u.addWrapping(1))");
}
