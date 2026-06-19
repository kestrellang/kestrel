// test: execution
// stdlib: true
// backends: cranelift,llvm

// Indirection read peel (R1): a member missing on the wrapper resolves
// through to the pointee. `rc.balance` == `rc.pointeeRef().balance`. RcBox
// has no `balance`, so member lookup peels to the Account pointee.
module Test

import std.memory.(RcBox)
import std.numeric.(Int64)

struct Account {
    var balance: Int64
}

@main
func main() -> lang.i64 {
    let rc = RcBox(Account(balance: 42));
    if rc.balance != 42 { return 1; }
    0
}
