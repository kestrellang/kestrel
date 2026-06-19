// test: execution
// stdlib: true
// backends: cranelift,llvm

// Property form of the pure ref pair: `var first: T { ref {…}
// mutating ref {…} }` — read, member-through, write, RMW, binding decay.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Cell {
    var p: Pointer[Int64]

    var value: Int64 {
        ref { self.p.value }
        mutating ref { self.p.mutatingValue }
    }
}

@main
func main() -> lang.i64 {
    let p = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    p.write(5);
    var c = Cell(p: p);

    if c.value != 5 { return 1; }
    c.value = 9;
    if c.value != 9 { return 2; }
    c.value += 3;
    if c.value != 12 { return 3; }

    let copied = c.value;
    c.value = 0;
    if copied != 12 { return 4; }
    0
}
