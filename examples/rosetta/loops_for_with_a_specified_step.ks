// Rosetta Code: Loops/For with a specified step
// https://rosettacode.org/wiki/Loops/For_with_a_specified_step
module Main

@main
func main() {
    for i in (2..=8).iter().stepBy(2) {
        print("\(i), ");
    }
    println("who do we appreciate?");
}
