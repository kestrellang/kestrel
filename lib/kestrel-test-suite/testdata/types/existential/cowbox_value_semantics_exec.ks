// test: execution
// stdlib: true
// expect-exit: 0
//
// `any[CowBox] P` gives value semantics: a mutation forks a shared payload
// (design §3). Needs `P: Cloneable`.

module Test

import std.numeric.(Int64)
import std.core.(Cloneable)
import std.memory.(CowBox)

protocol Counter: Cloneable {
    func value() -> Int64
    mutating func increment()
}

struct Tally: Cloneable {
    var n: Int64;
    public func clone() -> Tally { Tally(n: self.n) }
}
extend Tally: Counter {
    public func value() -> Int64 { self.n }
    public mutating func increment() { self.n = self.n + 1; }
}

@main
func main() -> Int64 {
    var a: any[CowBox] Counter = Tally(n: 0);
    var b = a;
    b.increment();
    if a.value() != 0 { return 1 }
    if b.value() != 1 { return 2 }
    0
}
