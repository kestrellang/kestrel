// Rosetta Code: Sorting algorithms/Bubble sort
// https://rosettacode.org/wiki/Sorting_algorithms/Bubble_sort
module Main

func bubbleSort[T](mutating items: Array[T]) where T: Comparable {
    var n = items.count;
    var swapped = true;
    while swapped {
        swapped = false;
        for i in 1..<n {
            if items(i - 1) > items(i) {
                items.swap(at: i - 1, with: i);
                swapped = true;
            }
        }
        n = n - 1;
    }
}

@main
func main() {
    var numbers = [5, 3, 8, 1, 9, 2, 7, 4, 6, 0];
    bubbleSort(numbers);
    println(numbers);
    var words = ["pear", "apple", "fig", "banana"];
    bubbleSort(words);
    println(words);
}
