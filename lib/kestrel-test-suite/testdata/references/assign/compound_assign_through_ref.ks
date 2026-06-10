// test: execution
// stdlib: true
// backends: cranelift,llvm

// Compound assignment through a `&mutating T` place: the desugared
// `addAssign` receiver is the ref result, used in place — the write lands
// in the array's storage / the pointee, never in a temporary.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

@main
func main() -> lang.i64 {
    var arr = [11, 22, 33];
    arr.mutableAt(index: 1) += 77;
    if arr(1) != 99 { return 1; }
    if arr(0) != 11 { return 2; }

    // COW isolation: the write is invisible through a prior snapshot
    let snap = arr;
    arr.mutableAt(index: 2) += 1;
    if arr(2) != 34 { return 3; }
    if snap(2) != 33 { return 4; }

    // Pointer bridge getter as the place
    let cell = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    cell.write(40);
    cell.mutatingValue += 2;
    if cell.read() != 42 { return 5; }
    0
}
