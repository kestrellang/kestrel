// test: execution
// stdlib: true
// expect-stdout: set\nwhile ok\n

// An `if`/`while` condition that is an element of an Array[Bool] should read
// through the subscript's reference like any other operand. Today it fails
// with E101 "if condition must be Bool ... expected Bool, found &Bool", while
// `flags(i) == true` and `x and flags(i)` both compile.
// Found while writing examples/rosetta (sieve, 100 doors).

module Test

@main
func main() {
    var flags = Array[Bool](repeating: false, count: 4);
    flags(2) = true;
    if flags(2) { println("set"); }
    var i: Int64 = 2;
    while flags(i) {
        println("while ok");
        i = i + 1;
    }
}
