// test: diagnostics
// stdlib: true

// A generic function called with nothing to fix its param is E105 "cannot
// infer type parameter 'T'". The call used to report the implicit Copyable
// bound instead ("_ !: Copyable", E107): a bare function reference
// recorded its type args without naming them, so the E105 check skipped
// them. Context or an explicit argument still fixes `T`.
module Test

import std.numeric.(Int64)

func f[T]() -> Int64 { 0 }

func g[T]() -> T? { .None }

func check() -> Int64 {
    let a = f(); // ERROR(E105)
    let b = f[Int64]();
    let c: Int64? = g();
    a + b
}
