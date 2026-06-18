// test: execution
// stdlib: true
// backends: cranelift,llvm

// Indirection read peel for a method call (R1): a non-mutating method
// missing on the wrapper dispatches to the pointee via pointeeRef().
module Test

import std.memory.(RcBox)
import std.numeric.(Int64)

struct Account {
    var balance: Int64
    func describe() -> Int64 { self.balance + 1000 }
}

@main
func main() -> lang.i64 {
    let rc = RcBox(Account(balance: 42));
    if rc.describe() != 1042 { return 1; }
    0
}
