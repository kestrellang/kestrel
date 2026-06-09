// test: execution
// stdlib: true
// backends: cranelift,llvm

// Stage 0.5 Pointer bridge: `Pointer(to: x)` where `x` is `not Copyable`
// BORROWS the place — it must not move or copy the value. Detector
// (references-tests.md §"Detecting UAF"): `Resource.deinit` decrements a
// never-freed heap cell, so the resource must deinit exactly once, at
// scope exit. A capture-by-move would deinit early (read through the
// pointer sees a decremented cell) or twice (cell one-too-low at the end).
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Resource: not Copyable {
    var cell: Pointer[Int64]
    func value() -> Int64 { self.cell.read() }
    deinit { self.cell.write(self.cell.read() - 1); }
}

// The scope whose exit must run exactly one deinit.
func captureAndObserve(counter: Pointer[Int64]) -> Int64 {
    let r = Resource(cell: counter);
    let p = Pointer(to: r);
    // The capture borrowed `r` without consuming it: reading the field
    // through the pointer sees the live, not-yet-deinit'd resource.
    let observed = p.with { it.value() };
    if observed != 10 { return 1; }
    // `r` is still usable directly after the capture (not moved).
    if r.value() != 10 { return 2; }
    0
}

@main
func main() -> lang.i64 {
    let counter = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    counter.write(10);

    let status = captureAndObserve(counter);
    if status != 0 { return status.raw; }

    // Exactly one deinit ran, at `captureAndObserve` scope exit: 10 - 1 == 9.
    if counter.read() != 9 { return 3; }
    0
}
