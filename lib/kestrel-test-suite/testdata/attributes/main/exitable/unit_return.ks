// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: ran\n

// A `()`-returning `@main` exits 0 (the wrapper special-cases unit, which is
// not itself `Exitable`).

module Main
import std.io.stdio.println

@main
func main() { println("ran"); }
