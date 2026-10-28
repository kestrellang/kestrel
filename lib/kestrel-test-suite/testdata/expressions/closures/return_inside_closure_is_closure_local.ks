// test: execution
// stdlib: true

module Test

import std.numeric.(Int64)

// Regression (#199): `return e` inside a closure returns from the CLOSURE
// (closure-local, Swift-style), checked against the closure's return type — NOT
// the enclosing function. Previously the closure typed as `() -> Never`, its
// body lowered to a trap (calling it aborted), and generic-callee inference
// collapsed the param to the Never-typed return.

func callTwice(f: () -> Int64) -> Int64 {
    let a = f();
    let b = f();
    a + b
}

// A closure that early-returns from a branch, then falls through to a tail.
func pick(n: Int64, f: (Int64) -> Int64) -> Int64 { f(n) }

@main
func main() -> lang.i64 {
    // Both calls run and each returns 5 → 10 (the closure does NOT divert the
    // caller; "after" the call still executes).
    if callTwice({ return 5 }) != 10 { return 1 }

    // Mixed early-return + tail value inside one closure.
    let r = pick(3, { (x) in
        if x > 0 { return x * 10 };
        -1
    });
    if r != 30 { return 2 }

    let r2 = pick(-2, { (x) in
        if x > 0 { return x * 10 };
        -1
    });
    if r2 != -1 { return 3 }

    0
}
