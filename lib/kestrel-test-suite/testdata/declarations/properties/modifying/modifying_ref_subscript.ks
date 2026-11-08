// test: execution
// stdlib: true
// backends: cranelift,llvm

// `modifying ref` on a subscript paired with `mutating ref` for writes.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Slots {
    var p: Pointer[Int64]
    var reads: Int64

    subscript(i: Int64) -> Int64 {
        modifying ref {
            self.reads += 1;
            unsafe self.p.offset(by: i).value
        }
        mutating ref { unsafe self.p.offset(by: i).mutatingValue }
    }
}

@main
func main() -> lang.i64 {
    let p = SystemAllocator().allocate(Layout.array[Int64](4)).unwrap().cast[Int64]();
    unsafe p.write(10);
    unsafe p.offset(by: 1).write(20);
    var s = Slots(p: p, reads: 0);
    if s(0) != 10 { return 1; }
    if s(1) != 20 { return 2; }
    s(1) = 25;
    s(1) += 1;
    if s(1) != 26 { return 3; }
    if s.reads != 3 { return 4; }
    0
}
