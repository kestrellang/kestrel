// test: diagnostics
// stdlib: true

// R6, revised 2026-11: a BORROWED argument peels through `Indirection` to
// reach its param, so `wants(rc)` passes `rc.value`. What still never
// coerces: a `consuming` param (the wrapper would have to give up its
// payload), a mutably borrowed param through a read-only wrapper, and an
// operator's operand (R7 — `a == rc` would work while `rc == a` didn't).
module Test

import std.memory.(RcBox, OwnedBox)
import std.numeric.(Int64)
import std.core.(Indirection)

struct Account: Equatable {
    var balance: Int64
    func isEqual(to other: Account) -> Bool { self.balance == other.balance }
}

struct ReadView {
    var inner: RcBox[Account]
}

extend ReadView: Indirection {
    type Target = Account
    public var value: Account { ref { self.inner.value } }
}

func wants(a: Account) -> Int64 { a.balance }
func takes(consuming a: Account) -> Int64 { a.balance }
func bumps(mutating a: Account) { a.balance += 1; }

@main
func main() -> lang.i64 {
    let rc = RcBox(Account(balance: 1));
    let n = wants(rc);
    0
}

func consuming_never_peels() -> Int64 {
    takes(OwnedBox(Account(balance: 2))) // ERROR
}

func read_only_wrapper_never_lends_mutably() {
    var ro = ReadView(inner: RcBox(Account(balance: 0)));
    bumps(ro); // ERROR
}

func operands_never_peel() -> Bool {
    let rc = RcBox(Account(balance: 1));
    let a = Account(balance: 1);
    a == rc // ERROR
}
