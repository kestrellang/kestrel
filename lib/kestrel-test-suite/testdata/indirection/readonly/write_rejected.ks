// test: diagnostics
// stdlib: true

// Read-only smart pointer (R4 / D2): a wrapper conforming to `Indirection`
// but NOT `MutableIndirection` reads through fine, but a write is rejected
// (`indirection_no_mutating_accessor`) — there is no `pointeeMutRef()`.
module Test

import std.memory.(RcBox)
import std.numeric.(Int64)
import std.core.(Indirection)

struct Account {
    var balance: Int64
}

struct ReadView {
    var inner: RcBox[Account]
}

extend ReadView: Indirection {
    type Target = Account
    public func pointeeRef() -> &Account { self.inner.pointeeRef() }
}

@main
func main() -> lang.i64 {
    var ro = ReadView(inner: RcBox(Account(balance: 0)));
    let n = ro.balance;   // read peels fine
    ro.balance = 5;       // ERROR
    0
}
