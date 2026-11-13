// Rosetta Code: Strip whitespace from a string/Top and tail
// https://rosettacode.org/wiki/Strip_whitespace_from_a_string/Top_and_tail
module Main

@main
func main() {
    let s = " \t  padded text \n ";
    println("[\(s.trimmedStart().toOwned())]");
    println("[\(s.trimmedEnd().toOwned())]");
    println("[\(s.trimmed().toOwned())]");
}
