// Rosetta Code: Loops/For
// https://rosettacode.org/wiki/Loops/For
module Main

@main
func main() {
    for i in 1..=5 {
        var line = StringBuilder();
        for _ in 0..<i { line.append("*"); }
        println(line.build());
    }
}
