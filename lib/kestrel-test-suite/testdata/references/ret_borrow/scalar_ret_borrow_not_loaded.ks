// test: execution
// stdlib: true
// backends: cranelift,llvm

// THE resolve_scalar miscompile pin: a ret_borrow function must return
// the POINTER, not the loaded pointee. Write-through observation
// distinguishes them — if `.mutatingValue` returned the value, `bump`
// would mutate a temporary and the cell would stay 7. Runs on BOTH
// backends (each has its own load-through path).
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

func bump(mutating x: Int64) {
    x = x + 35;
}

@main
func main() -> lang.i64 {
    let cell = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    cell.write(7);

    bump(cell.mutatingValue);
    if cell.read() != 42 { return 1; }
    0
}
