// Rosetta Code: Tokenize a string
// https://rosettacode.org/wiki/Tokenize_a_string
module Main

@main
func main() {
    let text = "Hello,How,Are,You,Today";
    let words = text.split(",").iter().map { it.toOwned() }.collect();
    println(words.joined("."));
}
