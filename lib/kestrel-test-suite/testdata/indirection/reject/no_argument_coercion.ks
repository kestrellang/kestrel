// test: diagnostics
// stdlib: true

// THE line that never moves (R6): the receiver peels, arguments NEVER
// coerce. A wrapper passed where its pointee is expected is a clean type
// error — never a silent conversion. This is the single most important
// regression pin: if this compiles, the deref-coercion half leaked in.
module Test

import std.memory.(RcBox)
import std.numeric.(Int64)

struct Account {
    var balance: Int64
}

func wants(a: Account) -> Int64 { a.balance }

@main
func main() -> lang.i64 {
    let rc = RcBox(Account(balance: 1));
    let n = wants(rc); // ERROR
    0
}
