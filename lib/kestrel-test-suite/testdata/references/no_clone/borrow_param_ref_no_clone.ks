// test: execution
// stdlib: true
// backends: cranelift,llvm
// skip: stage1 — needs Array.at(index:) (S4)

// Decision (a), borrow args are place contexts: a ref passed to a
// borrow-convention free-function param passes the referent place as the
// @guaranteed argument — `describe(arr.at(index: 0))` borrows in place
// exactly like `arr.at(index: 0).size()`. Zero clones.
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
}

func describe(t: Tracked) -> Int64 { t.size() }

@main
func main() -> lang.i64 {
    let clones = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    clones.write(0);

    let arr = [Tracked(payload: "alpha", clones: clones)];
    let baseline = clones.read();

    if describe(arr.at(index: 0)) != 5 { return 1; }
    if clones.read() != baseline { return 2; }
    0
}
