// test: execution
// stdlib: true
// expect-exit: 0
//
// `any Iterator[Item = Int64]` over two iterator types; `next()` through the
// table, and `for-in` / `map` through self-conformance (design §9).

module Test

import std.numeric.(Int64)
import std.iter.(Iterator)

struct Upto { var n: Int64; var limit: Int64; }
extend Upto: Iterator {
    type Item = Int64
    public mutating func next() -> Int64? {
        if self.n >= self.limit { return .None }
        self.n = self.n + 1;
        .Some(self.n)
    }
}

struct Repeat { var v: Int64; var left: Int64; }
extend Repeat: Iterator {
    type Item = Int64
    public mutating func next() -> Int64? {
        if self.left == 0 { return .None }
        self.left = self.left - 1;
        .Some(self.v)
    }
}

func sum(xs: any Iterator[Item = Int64]) -> Int64 {
    var total: Int64 = 0;
    for x in xs { total = total + x; }
    total
}

@main
func main() -> Int64 {
    var a: any Iterator[Item = Int64] = Upto(n: 0, limit: 3);
    if let .Some(first) = a.next() { if first != 1 { return 1 } } else { return 2 }
    if sum(Upto(n: 0, limit: 4)) != 10 { return 3 }
    if sum(Repeat(v: 5, left: 2)) != 10 { return 4 }
    let doubled = Upto(n: 0, limit: 2).map(as: { it * 2 });
    let b: any Iterator[Item = Int64] = doubled;
    if sum(b) != 6 { return 5 }
    0
}
