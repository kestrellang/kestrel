// test: diagnostics
// stdlib: false

module Test
@dummy
indirect enum Tree {
    case Leaf(value: lang.i64)
    case Node(left: Tree, right: Tree)
}
