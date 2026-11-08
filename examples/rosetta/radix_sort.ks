// Rosetta Code: Sorting algorithms/Radix sort
// https://rosettacode.org/wiki/Sorting_algorithms/Radix_sort
module Main

// LSD radix sort, base 10. Negative numbers are handled by offsetting.
func radixSort(input: Array[Int64]) -> Array[Int64] {
    let offset = input.min() ?? 0;
    var a = input.map { it - offset };
    let largest = a.max() ?? 0;
    var place: Int64 = 1;
    while largest / place > 0 {
        var buckets = Array(of: 10, generatedBy: { (_) in Array[Int64]() });
        for x in a { buckets((x / place) % 10).append(x); }
        a = buckets.flatten();
        place = place * 10;
    }
    a.map { it + offset }
}

@main
func main() {
    println(radixSort([170, 45, 75, -90, -802, 24, 2, 66]));
}
