// Rosetta Code: Character codes
// https://rosettacode.org/wiki/Character_codes
module Main

@main
func main() {
    let a = 'a';
    println("'\(a)' has code \(a.value())");
    println("code 97 is '\(Char(97) ?? '?')'");
    let pi = 'π';
    println("'\(pi)' has code \(pi.value()) and is \(pi.utf8Length()) bytes in UTF-8");
}
