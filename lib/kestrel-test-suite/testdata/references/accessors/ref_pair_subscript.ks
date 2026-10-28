// test: execution
// stdlib: true
// backends: cranelift,llvm

// Pure ref pair (stage 1.5): a subscript whose READ provider is `ref` and
// WRITE provider is `mutating ref`. Reads borrow in place, `=` stores
// through the fabricated place, `+=` and mutating methods run in place.
// The post-mutation probes prove reads decay to owned copies.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Score {
    var points: Int64
    mutating func double() { self.points = self.points * 2; }
}

struct Buf {
    var p: Pointer[Score]

    subscript(at index: Int64) -> Score {
        ref { self.p.offset(by: index).value }
        mutating ref { self.p.offset(by: index).mutatingValue }
    }
}

@main
func main() -> lang.i64 {
    let p = SystemAllocator().allocate(Layout.of[Score]()).unwrap().cast[Score]();
    p.write(Score(points: 5));
    var b = Buf(p: p);

    // Read through the ref accessor (member access through the place).
    if b(at: 0).points != 5 { return 1; }

    // Plain assignment through the mutating ref accessor.
    b(at: 0) = Score(points: 8);
    if b(at: 0).points != 8 { return 2; }

    // Mutating method through the accessor place — in place, no writeback.
    b(at: 0).double();
    if b(at: 0).points != 16 { return 3; }

    // Binding decay: an owned copy, not an alias.
    let snapshot = b(at: 0);
    b(at: 0) = Score(points: 1);
    if snapshot.points != 16 { return 4; }
    if b(at: 0).points != 1 { return 5; }
    0
}
