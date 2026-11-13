// Rosetta Code: Generate lower case ASCII alphabet
// https://rosettacode.org/wiki/Generate_lower_case_ASCII_alphabet
module Main

@main
func main() {
    // Char isn't a range bound, so step through the code points.
    let alphabet = ('a'.value()..='z'.value()).iter().map { Char(unchecked: it) }.collect();
    println(alphabet.joined(by: ""));
    println("\(alphabet.count) letters");
}
