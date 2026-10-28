// test: diagnostics
// stdlib: true

// #210: an ambiguous call to an overloaded MODULE-LEVEL function has no
// receiver. The diagnostic must name the call plainly ("ambiguous call to
// 'f'") and must NOT leak the synthetic placeholder receiver as the internal
// `Error` type (the bug rendered "ambiguous member 'f': Error.f ambiguous").

module Test

import std.numeric.Int64

func f(a: Int64) -> Int64 { a }
func f(a: Int64, b: Int64 = 0) -> Int64 { a }

func test() -> Int64 {
    let r = f(1); // ERROR: ambiguous call to 'f'
    r
}
