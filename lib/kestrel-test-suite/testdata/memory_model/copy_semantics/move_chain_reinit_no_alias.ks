// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#219 guardrail, forwarding hole): a `Take` may be aliased only if
// every use of its result settles the bytes before any same-slot reinit. A
// `move_value` (zero-copy aggregate rename) FORWARDS the take result's storage
// alias to a new value that can outlive the reinit. Here `let t = i; let u = t`
// move-chains i's aggregate; the take result `t` is consumed by the
// `move_value` into `u` (before the reinit), but `u` is read AFTER `i` is
// reinitialized. If the take were aliased, `u` would observe i's NEW value (9).
// mark_independent_takes must keep this `Take` independent (its result flows
// into a forwarder, not an alias-safe consumer). Found by adversarial review.

module Test

import std.numeric.Int64

public var drops: Int64 = 0;

struct Inner: not Copyable {
    var a: Int64
    var b: Int64
    deinit { drops = drops + 1; }
}

func consume(consuming i: Inner) -> Int64 { i.a }

@main
func main() -> lang.i64 {
    var i = Inner(a: 5, b: 6);
    let t = i;                   // move i out
    let u = t;                   // move_value chain forwards the aliased storage
    i = Inner(a: 9, b: 8);       // reinit i's slot — must NOT clobber `u`
    if consume(u) != 5 { return 1 };   // `u` must still be 5, not 9
    if consume(i) != 9 { return 2 };
    if drops != 2 { return 3 };
    0
}
