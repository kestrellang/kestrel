// Rosetta Code: Balanced brackets
// https://rosettacode.org/wiki/Balanced_brackets
module Main

func isBalanced(s: String) -> Bool {
    var depth = 0;
    for c in s.chars {
        depth = depth + (if c == '[' { 1 } else { -1 });
        if depth < 0 { return false }
    }
    depth == 0
}

func report(s: String) {
    println("\(s:<20) \(if isBalanced(s) { "OK" } else { "NOT OK" })");
}

@main
func main() {
    for s in ["[]", "][", "[][]", "][][", "[[][]]", "[]][[]"] { report(s); }
    // A fixed seed keeps the output reproducible.
    var rng = Lcg64(seed: 2024);
    for n in 0..<10 {
        var brackets = Array[Char]();
        for _ in 0..<n {
            brackets.append('[');
            brackets.append(']');
        }
        brackets.shuffle(using: rng);
        report(brackets.joined(""));
    }
}
