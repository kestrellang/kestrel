// test: execution
// stdlib: true

module Test

import std.numeric.(Int64)

struct Counter { var n: Int64 }

// Higher-order fn taking a by-reference (mutating) closure param.
func apply(mutating c: Counter, with f: (mutating Counter) -> Int64) -> Int64 {
    f(c)
}

func main() -> lang.i64 {
    var c = Counter(n: 0);
    let r = apply(c, with: { (mutating x) in x.n = x.n + 5; x.n });
    // The closure mutated c in place, visible through the mutating param chain.
    if c.n != 5 { return 1 }
    if r != 5 { return 2 }
    0
}
