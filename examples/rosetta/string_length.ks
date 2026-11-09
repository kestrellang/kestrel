// Rosetta Code: String length
// https://rosettacode.org/wiki/String_length
module Main

@main
func main() {
    for s in ["møøse", "𝔘𝔫𝔦𝔠𝔬𝔡𝔢", "J̲o̲s̲é̲"] {
        println("\(s): \(s.bytes.count) bytes, \(s.chars.count) chars, \(s.graphemes.count) graphemes");
    }
}
