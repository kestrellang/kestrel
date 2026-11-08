// test: execution
// stdlib: true
// expect-stdout: fdsa\ncba\n

// `String(from:)` over an Iterable of Char (the overload documented with an
// example in lang/std/text/string.ks) should be picked for an Array[Char] or
// a ReversedCharsView. Today overload resolution settles on
// `init(from: CString)` and reports E109 "expected CString got Array[Char]".
// Found while writing examples/rosetta (reverse a string, palindromes).

module Test

@main
func main() {
    let s = "asdf";
    println(String(from: s.chars.reversed));
    let chars = "abc".chars.iter().collect();
    var reversed = chars;
    reversed.reverse();
    println(String(from: reversed));
}
