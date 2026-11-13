// test: execution
// stdlib: true
// expect-stdout: <hello><world><foo>\n3\n0\ntrue\n<a>\n<b>\n

// `words` splits on runs of ASCII whitespace and never yields an empty
// word, unlike `split(by: " ")`. Works on a String and on a slice.

module Test

@main
func main() {
    for w in "  hello \t world\nfoo  ".words { print("<\(w)>") }
    println("");
    println("a b c".words.count);
    println("   ".words.count);
    println("".words.isEmpty);
    let line = "  a   b  ".trimmed();
    for w in line.words { println("<\(w)>") }
}
