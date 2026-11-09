// Rosetta Code: Fibonacci n-step number sequences
// https://rosettacode.org/wiki/Fibonacci_n-step_number_sequences
module Main

func nStep(start: Array[Int64], count: Int64) -> Array[Int64] {
    let n = start.count;
    var seq = start;
    while seq.count < count {
        seq.append(seq.suffix(n).iter().sum());
    }
    seq
}

@main
func main() {
    let names = ["fibonacci", "tribonacci", "tetranacci", "pentanacci", "hexanacci",
                 "heptanacci", "octonacci", "nonanacci", "decanacci"];
    // Each n-step sequence starts with 1, 1, 2, 4, ... 2^(n-2).
    for n in 2..=10 {
        var start: Array[Int64] = [1];
        for k in 0..<(n - 1) { start.append(1 << k); }
        println("\(names(n - 2):>10): \(nStep(start, 10).joined(" "))");
    }
    println("\("lucas":>10): \(nStep([2, 1], 10).joined(" "))");
}
