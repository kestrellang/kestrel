// test: diagnostics
// stdlib: false

module Main

indirect enum Tree {
    case Leaf(value: lang.i64)
    case Node(left: Tree, right: Tree)
}
