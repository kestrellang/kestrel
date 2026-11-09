// Rosetta Code: Thue-Morse
// https://rosettacode.org/wiki/Thue-Morse
module Main

@main
func main() {
    // Bit n is the parity of the number of 1 bits in n.
    let bits = (0..<64).iter().map { UInt64(truncating: it).countOnes % 2 }.collect();
    println(bits.joined(by: ""));
    // Or by repeatedly appending the complement.
    var s = "0";
    for _ in 0..<6 {
        s = s + String(from: s.chars.iter().map { if it == '0' { '1' } else { '0' } }.collect());
    }
    println(s);
}
