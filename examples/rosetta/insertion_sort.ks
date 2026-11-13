// Rosetta Code: Sorting algorithms/Insertion sort
// https://rosettacode.org/wiki/Sorting_algorithms/Insertion_sort
module Main

func insertionSort[T](mutating a: Array[T]) where T: Comparable {
    for i in 1..<a.count {
        var j = i;
        while j > 0 and a(j - 1) > a(j) {
            a.swap(at: j - 1, with: j);
            j = j - 1;
        }
    }
}

@main
func main() {
    var a = [4, 65, 2, -31, 0, 99, 2, 83, 782, 1];
    insertionSort(a);
    println(a);
}
