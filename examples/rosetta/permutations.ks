// Rosetta Code: Permutations
// https://rosettacode.org/wiki/Permutations
module Main

// Heap's algorithm would be faster; this generates them in lexicographic order.
func nextPermutation(mutating a: Array[Int64]) -> Bool {
    var i = a.count - 2;
    while i >= 0 and a(i) >= a(i + 1) { i = i - 1; }
    if i < 0 { return false }
    var j = a.count - 1;
    while a(j) <= a(i) { j = j - 1; }
    a.swap(at: i, with: j);
    // Reverse the suffix after i.
    var lo = i + 1;
    var hi = a.count - 1;
    while lo < hi {
        a.swap(at: lo, with: hi);
        lo = lo + 1;
        hi = hi - 1;
    }
    true
}

@main
func main() {
    var p = [1, 2, 3];
    loop {
        println(p);
        if not nextPermutation(p) { break }
    }
}
