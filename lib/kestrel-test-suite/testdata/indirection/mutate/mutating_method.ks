// test: execution
// stdlib: true
// backends: cranelift,llvm

// Indirection mutating-method peel (R4): a `mutating` method on the pointee
// selects `pointeeMutRef()`; the side effect persists in the wrapped value.
module Test

import std.memory.(RcBox)
import std.numeric.(Int64)

struct Account {
    var balance: Int64
    mutating func deposit(amount: Int64) {
        self.balance = self.balance + amount;
    }
}

@main
func main() -> lang.i64 {
    var rc = RcBox(Account(balance: 0));
    rc.deposit(100);
    rc.deposit(5);
    if rc.balance != 105 { return 1; }
    0
}
