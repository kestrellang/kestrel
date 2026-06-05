// test: diagnostics
// stdlib: true

module Test

import std.numeric.(Int64)

// `run` expects a borrowing closure; passing a `mutating` literal must error
// (MutBorrow is not assignable where Borrow is expected — variance).
func run(f: (Int64) -> Int64) -> Int64 { f(3) }

func main() -> lang.i64 {
    let _ = run({ (mutating x) in x = x + 1; x }); // ERROR: convention
    0
}
