// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: len=3\n

// The `[T]` sugar binds to the stdlib's Array through the
// `@builtin(.ArrayTypeOperator)` lang item, resolved in the stdlib's own
// scope. A user type that happens to be named `Array` must not capture it.
//
// Before that was wired, `lower_sugar_type` resolved the hardcoded string
// "Array" with `context: owner` — the *user's* scope — so this file's `Array`
// won, `[Int]` became the user struct, and the array literal below failed with
// "Array[Int64] !: _ExpressibleByArrayLiteral" (fragility audit F33i).

module Main
import std.io.stdio.println

struct Array[T] {
    let sneaky: Int;
}

@main
func main() {
    let xs: [Int] = [1, 2, 3];
    println("len=\(xs.count)");
}
