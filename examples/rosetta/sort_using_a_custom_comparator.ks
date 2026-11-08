// Rosetta Code: Sort using a custom comparator
// https://rosettacode.org/wiki/Sort_using_a_custom_comparator
module Main

@main
func main() {
    let words = ["Here", "are", "some", "sample", "strings", "to", "be", "sorted"];
    // Longest first, then case-insensitive alphabetical.
    let sorted = words.sorted(by: { (a, b) in
        if a.chars.count != b.chars.count { return a.chars.count > b.chars.count }
        a.lowercased() < b.lowercased()
    });
    println(sorted);
}
