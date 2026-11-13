// Rosetta Code: Sorting algorithms/Quicksort
// https://rosettacode.org/wiki/Sorting_algorithms/Quicksort
module Main

// Functional version: partition around the first element.
func quicksort[T](items: Array[T]) -> Array[T] where T: Comparable {
    if items.count <= 1 { return items }
    let pivot = items(0);
    let rest = items.drop(first: 1);
    var result = quicksort(rest.filter { it < pivot });
    result.append(pivot);
    result.append(contentsOf: quicksort(rest.filter { it >= pivot }));
    result
}

// In-place version (Lomuto partition).
func quicksortInPlace(mutating a: Array[Int64], lo: Int64, hi: Int64) {
    if lo >= hi { return }
    let pivot = a(hi);
    var i = lo;
    for j in lo..<hi {
        if a(j) < pivot {
            a.swap(at: i, with: j);
            i = i + 1;
        }
    }
    a.swap(at: i, with: hi);
    quicksortInPlace(a, lo, i - 1);
    quicksortInPlace(a, i + 1, hi);
}

@main
func main() {
    let numbers = [65, 2, -31, 0, 99, 2, 83, 782, 1];
    println(quicksort(numbers));
    var inPlace = numbers;
    quicksortInPlace(inPlace, 0, inPlace.count - 1);
    println(inPlace);
}
