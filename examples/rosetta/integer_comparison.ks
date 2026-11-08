// Rosetta Code: Integer comparison
// https://rosettacode.org/wiki/Integer_comparison
module Main

func compare(a: Int64, b: Int64) {
    if a < b { println("\(a) is less than \(b)"); }
    if a == b { println("\(a) is equal to \(b)"); }
    if a > b { println("\(a) is greater than \(b)"); }
    let word: String = match a.compare(b) {
        .Less => "Less",
        .Equal => "Equal",
        .Greater => "Greater",
    };
    println("  compare: \(word)");
}

@main
func main() {
    compare(1, 2);
    compare(5, 5);
    compare(9, -9);
}
