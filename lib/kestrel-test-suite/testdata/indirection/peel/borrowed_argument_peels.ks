// test: execution
// stdlib: true
// expect-stdout: 42 5 15 7 3 8\n

// A borrowed argument peels through `Indirection` to reach its param (R6):
// `x: T` and `x: &T` take the wrapper's `value`, `mutating x: T` and
// `x: &mutating T` its `mutating ref`, and nested wrappers peel once per
// level. Writes through the peel land in the wrapper's payload.
module Test

import std.memory.(OwnedBox, RcBox)

struct Account {
    var balance: Int64
}

func plusOne(x: &Int64) -> Int64 { x + 1 }
func show(a: Account) -> Int64 { a.balance }
func showRef(a: &Account) -> Int64 { a.balance }
func bump(a: &mutating Account) { a.balance += 10; }
func bumpOne(mutating a: Account) { a.balance += 1; }

@main
func main() {
    let b = OwnedBox(41);
    var acct = OwnedBox(Account(balance: 5));
    let before = show(acct);
    bump(acct);
    let rc = RcBox(Account(balance: 7));
    let nested = OwnedBox(OwnedBox(Account(balance: 3)));
    var other = RcBox(Account(balance: 7));
    bumpOne(other);
    println("\(plusOne(b)) \(before) \(showRef(acct)) \(show(rc)) \(show(nested)) \(other.balance)");
}
