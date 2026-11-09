// Rosetta Code: Loops/Do-while
// https://rosettacode.org/wiki/Loops/Do-while
module Main

@main
func main() {
    var value = 0;
    loop {
        value = value + 1;
        println(value);
        if value % 6 == 0 { break }
    }
}
