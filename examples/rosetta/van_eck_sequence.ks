// Rosetta Code: Van Eck sequence
// https://rosettacode.org/wiki/Van_Eck_sequence
module Main

@main
func main() {
    let n = 1000;
    var seq = Array[Int64](repeating: 0, count: n);
    // last(v) = most recent index at which v appeared (before the current term).
    var last = Dictionary[Int64, Int64]();
    for i in 0..<(n - 1) {
        let v = seq(i);
        seq(i + 1) = match last(v) {
            some j => i - j,
            null => 0,
        };
        last.insert(v, i);
    }
    println("Terms 1-10:    \(seq.prefix(10).toArray().joined(by: " "))");
    println("Terms 991-1000: \(seq.suffix(10).toArray().joined(by: " "))");
}
