// Rosetta Code: Remove duplicate elements
// https://rosettacode.org/wiki/Remove_duplicate_elements
module Main

@main
func main() {
    let items = [1, 2, 3, 1, 2, 5, 1, 7, 3, 9];
    // unique() keeps the first occurrence of each element, in order.
    println(items.unique());
    // Sort, then drop adjacent repeats.
    var sorted = items.sorted();
    sorted.dedup();
    println(sorted);
    let words = ["the", "cat", "sat", "on", "the", "mat", "cat"];
    println(words.unique());
}
