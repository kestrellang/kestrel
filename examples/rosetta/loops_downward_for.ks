// Rosetta Code: Loops/Downward for
// https://rosettacode.org/wiki/Loops/Downward_for
module Main

@main
func main() {
    // Ranges only count up, so count up and subtract.
    for i in 0..=10 {
        println(10 - i);
    }
}
