// Rosetta Code: Longest increasing subsequence
// https://rosettacode.org/wiki/Longest_increasing_subsequence
module Main

// Patience sorting, O(n log n), with back-pointers to rebuild the sequence.
func lis(a: Array[Int64]) -> Array[Int64] {
    var tails = Array[Int64]();      // index of the smallest tail of each length
    var parent = Array[Int64](repeating: -1, count: a.count);
    for i in 0..<a.count {
        var lo: Int64 = 0;
        var hi = tails.count;
        while lo < hi {
            let mid = (lo + hi) / 2;
            if a(tails(mid)) < a(i) { lo = mid + 1; } else { hi = mid; }
        }
        if lo > 0 { parent(i) = tails(lo - 1); }
        if lo == tails.count { tails.append(i); } else { tails(lo) = i; }
    }
    var out = Array[Int64]();
    var k = tails.last() ?? -1;
    while k >= 0 {
        out.insert(a(k), at: 0);
        k = parent(k);
    }
    out
}

@main
func main() {
    println(lis([3, 2, 6, 4, 5, 1]));
    println(lis([0, 8, 4, 12, 2, 10, 6, 14, 1, 9, 5, 13, 3, 11, 7, 15]));
}
