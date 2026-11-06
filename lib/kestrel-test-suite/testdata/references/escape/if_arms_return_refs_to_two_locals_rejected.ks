// test: diagnostics
// stdlib: true

// Fuzz A3: an `if` in return position of a ref-returning function whose arms
// name two different locals. Arms yield owned values (arm-value decay), so the
// returned reference roots at the merge temporary. Must report exactly one
// E494 and no OSSA verification failure afterwards.
module Test

import std.numeric.(Int64)

func pick(c: Bool) -> &Int64 {
    let a: Int64 = 1;
    let b: Int64 = 2;
    if c { a } else { b } // ERROR(E494)
}
