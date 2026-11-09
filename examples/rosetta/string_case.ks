// Rosetta Code: String case
// https://rosettacode.org/wiki/String_case
module Main

@main
func main() {
    let s = "alphaBETA";
    println(s.uppercased());
    println(s.lowercased());
    println(s.titlecased());
    // Full Unicode case mapping: ß uppercases to SS.
    println("straße".uppercased());
}
