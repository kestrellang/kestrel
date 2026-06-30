// test: diagnostics
// stdlib: true

module Test

import std.numeric.(Int64)

// Regression (#174): a capturing closure cannot escape its defining frame —
// its environment is stack-allocated there. The old syntactic check (E605)
// only saw the closure LITERAL in return position and missed laundering
// through a `let` binding. The provenance escape check (E494) follows the
// closure's root through the binding and rejects it.
func makeAdder(n: Int64) -> (Int64) -> Int64 {
    let f = { (x: Int64) in x + n };
    f // ERROR(E494)
}

func main() -> lang.i64 { 0 }
