// test: execution
// stdlib: true
// expect-exit: 0

// #214: a field access chained directly off a static computed var
// (`Money.seven.cents`) must project the field, not collapse to the whole
// value. The path resolver stops at the `seven` getter (a value) and reports
// the remaining segments as leftover; HIR lowering must apply them as field
// accesses instead of dropping them. Previously `Money.seven.cents` was typed
// AND valued as the entire `Money`.

module Test

import std.numeric.Int64

struct Inner { var n: Int64; }

struct Money {
    var cents: Int64;
    var inner: Inner;
    static var seven: Money { Money(cents: 77, inner: Inner(n: 9)) }
}

@main
func main() -> lang.i32 {
    let c = Money.seven.cents;        // Int64 = 77
    if c != 77 { return 10 }

    // Deeper chain: static var -> field -> field.
    let n = Money.seven.inner.n;      // Int64 = 9
    if n != 9 { return 11 }

    // Used directly in an expression, not just a let binding.
    if Money.seven.cents + 3 != 80 { return 12 }
    0
}
