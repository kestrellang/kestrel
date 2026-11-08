// Rosetta Code: Loops/N plus one half
// https://rosettacode.org/wiki/Loops/N_plus_one_half
module Main

@main
func main() {
    for i in 1..=10 {
        print(i);
        if i == 10 { break }
        print(", ");
    }
    println();
}
