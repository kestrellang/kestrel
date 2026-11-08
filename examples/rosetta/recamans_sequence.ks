// Rosetta Code: Recaman's sequence
// https://rosettacode.org/wiki/Recaman%27s_sequence
module Main

import std.collections.Set

@main
func main() {
    var seq: Array[Int64] = [0];
    var seen: Set[Int64] = [0];
    var firstDuplicate: Int64? = .None;
    var n: Int64 = 1;
    // Keep going until every integer 0..1000 has been generated.
    var covered = Array[Bool](repeating: false, count: 1001);
    covered(0) = true;
    var coveredCount: Int64 = 1;
    while coveredCount < 1001 {
        let prev = seq(n - 1);
        let back = prev - n;
        let next = if back > 0 and not seen.contains(back) { back } else { prev + n };
        if not seen.insert(next) and firstDuplicate == .None { firstDuplicate = .Some(next); }
        if next <= 1000 and covered(next) == false {
            covered(next) = true;
            coveredCount = coveredCount + 1;
        }
        seq.append(next);
        n = n + 1;
    }
    println("First 15: \(seq.prefix(15).toArray())");
    println("First duplicate: \(firstDuplicate ?? -1)");
    println("Terms needed to cover 0..1000: \(seq.count - 1)");
}
