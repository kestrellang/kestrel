// test: execution
// stdlib: true
// expect-stdout: [a][b,,c]\n[a,b,,c]\n[a][b][][c]\n4\n0\n

// `split(by:limit:)` yields at most `limit` pieces; the last holds the rest
// of the string with its separators. A limit above the piece count is the
// same as no limit, and an empty string still has no pieces.

module Test

@main
func main() {
    let s = "a,b,,c";
    for p in s.split(by: ",", limit: 2) { print("[\(p)]") }
    println("");
    for p in s.split(by: ",", limit: 1) { print("[\(p)]") }
    println("");
    for p in s.split(by: ",", limit: 10) { print("[\(p)]") }
    println("");
    println(s.split(by: ",", limit: 10).count);
    println("".split(by: ",", limit: 2).count);
}
