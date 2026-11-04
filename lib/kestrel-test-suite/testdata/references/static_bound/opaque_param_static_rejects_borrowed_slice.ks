// test: diagnostics
// stdlib: true

// Without `and not Static`, an opaque param keeps the implicit `Static`
// bound, so a borrowing `ArraySlice` is rejected.

module Test

import std.collections.(Array, ArraySlice, Slice)

func count(xs: some Slice[Int64]) -> Int64 { xs.asUnsafeSlice().count }

func caller() -> Int64 {
    let xs = [1, 2, 3];
    count(xs(1..)) // ERROR: !: Static
}
