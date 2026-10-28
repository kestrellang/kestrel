// test: execution
// stdlib: true
// backends: cranelift,llvm

// Wrapper-wins (R2) + explicit pointee reach (R3): `rc.clone()` is RcBox's
// own clone (shares storage), NOT the pointee's. `rc.pointeeRef().clone()`
// forces Account's clone (an independent copy). Distinguished by whether a
// later mutation through `rc` is observed.
module Test

import std.memory.(RcBox)
import std.numeric.(Int64)

struct Account: Cloneable {
    var balance: Int64
    func clone() -> Account { Account(balance: self.balance) }
    mutating func deposit(amount: Int64) {
        self.balance = self.balance + amount;
    }
}

@main
func main() -> lang.i64 {
    var rc = RcBox(Account(balance: 1));
    let shared = rc.clone();               // RcBox.clone — wrapper wins, shares storage
    let deep = rc.pointeeRef().clone();     // Account.clone — independent copy
    rc.deposit(10);                         // mutate the shared heap value
    if rc.balance != 11 { return 1; }
    if shared.balance != 11 { return 2; }   // shares storage -> sees the write
    if deep.balance != 1 { return 3; }      // independent -> does NOT
    0
}
