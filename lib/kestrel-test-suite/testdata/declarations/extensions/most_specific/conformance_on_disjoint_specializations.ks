// test: execution
// stdlib: true
// expect-exit: 0

// #165: conforming a generic type to the same protocol on two DISJOINT
// concrete specializations is legal — `Box[Int64]` and `Box[Bool]` are
// different types, so their same-named `show` methods are not duplicates.
// The E412 "duplicate method" analyzer used to key its dedup on the bare
// nominal `Box`, ignoring the extension's self-type arguments, so this whole
// program failed to build. Each specialization must dispatch to its own
// witness.

module Test

import std.numeric.Int64

protocol Show { func show() -> Int64 }

struct Box[T] { var value: T }

extend Box[Int64]: Show { func show() -> Int64 { 1 } }
extend Box[Bool]: Show { func show() -> Int64 { 2 } }

@main
func main() -> lang.i32 {
    let a = Box(value: 5).show();    // Box[Int64] -> 1
    let b = Box(value: true).show(); // Box[Bool]  -> 2
    if a != 1 { return 10 }
    if b != 2 { return 20 }
    0
}
