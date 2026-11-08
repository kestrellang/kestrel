// Rosetta Code: Sorting algorithms/Pancake sort
// https://rosettacode.org/wiki/Sorting_algorithms/Pancake_sort
module Main

// Reverse the first k elements.
func flip(mutating a: Array[Int64], k: Int64) {
    var lo: Int64 = 0;
    var hi = k - 1;
    while lo < hi {
        a.swap(at: lo, with: hi);
        lo = lo + 1;
        hi = hi - 1;
    }
}

func pancakeSort(mutating a: Array[Int64]) -> Int64 {
    var flips: Int64 = 0;
    var size = a.count;
    while size > 1 {
        // Find the largest pancake in the unsorted stack, flip it to the top, then to the bottom.
        var top: Int64 = 0;
        for i in 1..<size { if a(i) > a(top) { top = i; } }
        if top != size - 1 {
            if top != 0 { flip(a, top + 1); flips = flips + 1; }
            flip(a, size);
            flips = flips + 1;
        }
        size = size - 1;
    }
    flips
}

@main
func main() {
    var a = [6, 7, 8, 9, 2, 5, 3, 4, 1];
    let flips = pancakeSort(a);
    println("\(a) in \(flips) flips");
}
