// Rosetta Code: Sorting algorithms/Heapsort
// https://rosettacode.org/wiki/Sorting_algorithms/Heapsort
module Main

// Restores the max-heap property for the subtree rooted at `root`,
// looking only at a(0..<end).
func siftDown[T](mutating a: Array[T], root: Int64, end: Int64) where T: Comparable {
    var parent = root;
    loop {
        var child = 2 * parent + 1;
        if child >= end { return }
        if child + 1 < end and a(child) < a(child + 1) { child = child + 1; }
        if a(parent) >= a(child) { return }
        a.swap(at: parent, with: child);
        parent = child;
    }
}

func heapsort[T](mutating a: Array[T]) where T: Comparable {
    let n = a.count;
    var start = n / 2 - 1;
    while start >= 0 {
        siftDown(a, start, n);
        start = start - 1;
    }
    var end = n - 1;
    while end > 0 {
        a.swap(at: 0, with: end);
        siftDown(a, 0, end);
        end = end - 1;
    }
}

@main
func main() {
    var a = [12, 11, 13, 5, 6, 7, -3, 0, 42];
    heapsort(a);
    println(a);
}
