// test: diagnostics
// stdlib: true

module Test

import std.numeric.(Int64)

func run(f: (Int64) -> Int64) -> Int64 { f(3) }

func main() -> lang.i64 {
    // `mutating consuming` on one closure param is contradictory.
     run({ (mutating consuming x) in x }); // ERROR:
    0
}
