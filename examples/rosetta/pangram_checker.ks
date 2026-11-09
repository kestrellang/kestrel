// Rosetta Code: Pangram checker
// https://rosettacode.org/wiki/Pangram_checker
module Main

func isPangram(s: String) -> Bool {
    let letters = Set[Char](from: s.lowercased().chars.iter().filter { it.isAsciiLowercase }.collect());
    letters.count == 26
}

@main
func main() {
    for s in ["The quick brown fox jumps over the lazy dog", "The quick brown fox jumped over the lazy dog"] {
        println("\(s): \(isPangram(s))");
    }
}
