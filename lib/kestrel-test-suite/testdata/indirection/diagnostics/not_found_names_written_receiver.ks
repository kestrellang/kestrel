// test: diagnostics
// stdlib: true

// A member found on neither a peeled wrapper nor its pointee names the
// receiver as written first, then the pointee the peel searched too.
module Test

import std.memory.(RcBox)
import std.numeric.(Int64)

struct Account {
    var balance: Int64
}

@main
func main() -> lang.i64 {
    let rc = RcBox(Account(balance: 1));
    let n = rc.nonexistent; // ERROR: or its pointee 'Account'
    0
}
