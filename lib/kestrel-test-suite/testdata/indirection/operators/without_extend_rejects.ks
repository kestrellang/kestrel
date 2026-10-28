// test: diagnostics
// stdlib: true

// Operators do NOT ride the member peel (R7). Without an explicit
// `extend RcBox: Equatable`, `rc1 == rc2` is rejected even though the
// pointee Account IS Equatable — the peel never supplies operators (it
// would require the *argument* to coerce, which never happens).
module Test

import std.memory.(RcBox)
import std.numeric.(Int64)
import std.core.(Bool, Equatable)

struct Account: Equatable {
    var balance: Int64
    func isEqual(to other: Self) -> Bool { self.balance == other.balance }
}

@main
func main() -> lang.i64 {
    let a = RcBox(Account(balance: 5));
    let b = RcBox(Account(balance: 5));
    if a == b { return 1; } // ERROR
    0
}
