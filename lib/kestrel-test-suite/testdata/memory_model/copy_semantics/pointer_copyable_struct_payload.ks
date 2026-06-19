// test: execution
// stdlib: true
// backends: cranelift,llvm

// Regression (copy-drift #5, revised 2026-06-16): a generic method with
// `where T: Copyable` declared on a struct whose param is `not Copyable`
// (`Pointer.read() -> T where T: Copyable` on `struct Pointer[T] where
// T: not Copyable`; likewise `RcBox.getValue()` reached through it) produced a
// where-clause with BOTH a positive `Copyable` bound and the `not Copyable`
// relaxation on the same param. The MIR `copy_behavior` resolver was
// first-match-wins + a debug-only assert that the two never coexist — so a
// DEBUG build of any program instantiating `RcBox[S]`/`Pointer[S]` for a
// Copyable struct `S` panicked in ty_query.rs ("declaration order would decide
// its copy behavior"); release silently picked first-match. Fixed by letting a
// positive Copyable/Cloneable bound WIN over the `not Copyable` relaxation
// (the method's `where T: Copyable` narrows the struct's "need not"), which is
// order-independent. This test must build+run in BOTH profiles.
module Test

import std.memory.(RcBox, Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Point {
    var x: Int64
    var y: Int64
}

@main
func main() -> lang.i64 {
    // RcBox[Point].getValue() — value-out of a Copyable struct payload.
    let rc = RcBox(Point(x: 3, y: 4));
    let v = rc.getValue();
    if v.x != 3 { return 1; }
    if v.y != 4 { return 2; }

    // Pointer[Point].read() — the `-> T where T: Copyable` method directly.
    let p = SystemAllocator().allocate(Layout.of[Point]()).unwrap().cast[Point]();
    p.write(Point(x: 10, y: 20));
    let r = p.read();
    if r.x != 10 { return 3; }
    if r.y != 20 { return 4; }
    0
}
