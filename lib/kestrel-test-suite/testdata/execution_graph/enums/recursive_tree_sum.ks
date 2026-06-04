// test: diagnostics
// stdlib: false
// skip: unbounded recursion in CopyBehavior over indirect enum — separate bug

module Main

indirect enum Tree { // ERROR: indirect enums are not yet supported
    case Leaf(value: lang.i64)
    case Node(left: Tree, right: Tree)
}

func sum(tree: Tree) -> lang.i64 {
    match tree {
        .Leaf(value) => value,
        .Node(left, right) => lang.i64_add(sum(left), sum(right))
    }
}
