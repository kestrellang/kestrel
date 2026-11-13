// Rosetta Code: Sorting algorithms/Comb sort
// https://rosettacode.org/wiki/Sorting_algorithms/Comb_sort
module Main

func combSort[T](mutating a: Array[T]) where T: Comparable {
    var gap = a.count;
    var sorted = false;
    while not sorted {
        gap = gap * 10 / 13;
        if gap <= 1 {
            gap = 1;
            sorted = true;
        }
        for i in 0..<(a.count - gap) {
            if a(i) > a(i + gap) {
                a.swap(at: i, with: i + gap);
                sorted = false;
            }
        }
    }
}

@main
func main() {
    var a = [88, 18, 31, 44, 4, 0, 8, 81, 14, 78, 20, 76, 84, 33, 73, 75, 82, 5, 62, 70];
    combSort(a);
    println(a);
}
