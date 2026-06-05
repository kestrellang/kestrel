// test: diagnostics
// stdlib: true

module Test

import std.numeric.(Int64)

// Control: a plain (non-mutating) closure param is still immutable.
func run(f: (Int64) -> Int64) -> Int64 { f(3) }

func main() -> lang.i64 {
    let _ = run({ (x) in x = x + 1; x }); // ERROR: cannot assign to immutable variable
    0
}
