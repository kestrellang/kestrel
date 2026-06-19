// test: execution
// stdlib: true
// backends: cranelift,llvm

// Operators forward via an explicit `extend`, NOT the member peel (R7).
// With `extend RcBox[Account]: Equatable`, `==` / `!=` compare the pointees.
module Test

import std.memory.(RcBox)
import std.numeric.(Int64)
import std.core.(Bool, Equatable)

struct Account: Equatable {
    var balance: Int64
    func isEqual(to other: Self) -> Bool { self.balance == other.balance }
}

extend RcBox[Account]: Equatable {
    public func isEqual(to other: Self) -> Bool {
        self.getValue() == other.getValue()
    }
}

@main
func main() -> lang.i64 {
    let a = RcBox(Account(balance: 5));
    let b = RcBox(Account(balance: 5));
    let c = RcBox(Account(balance: 9));
    if a != b { return 1; }   // a == b
    if a == c { return 2; }   // a != c
    0
}
