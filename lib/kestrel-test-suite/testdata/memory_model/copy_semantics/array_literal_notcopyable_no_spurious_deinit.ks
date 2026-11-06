// test: diagnostics
// stdlib: true
// Arrays require Copyable elements (decision for C16): an array literal of a
// `not Copyable` type is rejected like the annotation `Array[Res]`. It used to
// be accepted, and reading an element back out trapped at run time.
//
// History:
// A `not Copyable` element MOVES into an array literal: exactly zero
// deinits may fire during construction, and exactly one when the array
// (and with it the element) goes out of scope.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Res: not Copyable {
    var tag: Int64
    var drops: Pointer[Int64]
    deinit { self.drops.write(self.drops.read() + 1); }
}

func build(drops: Pointer[Int64]) -> Int64 {
    var rs = [Res(tag: 1, drops: drops)]; // ERROR: Res !: Copyable
    // construction is a MOVE: no deinit yet
    if drops.read() != 0 { return 1; }
    0
}

@main
func main() -> lang.i64 {
    let drops = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    drops.write(0);
    let status = build(drops);
    if status != 0 { return status.raw; }
    // exactly one deinit, at `build` scope exit
    if drops.read() != 1 { return 2; }
    0
}
