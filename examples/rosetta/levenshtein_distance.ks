// Rosetta Code: Levenshtein distance
// https://rosettacode.org/wiki/Levenshtein_distance
module Main

func levenshtein(a: String, b: String) -> Int64 {
    let s = a.chars.iter().collect();
    let t = b.chars.iter().collect();
    // Only the previous row of the DP table is needed.
    var prev = Array(of: t.count + 1, generatedBy: { it });
    for i in 1..=s.count {
        var row = Array[Int64](repeating: i, count: t.count + 1);
        for j in 1..=t.count {
            let cost: Int64 = if s(i - 1) == t(j - 1) { 0 } else { 1 };
            var best = prev(j - 1) + cost;
            if prev(j) + 1 < best { best = prev(j) + 1; }
            if row(j - 1) + 1 < best { best = row(j - 1) + 1; }
            row(j) = best;
        }
        prev = row;
    }
    prev(t.count)
}

@main
func main() {
    for (a, b) in [("kitten", "sitting"), ("rosettacode", "raisethysword"), ("saturday", "sunday"), ("", "abc")] {
        println("\(a) -> \(b): \(levenshtein(a, b))");
    }
}
