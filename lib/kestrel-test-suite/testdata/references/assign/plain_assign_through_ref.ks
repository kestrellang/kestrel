// test: execution
// stdlib: true
// backends: cranelift,llvm

// Plain assignment through a `&mutating T` place: lowered as PtrTo on the
// @guaranteed ref result + StoreAssign — the OLD pointee is dropped
// exactly once (deinit-count delta), the new value is stored in place,
// and a COW snapshot taken before the write is untouched.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Tracked {
    var tag: Int64
    var drops: Pointer[Int64]
    deinit { self.drops.write(self.drops.read() + 1); }
}

@main
func main() -> lang.i64 {
    var arr = [11, 22, 33];
    arr.mutableAt(index: 0) = 5;
    if arr(0) != 5 { return 1; }
    if arr(1) != 22 { return 2; }

    let snap = arr;
    arr.mutableAt(index: 2) = 1000;
    if arr(2) != 1000 { return 3; }
    if snap(2) != 33 { return 4; }

    // the overwritten element deinits exactly once, at the store
    // (delta-based: literal construction has its own temp deinit)
    let drops = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    drops.write(0);
    var ts = [Tracked(tag: 1, drops: drops)];
    let baseline = drops.read();
    ts.mutableAt(index: 0) = Tracked(tag: 2, drops: drops);
    if drops.read() != baseline + 1 { return 5; }
    // read back through a borrowed view — no element copy, no extra deinit
    if ts.at(index: 0).tag != 2 { return 6; }
    if drops.read() != baseline + 1 { return 7; }

    // Pointer bridge getter as the place
    let cell = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    cell.write(7);
    cell.mutatingValue = 42;
    if cell.read() != 42 { return 8; }
    0
}
