// test: execution
// stdlib: true
// expect-exit: 0

// #211: a `{integer-literal, float-literal}` pair must unify uniformly across
// contexts. Unifying two literals through a generic call (`pick(1, 2.5)`)
// already resolved to Float64; the SAME pair in an array literal and in an
// if-expression was rejected ("expected integer literal got float literal",
// with the if-branch message also swapped). The int->float relaxation now
// lives in the literal-marker merge in `unify`, so element/branch unification
// (Equal) behaves identically to call-argument unification (Coerce).

module Test

import std.numeric.Float64

func pick[T](a: T, b: T) -> T { b }

@main
func main() -> lang.i32 {
    // Generic call (always worked) — establishes the expected unification.
    let viaCall = pick(1, 2.5);
    if viaCall != 2.5 { return 10 }

    // Array literal: int + float elements unify to Float64.
    let xs = [1, 2, 2.5];
    if xs(0) != 1.0 { return 20 }
    if xs(2) != 2.5 { return 21 }
    // Arithmetic confirms the elements are genuinely Float64, not Int.
    let s = xs(0) + xs(2);
    if s != 3.5 { return 22 }

    // If-expression branches: int + float unify to Float64.
    let t = if false { 1 } else { 2.5 };
    if t != 2.5 { return 30 }
    let u = if true { 2 } else { 1.5 };
    if u != 2.0 { return 31 }

    0
}
