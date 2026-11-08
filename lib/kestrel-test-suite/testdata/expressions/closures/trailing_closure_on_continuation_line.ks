// test: execution
// stdlib: true
// expect-stdout: [3, 4]\n

// A multi-line method chain whose `.member` line takes a trailing closure.
// Today `.filter { … }` on a continuation line fails with E801 "expected `;`,
// found '{'"; only the parenthesised `.filter(where: { … })` form parses.
// Found while writing examples/rosetta.

module Test

@main
func main() {
    let ys = [1, 2, 3, 4].iter()
        .filter { it > 2 }
        .collect();
    println(ys);
}
