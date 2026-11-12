// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// The parser loop shape: `var left` is read into a new parent node and then
// reassigned. That read is the old value's last use, so it moves (and the
// reassignment initializes the slot instead of dropping a value that's gone).
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

func depth(t: Tree) -> Int64 {
    match t {
        .Tip(_) => 1,
        .Bin(l, _) => 1 + depth(l)
    }
}

func build(n: Int64, logical: Bool) -> Tree {
    var left = Tree.Tip(Leaf(id: 0));
    var i = 1;
    while i < n {
        let right = Tree.Tip(Leaf(id: i));
        // Read in both branches, like `Logical(..)` vs `Binary(..)`.
        let kind = if logical and i > 1 {
            Tree.Bin(left: left, right: right)
        } else {
            Tree.Bin(left: right, right: left)
        };
        left = kind;
        i = i + 1;
    }
    left
}

@main
func main() -> lang.i64 {
    let a = build(5, false);
    let b = build(5, true);
    if depth(a) != 2 { return 1 };
    if depth(b) != 5 { return 2 };
    if clones != 0 { return 3 };
    if deinits != 0 { return 4 };
    0
}
