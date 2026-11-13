// Rosetta Code: Substring
// https://rosettacode.org/wiki/Substring
module Main

@main
func main() {
    let s = "abcdefgh";
    let n: Int64 = 2;
    let m: Int64 = 3;
    let chars = s.chars;
    // Starting n characters in and m characters long.
    println(chars.substring(n..<(n + m)));
    // Starting n characters in, up to the end.
    println(chars.substring(n..));
    // Whole string minus the last character.
    println(chars.substring(..<(chars.count - 1)));
    // Starting from a known character.
    let d = chars.firstIndex(of: 'd') ?? chars.startIndex;
    println(chars.slice(from: d, to: chars.endIndex).toOwned().chars.substring(..<m));
    // Starting from a known substring.
    let cd = s.firstIndex(of: "cd") ?? s.bytes.startIndex;
    println(s.bytes.slice(from: cd, to: s.bytes.endIndex).toOwned().chars.substring(..<m));
}
