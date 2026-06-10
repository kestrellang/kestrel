// test: execution
// stdlib: true
// backends: cranelift,llvm

// §10.1: `&mutating T → &T` coerces one-way (free bit-copy under
// may-alias), and a borrow-convention argument position is a PLACE
// context — the coerced ref passes the referent place, no copy.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

func readIt(x: Int64) -> Int64 { x }

@main
func main() -> lang.i64 {
    let cell = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    cell.write(42);

    if readIt(cell.mutatingValue) != 42 { return 1; }
    0
}
