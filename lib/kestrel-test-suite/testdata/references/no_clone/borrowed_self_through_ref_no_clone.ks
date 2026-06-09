// test: execution
// stdlib: true
// backends: cranelift,llvm
// skip: stage1 — needs Array.at(index:) (S4)

// No-clone pin: a borrowed-self method called THROUGH `&T` must not clone
// the pointee. The read-only through-ref tests pass identically under a
// silently-inserted clone (the CopyValue→clone mono-expand machinery is
// this codebase's most precedented failure mode) — only the counter can
// tell. Snapshot/delta absorbs any construction-time clones.
module Test

import std.memory.(Pointer, Layout, SystemAllocator)
import std.numeric.(Int64)

struct Tracked: Cloneable {
    var payload: String
    var clones: Pointer[Int64]
    func clone() -> Tracked {
        self.clones.write(self.clones.read() + 1);
        // payload-clone, never `{ self }` (the aliasing footgun)
        Tracked(payload: self.payload.clone(), clones: self.clones)
    }
    func size() -> Int64 { self.payload.byteCount }
}

@main
func main() -> lang.i64 {
    let clones = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    clones.write(0);

    let arr = [Tracked(payload: "alpha", clones: clones)];
    let baseline = clones.read();

    // borrowed-self call through the ref: zero clones
    if arr.at(index: 0).size() != 5 { return 1; }
    if clones.read() != baseline { return 2; }
    0
}
