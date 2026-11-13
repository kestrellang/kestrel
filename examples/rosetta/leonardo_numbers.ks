// Rosetta Code: Leonardo numbers
// https://rosettacode.org/wiki/Leonardo_numbers
module Main

func leonardo(count: Int64, l0: Int64, l1: Int64, add: Int64) -> Array[Int64] {
    var seq = [l0, l1];
    while seq.count < count {
        seq.append(seq(seq.count - 1) + seq(seq.count - 2) + add);
    }
    seq
}

@main
func main() {
    println("Leonardo:  \(leonardo(25, 1, 1, 1).joined(by: " "))");
    println("Fibonacci: \(leonardo(25, 0, 1, 0).joined(by: " "))");
}
