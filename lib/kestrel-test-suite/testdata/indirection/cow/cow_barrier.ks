// test: execution
// stdlib: true
// backends: cranelift,llvm

// CoW barrier for free (R4 / Wave C): CowBox's `pointeeMutRef()` runs the
// copy-on-write fork before handing out `&mutating T`, so a mutating-method
// peel through a *shared* CowBox forks — the prior alias is NOT mutated.
module Test

import std.memory.(CowBox)
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
    var a = CowBox(Account(balance: 10));
    let b = a.clone();        // shares storage with `a`
    a.deposit(5);             // peels via pointeeMutRef -> COW fork
    if a.balance != 15 { return 1; }   // writer sees the mutation
    if b.balance != 10 { return 2; }   // the shared alias does NOT
    0
}
