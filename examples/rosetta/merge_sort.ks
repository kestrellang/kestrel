// Rosetta Code: Sorting algorithms/Merge sort
// https://rosettacode.org/wiki/Sorting_algorithms/Merge_sort
module Main

func merge[T](left: Array[T], right: Array[T]) -> Array[T] where T: Comparable {
    var result = Array[T](capacity: left.count + right.count);
    var i = 0;
    var j = 0;
    while i < left.count and j < right.count {
        if left(i) <= right(j) {
            result.append(left(i));
            i = i + 1;
        } else {
            result.append(right(j));
            j = j + 1;
        }
    }
    result.append(contentsOf: left.drop(first: i));
    result.append(contentsOf: right.drop(first: j));
    result
}

func mergeSort[T](items: Array[T]) -> Array[T] where T: Comparable {
    if items.count <= 1 { return items }
    let mid = items.count / 2;
    merge(mergeSort(items.prefix(mid).toArray()), mergeSort(items.drop(first: mid).toArray()))
}

@main
func main() {
    println(mergeSort([38, 27, 43, 3, 9, 82, 10]));
    println(mergeSort(["delta", "alpha", "echo", "charlie", "bravo"]));
}
