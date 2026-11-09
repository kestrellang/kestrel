// Rosetta Code: Sorting algorithms/Cocktail sort
// https://rosettacode.org/wiki/Sorting_algorithms/Cocktail_sort
module Main

// Bubble sort that alternates direction on each pass.
func cocktailSort[T](mutating a: Array[T]) where T: Comparable {
    var lo = 0;
    var hi = a.count - 1;
    var swapped = true;
    while swapped {
        swapped = false;
        for i in lo..<hi {
            if a(i) > a(i + 1) { a.swap(at: i, with: i + 1); swapped = true; }
        }
        hi = hi - 1;
        var i = hi;
        while i > lo {
            if a(i - 1) > a(i) { a.swap(at: i - 1, with: i); swapped = true; }
            i = i - 1;
        }
        lo = lo + 1;
    }
}

@main
func main() {
    var a = [5, -1, 101, -4, 0, 1, 8, 6, 2, 3];
    cocktailSort(a);
    println(a);
}
