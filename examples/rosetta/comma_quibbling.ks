// Rosetta Code: Comma quibbling
// https://rosettacode.org/wiki/Comma_quibbling
module Main

func quibble(words: Array[String]) -> String {
    let inner = match words.count {
        0 => "",
        1 => words(0),
        _ => "\(words.drop(last: 1).toArray().joined(", ")) and \(words(words.count - 1))",
    };
    "{\(inner)}"
}

@main
func main() {
    println(quibble([]));
    println(quibble(["ABC"]));
    println(quibble(["ABC", "DEF"]));
    println(quibble(["ABC", "DEF", "G", "H"]));
}
