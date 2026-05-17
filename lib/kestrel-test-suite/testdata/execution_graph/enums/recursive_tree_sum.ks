// test: diagnostics
// stdlib: false

module Main

indirect enum Tree {
    case Leaf(value: lang.i64)
    case Node(left: Tree, right: Tree)
}

func sum(tree: Tree) -> lang.i64 {
    match tree {
        .Leaf(value) => value,
        .Node(left, right) => lang.i64_add(sum(left), sum(right))
    }
}
