// test: execution
// stdlib: true

module Test

import std.numeric.(Int64)

struct Counter { var n: Int64 }

// The closure literal omits `mutating`; its param convention is inferred
// from `with`'s expected type `(mutating Counter) -> Int64`.
func apply(mutating c: Counter, with f: (mutating Counter) -> Int64) -> Int64 {
    f(c)
}

@main
func main() -> lang.i64 {
    var c = Counter(n: 10);
    let r = apply(c, with: { (x) in x.n = x.n + 3; x.n });
    if c.n != 13 { return 1 }
    if r != 13 { return 2 }
    0
}
