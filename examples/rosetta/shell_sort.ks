// Rosetta Code: Sorting algorithms/Shell sort
// https://rosettacode.org/wiki/Sorting_algorithms/Shell_sort
module Main

func shellSort[T](mutating a: Array[T]) where T: Comparable, T: Copyable {
    var gap = a.count / 2;
    while gap > 0 {
        for i in gap..<a.count {
            let value = a(i);
            var j = i;
            while j >= gap and a(j - gap) > value {
                a(j) = a(j - gap);
                j = j - gap;
            }
            a(j) = value;
        }
        gap = if gap == 2 { 1 } else { gap * 5 / 11 };
    }
}

@main
func main() {
    var a = [4, 65, 2, -31, 0, 99, 2, 83, 782, 1, 17, -8, 33];
    shellSort(a);
    println(a);
}
