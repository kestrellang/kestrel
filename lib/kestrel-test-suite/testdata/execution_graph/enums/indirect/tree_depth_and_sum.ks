// test: execution
// stdlib: true
// expect-exit: 0
//
// A binary tree with two recursive fields in one boxed case, read through
// borrowed recursive methods.

module Test

import std.numeric.(Int64)

indirect enum Tree {
    case Leaf
    case Node(Tree, Int64, Tree)

    func sum() -> Int64 {
        match self {
            .Leaf => 0,
            .Node(left, value, right) => left.sum() + value + right.sum()
        }
    }

    func depth() -> Int64 {
        match self {
            .Leaf => 0,
            .Node(left, _, right) => {
                let l = left.depth();
                let r = right.depth();
                if l > r { 1 + l } else { 1 + r }
            }
        }
    }
}

@main
func main() -> Int64 {
    let tree = Tree.Node(
        .Node(.Leaf, 1, .Leaf),
        2,
        .Node(.Node(.Leaf, 3, .Leaf), 4, .Leaf)
    );
    if tree.sum() != 10 { return 1 }
    if tree.depth() != 3 { return 2 }
    0
}
