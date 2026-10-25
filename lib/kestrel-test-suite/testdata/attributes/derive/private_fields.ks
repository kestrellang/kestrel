// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0
//
// Derived code reads private and fileprivate fields: it sees the type as
// code written inside it would (design §C8, Visibility).

module Main

import std.numeric.(Int64)

@derive(Equatable, Comparable, Formattable)
public struct Account {
    public var id: Int64
    private var balance: Int64
    fileprivate var note: String

    public init(id: Int64, balance: Int64) {
        self.id = id;
        self.balance = balance;
        self.note = "n";
    }
}

@main
func main() -> Int64 {
    let a = Account(1, 10);
    if a != Account(1, 10) { return 1 }
    if a == Account(1, 11) { return 2 }
    if a >= Account(1, 11) { return 3 }
    if "\(a)" != "Account(id: 1, balance: 10, note: n)" { return 4 }
    0
}
