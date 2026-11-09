// Rosetta Code: Copy a string
// https://rosettacode.org/wiki/Copy_a_string
module Main

@main
func main() {
    let original = "Hello";
    // Strings are values: assignment copies (the buffer is shared until a write).
    var copy = original;
    copy.append(", world");
    println("original: \(original)");
    println("copy:     \(copy)");
}
