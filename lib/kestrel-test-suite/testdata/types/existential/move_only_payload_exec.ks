// test: execution
// stdlib: true
// expect-exit: 0
//
// A non-Copyable payload in the default box: reads and mutating calls work
// (design edge cases).

module Test

import std.numeric.(Int64)

protocol Counter {
    func get() -> Int64
    mutating func increment()
}

struct Handle: not Copyable {
    var n: Int64;
}
extend Handle: Counter {
    public func get() -> Int64 { self.n }
    public mutating func increment() { self.n = self.n + 1; }
}

@main
func main() -> Int64 {
    var h: any Counter = Handle(n: 1);
    h.increment();
    h.increment();
    if h.get() != 3 { return 1 }
    0
}
