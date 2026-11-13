// Rosetta Code: Strip a set of characters from a string
// https://rosettacode.org/wiki/Strip_a_set_of_characters_from_a_string
module Main

func stripChars(s: String, remove: String) -> String {
    let drop = remove.chars.iter().collect();
    String(from: s.chars.iter().filter { not drop.contains(it) }.collect())
}

@main
func main() {
    println(stripChars("She was a soul stripper. She took my heart!", "aei"));
}
