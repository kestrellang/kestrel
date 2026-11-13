// Rosetta Code: Towers of Hanoi
// https://rosettacode.org/wiki/Towers_of_Hanoi
module Main

func hanoi(n: Int64, from: Int64, to: Int64, via: Int64) {
    if n == 0 { return }
    hanoi(n - 1, from, via, to);
    println("Move disk \(n) from pole \(from) to pole \(to)");
    hanoi(n - 1, via, to, from);
}

@main
func main() {
    hanoi(4, 1, 3, 2);
}
