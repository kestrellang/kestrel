// Rosetta Code: Palindrome detection
// https://rosettacode.org/wiki/Palindrome_detection
module Main

func isPalindrome(s: String) -> Bool {
    let chars = s.chars.iter().collect();
    let n = chars.count;
    (0..<(n / 2)).iter().all { chars(it) == chars(n - 1 - it) }
}

// Inexact version: ignore case and anything that isn't a letter or digit.
func isPalindromeInexact(s: String) -> Bool {
    let cleaned = s.chars.iter().filter { it.isAsciiAlphanumeric }.collect().joined("");
    isPalindrome(cleaned.lowercased())
}

@main
func main() {
    for word in ["rotor", "racecar", "kestrel", "A man, a plan, a canal: Panama!"] {
        println("\"\(word)\": exact \(isPalindrome(word)), inexact \(isPalindromeInexact(word))");
    }
}
