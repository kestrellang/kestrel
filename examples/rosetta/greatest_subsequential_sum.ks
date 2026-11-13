// Rosetta Code: Greatest subsequential sum
// https://rosettacode.org/wiki/Greatest_subsequential_sum
module Main

// Kadane's algorithm, tracking where the best run starts and ends.
func maxSubarray(a: Array[Int64]) -> (Int64, Int64, Int64) {
    var best = 0;
    var bestStart = 0;
    var bestEnd = 0;
    var current = 0;
    var start = 0;
    for (i, x) in a.iter().enumerate() {
        current = current + x;
        if current < 0 {
            current = 0;
            start = i + 1;
        } else if current > best {
            best = current;
            bestStart = start;
            bestEnd = i + 1;
        }
    }
    (best, bestStart, bestEnd)
}

@main
func main() {
    let a = [-1, -2, 3, 5, 6, -2, -1, 4, -4, 2, -1];
    let (sum, from, to) = maxSubarray(a);
    println("Max sum \(sum) from \(a(from..<to).toOwned())");
    let (s2, _, _) = maxSubarray([-1, -2, -3]);
    println("All negative: \(s2) (the empty subsequence)");
}
