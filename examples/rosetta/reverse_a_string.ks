// Rosetta Code: Reverse a string
// https://rosettacode.org/wiki/Reverse_a_string
module Main

@main
func main() {
    let s = "asdf";
    println(String(from: s.chars.reversed));
    // Reversing by grapheme keeps combining marks attached to their letters.
    let accented = "as\u{301}df\u{311}";
    var graphemes = accented.graphemes.iter().collect();
    graphemes.reverse();
    println(graphemes.joined(""));
}
