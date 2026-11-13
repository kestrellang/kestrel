// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// A last use on one branch only: `x` is stored on the `then` path and merely
// read on the `else` path, and the slot is dropped after the join. The store
// could move (with the drop sunk into the `else` path), but the last-use move
// only rewrites drops that every path reaches moved-out, so today this still
// clones once. Documents the gap; should reach 0 clones.
module Test

import std.numeric.Int64

public var clones: Int64 = 0;

struct Leaf: Cloneable {
    var id: Int64
    func clone() -> Leaf { clones = clones + 1; return Leaf(id: self.id); }
}

struct Holder { var leaf: Leaf }

func pick(store: Bool) -> Int64 {
    let x = Leaf(id: 5);
    if store {
        let h = Holder(leaf: x);
        return h.leaf.id;
    }
    x.id + 1
}

func branchStore(store: Bool) -> Int64 {
    let x = Leaf(id: 5);
    var out = 0;
    if store {
        let h = Holder(leaf: x);
        out = h.leaf.id;
    } else {
        out = x.id + 1;
    }
    out
}

@main
func main() -> lang.i64 {
    if pick(true) != 5 { return 1 };
    if pick(false) != 6 { return 2 };
    if clones != 0 { return 3 };       // early return: already a move
    if branchStore(true) != 5 { return 4 };
    if branchStore(false) != 6 { return 5 };
    if clones != 0 { return 6 };       // join drop: not yet a move
    0
}
