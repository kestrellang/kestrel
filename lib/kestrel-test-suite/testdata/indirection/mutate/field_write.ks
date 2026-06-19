// test: execution
// stdlib: true
// backends: cranelift,llvm

// Indirection write peel (R4): `rc.balance = v` routes through the
// MutableIndirection `pointeeMutRef()` half. The mutation is visible on a
// subsequent read through the same wrapper.
module Test

import std.memory.(RcBox)
import std.numeric.(Int64)

struct Account {
    var balance: Int64
}

@main
func main() -> lang.i64 {
    var rc = RcBox(Account(balance: 0));
    rc.balance = 100;
    if rc.balance != 100 { return 1; }
    0
}
