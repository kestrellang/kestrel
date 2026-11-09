// Rosetta Code: Sorting algorithms/Selection sort
// https://rosettacode.org/wiki/Sorting_algorithms/Selection_sort
module Main

func selectionSort[T](mutating a: Array[T]) where T: Comparable {
    for i in 0..<a.count {
        var smallest = i;
        for j in (i + 1)..<a.count {
            if a(j) < a(smallest) { smallest = j; }
        }
        if smallest != i { a.swap(at: i, with: smallest); }
    }
}

@main
func main() {
    var a = [4, 65, 2, -31, 0, 99, 2, 83, 782, 1];
    selectionSort(a);
    println(a);
}
