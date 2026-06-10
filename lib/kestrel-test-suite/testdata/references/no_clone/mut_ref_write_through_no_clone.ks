// test: execution
// stdlib: true
// backends: cranelift,llvm

// Kills the clone-mutate-writeback impostor: mut_ref_pass_through cannot
// distinguish a true write-through from "clone, mutate the clone, write it
// back" — the clone counter can. Also proves makeUnique no-ops on unique
// storage (the COW make-unique runs BEFORE the ref is fabricated).
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
    mutating func rename(to name: String) { self.payload = name; }
}

@main
func main() -> lang.i64 {
    let clones = SystemAllocator().allocate(Layout.of[Int64]()).unwrap().cast[Int64]();
    clones.write(0);

    var arr = [Tracked(payload: "alpha", clones: clones)];
    let baseline = clones.read();

    arr.mutableAt(index: 0).rename(to: "renamed");
    if clones.read() != baseline { return 1; }

    // observe through another borrow (a value subscript would clone)
    if arr.at(index: 0).size() != 7 { return 2; }
    if clones.read() != baseline { return 3; }
    0
}
