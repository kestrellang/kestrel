// test: diagnostics
// stdlib: true
//
// E151: converting to `any P[A = X]` requires the payload's `A` to be `X`
// (design §9).

module Test

import std.numeric.(Int64)
import std.core.(String)
import std.iter.(Iterator)

struct Counter { var n: Int64; }
extend Counter: Iterator {
    type Item = Int64
    public mutating func next() -> Int64? { .None }
}

func test() {
    let it: any Iterator[Item = String] = Counter(n: 0); // ERROR: cannot convert 'Counter' to 'any Iterator[Item = String]': its 'Item' is 'Int64'
}
