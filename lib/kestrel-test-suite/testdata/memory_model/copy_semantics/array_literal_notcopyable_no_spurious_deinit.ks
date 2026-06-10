// test: execution
// stdlib: true
// skip: #127 — array-literal lowering deinits the moved-from element temp for a `not Copyable` element (spurious deinit at construction; the stored element will deinit AGAIN when the array drops). Unskip when fixing #127.

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
    var rs = [Res(tag: 1, drops: drops)];
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
