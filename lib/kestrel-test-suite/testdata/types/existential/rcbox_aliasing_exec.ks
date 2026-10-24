// test: execution
// stdlib: true
// expect-exit: 0
//
// The default box is RcBox: copies share one payload, so a mutation through
// one copy is visible through the other (design §3).

module Test

import std.numeric.(Int64)

protocol Counter {
    func get() -> Int64
    mutating func increment()
}

struct Tally { var n: Int64; }
extend Tally: Counter {
    public func get() -> Int64 { self.n }
    public mutating func increment() { self.n = self.n + 1; }
}

@main
func main() -> Int64 {
    var a: any Counter = Tally(n: 0);
    var b = a;
    b.increment();
    if a.get() != 1 { return 1 }
    a.increment();
    if b.get() != 2 { return 2 }
    0
}
