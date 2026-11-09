// Rosetta Code: Hailstone sequence
// https://rosettacode.org/wiki/Hailstone_sequence
module Main

func hailstone(start: Int64) -> Array[Int64] {
    var n = start;
    var seq = [n];
    while n != 1 {
        n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
        seq.append(n);
    }
    seq
}

@main
func main() {
    let h27 = hailstone(27);
    println("hailstone(27) has \(h27.count) elements, starting \(h27.prefix(4).toArray()) and ending \(h27.suffix(4).toArray())");
    var best = 1;
    var bestLen = 1;
    for n in 1..<100000 {
        let len = hailstone(n).count;
        if len > bestLen {
            best = n;
            bestLen = len;
        }
    }
    println("Below 100000, \(best) has the longest sequence: \(bestLen) elements");
}
