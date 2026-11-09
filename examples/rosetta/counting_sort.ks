// Rosetta Code: Sorting algorithms/Counting sort
// https://rosettacode.org/wiki/Sorting_algorithms/Counting_sort
module Main

func countingSort(a: Array[Int64]) -> Array[Int64] {
    let lo = a.min() ?? 0;
    let hi = a.max() ?? 0;
    var counts = Array[Int64](repeating: 0, count: hi - lo + 1);
    for x in a { counts(x - lo) = counts(x - lo) + 1; }
    var out = Array[Int64](capacity: a.count);
    for (i, c) in counts.iter().enumerate() {
        for _ in 0..<c { out.append(i + lo); }
    }
    out
}

@main
func main() {
    println(countingSort([4, 65, 2, -31, 0, 99, 2, 83, 782, 1]));
}
