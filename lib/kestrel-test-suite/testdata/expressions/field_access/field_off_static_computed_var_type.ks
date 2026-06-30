// test: diagnostics
// stdlib: true

// #214: `Money.seven.cents` must be typed as the FIELD's type (Int64), not the
// whole `Money`. Passing it to a `(m: Money)` parameter is therefore a type
// mismatch — the soundness hole was that the dropped projection let this
// type-check (and miscompile) silently.

module Test

import std.numeric.Int64

struct Money {
    var cents: Int64;
    static var seven: Money { Money(cents: 77) }
}

func takesMoney(m: Money) -> Int64 { m.cents }

func test() -> Int64 {
    takesMoney(Money.seven.cents) // ERROR: expected Money got Int64
}
