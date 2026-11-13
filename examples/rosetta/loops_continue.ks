// Rosetta Code: Loops/Continue
// https://rosettacode.org/wiki/Loops/Continue
module Main

@main
func main() {
    for i in 1..=10 {
        print(i);
        if i % 5 == 0 {
            println();
            continue
        }
        print(", ");
    }
}
