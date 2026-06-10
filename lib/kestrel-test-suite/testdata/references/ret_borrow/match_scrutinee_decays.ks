// test: execution
// stdlib: true
// backends: cranelift,llvm

// A `match` scrutinee is a VALUE context: the ref decays (copy-out) before
// the match machinery, so arms see an owned value — a ref never enters
// pattern matching.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

@main
func main() -> lang.i64 {
    let cell = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    cell.write(7);

    let r = match cell.value {
        7 => 0,
        _ => 1
    };
    if r != 0 { return 1; }
    0
}
