// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// A `let` whose last use is storing it into an aggregate is moved there, not
// cloned. Building a tree bottom-up (`let left = ..; Node(.Bin(left: left))`)
// used to clone each child's whole subtree through the `indirect` box, which
// made the build quadratic. Expect no clones, and each node dropped once.
module Test

import std.numeric.Int64

public var clones: Int64 = 0;
public var deinits: Int64 = 0;

struct Leaf: Cloneable {
    var id: Int64
    func clone() -> Leaf { clones = clones + 1; return Leaf(id: self.id); }
    deinit { deinits = deinits + 1; }
}

indirect enum Tree {
    case Tip(Leaf)
    case Bin(left: Tree, right: Tree)
}

func sum(t: Tree) -> Int64 {
    match t {
        .Tip(l) => l.id,
        .Bin(l, r) => sum(l) + sum(r)
    }
}

func wrap(l: Tree, i: Int64) -> Tree {
    let right = Tree.Tip(Leaf(id: i));
    let left = l;
    .Bin(left: left, right: right)
}

@main
func main() -> lang.i64 {
    var t = Tree.Tip(Leaf(id: 0));
    var i = 1;
    while i <= 4 {
        t = wrap(t, i);
        i = i + 1;
    }
    if sum(t) != 10 { return 1 };
    if clones != 0 { return 2 };
    if deinits != 0 { return 3 };
    0
}
