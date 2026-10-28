// test: execution
// stdlib: true
// backends: cranelift,llvm

// Ref accessors on a GENERIC container: the accessor children inherit the
// container's type parameter through the same path as setters (sig
// lowering's type-param inheritance + receiver type-arg prepending).
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Slot[T] {
    var p: Pointer[T]

    var item: T {
        ref { self.p.value }
        mutating ref { self.p.mutatingValue }
    }

    subscript(at index: Int64) -> T {
        ref { self.p.offset(by: index).value }
        mutating ref { self.p.offset(by: index).mutatingValue }
    }
}

@main
func main() -> lang.i64 {
    let p = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    p.write(5);
    var s = Slot[Int64](p: p);

    if s.item != 5 { return 1; }
    s.item = 9;
    if s.item != 9 { return 2; }
    s(at: 0) += 1;
    if s.item != 10 { return 3; }

    let sp = SystemAllocator().allocate(Layout.of[String]()).unwrap().cast[String]();
    sp.write("hi");
    var t = Slot[String](p: sp);
    if t.item != "hi" { return 4; }
    t.item = "yo";
    if t(at: 0) != "yo" { return 5; }
    0
}
