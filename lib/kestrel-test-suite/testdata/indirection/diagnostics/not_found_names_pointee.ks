// test: diagnostics
// stdlib: true

// D1: a member found on neither the wrapper nor its pointee errors, and the
// message names BOTH (the wrapper conformed, so the peel was attempted and
// the pointee was searched too).
module Test

import std.memory.(RcBox)
import std.numeric.(Int64)

struct Account {
    var balance: Int64
}

@main
func main() -> lang.i64 {
    let rc = RcBox(Account(balance: 1));
    let n = rc.nonexistent; // ERROR
    0
}
