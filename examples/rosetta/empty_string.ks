// Rosetta Code: Empty string
// https://rosettacode.org/wiki/Empty_string
module Main

@main
func main() {
    let empty = "";
    let full = "x";
    println("empty.isEmpty = \(empty.isEmpty)");
    println("full.isEmpty = \(full.isEmpty)");
    println("empty == \"\": \(empty == "")");
    println("String().isEmpty = \(String().isEmpty)");
}
