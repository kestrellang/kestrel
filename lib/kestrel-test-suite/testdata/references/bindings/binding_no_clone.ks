// test: execution
// stdlib: true
// backends: cranelift,llvm

// No-clone pin: creating and holding a named ref binding never clones the
// Cloneable referent; each VALUE-context read (`let s = r`) decays to
// exactly one clone, with the borrow live across them. Counts captured
// straight-line; asserts after the binding's last use (block-local rule).
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
}

struct Holder {
    var t: Tracked
    func peek() -> &Tracked { self.t }
}

@main
func main() -> lang.i64 {
    let clones = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    clones.write(0);
    let h = Holder(t: Tracked(payload: "alpha", clones: clones));

    let r = &h.peek();
    let c0 = clones.read();
    let s = r;
    let c1 = clones.read();
    let t = r;
    let c2 = clones.read();

    if c0 != 0 { return 1; }
    if c1 != 1 { return 2; }
    if c2 != 2 { return 3; }
    if s.payload != "alpha" { return 4; }
    if t.payload != "alpha" { return 5; }
    0
}
