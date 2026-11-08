// Rosetta Code: Sorting algorithms/Gnome sort
// https://rosettacode.org/wiki/Sorting_algorithms/Gnome_sort
module Main

func gnomeSort[T](mutating a: Array[T]) where T: Comparable {
    var i: Int64 = 1;
    while i < a.count {
        if a(i - 1) <= a(i) {
            i = i + 1;
        } else {
            a.swap(at: i - 1, with: i);
            if i > 1 { i = i - 1; }
        }
    }
}

@main
func main() {
    var a = [5, 3, 1, 4, 2, 9, 0, -3];
    gnomeSort(a);
    println(a);
}
