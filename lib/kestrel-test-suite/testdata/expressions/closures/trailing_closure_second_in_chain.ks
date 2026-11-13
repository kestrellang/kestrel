// test: execution
// stdlib: true
// expect-stdout: [4, 6]\n[3, 5, 7]\n

// A second trailing closure in one postfix chain should attach to its call,
// like the first one does. Today only the first `{ … }` after the primary is
// taken: `.map { … }` after `.filter { … }` fails with E801 "expected `;`,
// found '{'" because postfix()'s `.member` continuation loop never calls
// trailing_closures (lib/kestrel-parser/src/grammar/exprs.rs).
// Found while writing examples/rosetta.

module Test

@main
func main() {
    let a = [1, 2, 3].iter().filter { it > 1 }.map { it * 2 }.collect();
    println(a);
    let b = [1, 2, 3].iter().map { it * 2 }.map { it + 1 }.collect();
    println(b);
}
