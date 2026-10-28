// test: diagnostics
// stdlib: true

module Test

import std.numeric.(Int64)

struct Counter { var n: Int64 }

// Control for #178: the convention reconciliation at an annotated binding is
// one-way. A closure literal that DECLARES `mutating` bound to a plain
// `(Counter) -> ()` annotation must still error — MutBorrow is not assignable
// where Borrow is promised (same variance rule as the call-argument case).
func main() -> lang.i64 {
    let f: (Counter) -> () = { (mutating x) in x.n = x.n + 1; }; // ERROR: convention
    0
}
