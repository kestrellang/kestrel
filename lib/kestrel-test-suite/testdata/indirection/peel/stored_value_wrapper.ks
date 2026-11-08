// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-stdout: 42 47 48 | 7 8\n

// `Indirection` requires a `value` place, so a wrapper can meet it with a
// plain stored field: the peel lends it in place through the witness.
// Generic code over `W: MutableIndirection` reads and writes through it.
module Test

import std.core.(Indirection, MutableIndirection)
import std.numeric.(Int64)

struct Account {
    var balance: Int64

    mutating func deposit(amount: Int64) { self.balance = self.balance + amount; }
}

struct Holder {
    var value: Account
}

extend Holder: MutableIndirection {
    type Target = Account
}

func bump[W](w: &mutating W) where W: MutableIndirection, W.Target = Account {
    w.value.deposit(1)
}

func read[W](w: &W) -> Int64 where W: Indirection, W.Target = Account { w.value.balance }

@main
func main() -> lang.i64 {
    var h = Holder(value: Account(balance: 42));
    let before = h.balance;
    h.deposit(5);
    let after = h.balance;
    bump(h);
    var k = Holder(value: Account(balance: 7));
    let r = read(k);
    bump(k);
    print("\(before) \(after) \(read(h)) | \(r) \(k.balance)\n");
    0
}
