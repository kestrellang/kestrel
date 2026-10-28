// test: execution
// stdlib: true
// backends: cranelift,llvm

// Bounded transitivity (R5): RcBox[RcBox[Account]] reaches Account through
// two peels. The outer wrapper has no `balance` -> peel -> inner RcBox has
// no `balance` -> peel -> Account.balance. A write threads back through both
// pointeeMutRef() projections.
module Test

import std.memory.(RcBox)
import std.numeric.(Int64)

struct Account {
    var balance: Int64
}

@main
func main() -> lang.i64 {
    var rc = RcBox(RcBox(Account(balance: 7)));
    if rc.balance != 7 { return 1; }   // read through two levels
    rc.balance = 99;                    // write through two levels
    if rc.balance != 99 { return 2; }
    0
}
