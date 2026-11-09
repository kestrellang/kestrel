// Rosetta Code: Even or odd
// https://rosettacode.org/wiki/Even_or_odd
module Main

@main
func main() {
    for n in [-3, -2, 0, 7, 10] {
        let parity = if n % 2 == 0 { "even" } else { "odd" };
        // Bitwise test works for negatives too in two's complement.
        let bit = if n & 1 == 0 { "even" } else { "odd" };
        println("\(n): \(parity) (\(bit) by bit test)");
    }
}
