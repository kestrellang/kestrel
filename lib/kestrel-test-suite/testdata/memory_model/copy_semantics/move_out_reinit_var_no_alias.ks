// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression (#219 guardrail): aliasing a `Take` whose source slot is later
// re-initialized while the moved value is still live would re-introduce the
// #141 double-free / clobber. `var x = agg; let y = x; x = new` moves x's
// aggregate out into `y`, then reinitializes x's slot — `y` must stay an
// INDEPENDENT value, never an alias of x's storage. The mark_independent_takes
// pass must keep this `Take` as a copy: the source roots to x's StackAlloc and
// a later store targets the same slot while `y` is still live. If it were
// wrongly aliased, `y.a` would observe the new value (9) and the deinit
// accounting would double-count.

module Test

import std.numeric.Int64

public var drops: Int64 = 0;

struct Box: not Copyable {
    var a: Int64
    var b: Int64
    var c: Int64
    deinit { drops = drops + 1; }
}

func consume(consuming x: Box) -> Int64 { x.a }

@main
func main() -> lang.i64 {
    var x = Box(a: 1, b: 2, c: 3);
    let y = x;                    // move x's aggregate out into `y`
    x = Box(a: 9, b: 8, c: 7);    // reinit x's slot — `y` must NOT observe this
    if y.a != 1 { return 1 };     // `y` is still the OLD value (1), not 9
    if drops != 0 { return 2 };   // nothing dropped yet — both live
    let yv = consume(y);          // drops y (a == 1)
    if yv != 1 { return 3 };
    if drops != 1 { return 4 };
    let xv = consume(x);          // drops the reinitialized x (a == 9)
    if xv != 9 { return 5 };
    if drops != 2 { return 6 };
    0
}
