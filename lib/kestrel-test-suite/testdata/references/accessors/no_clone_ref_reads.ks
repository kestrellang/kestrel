// test: execution
// stdlib: true
// backends: cranelift,llvm

// No-clone pins: reads through a `ref` accessor borrow in place — member
// access, borrow-convention arguments, and RMW through the mutating ref
// never clone a Cloneable element. Only binding decay (an owned copy by
// design) clones, exactly once.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Tracked: Cloneable {
    var payload: String
    var clones: Pointer[Int64]
    func clone() -> Tracked {
        self.clones.write(self.clones.read() + 1);
        Tracked(payload: self.payload.clone(), clones: self.clones)
    }
    func size() -> Int64 { self.payload.byteCount }
    mutating func grow() { self.payload.append("!"); }
}

struct Buf {
    var p: Pointer[Tracked]
    subscript(at index: Int64) -> Tracked {
        ref { self.p.offset(by: index).value }
        mutating ref { self.p.offset(by: index).mutatingValue }
    }
}

func describe(t: Tracked) -> Int64 { t.size() }

@main
func main() -> lang.i64 {
    let clones = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    clones.write(0);
    let p = SystemAllocator().allocate(Layout.of[Tracked]()).unwrap().cast[Tracked]();
    p.write(Tracked(payload: "alpha", clones: clones));
    var b = Buf(p: p);
    let baseline = clones.read();

    // Member read through the place — no clone.
    if b(at: 0).size() != 5 { return 1; }
    if clones.read() != baseline { return 2; }

    // Borrow-convention argument — borrows the place, no clone.
    if describe(b(at: 0)) != 5 { return 3; }
    if clones.read() != baseline { return 4; }

    // Mutating method through the mutating ref — in place, no clone.
    b(at: 0).grow();
    if b(at: 0).size() != 6 { return 5; }
    if clones.read() != baseline { return 6; }

    // Binding decay clones exactly once.
    let copied = b(at: 0);
    if clones.read() != baseline + 1 { return 7; }
    if copied.size() != 6 { return 8; }
    0
}
