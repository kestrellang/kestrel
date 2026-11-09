// Rosetta Code: Count occurrences of a substring
// https://rosettacode.org/wiki/Count_occurrences_of_a_substring
module Main

// Non-overlapping matches, scanning left to right.
func countSubstring(s: String, sub: String) -> Int64 {
    s.split(sub).count - 1
}

@main
func main() {
    println(countSubstring("the three truths", "th"));
    println(countSubstring("ababababab", "abab"));
}
