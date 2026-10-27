// test: execution
// stdlib: true
// expect-exit: 0
//
// `&mutating` binders reach into the boxed payload in place: inserting into
// a binary search tree recurses through them, and only the `.Leaf` it lands
// on is replaced (`self = .Node(…)`).

module Test

import std.numeric.(Int64)

indirect enum Tree {
    case Leaf
    case Node(Tree, Int64, Tree)

    mutating func insert(value: Int64) {
        match self {
            .Leaf => { self = .Node(.Leaf, value, .Leaf); },
            .Node(&mutating left, key, &mutating right) => {
                if value < key { left.insert(value) } else { right.insert(value) }
            }
        }
    }

    func min() -> Int64 {
        match self {
            .Leaf => -1,
            .Node(.Leaf, key, _) => key,
            .Node(left, _, _) => left.min()
        }
    }

    func sum() -> Int64 {
        match self {
            .Leaf => 0,
            .Node(left, key, right) => left.sum() + key + right.sum()
        }
    }
}

@main
func main() -> Int64 {
    var tree = Tree.Leaf;
    tree.insert(5);
    tree.insert(3);
    tree.insert(8);
    tree.insert(1);
    tree.insert(4);
    if tree.sum() != 21 { return 1 }
    if tree.min() != 1 { return 2 }
    0
}
