// test: diagnostics
// stdlib: false

module Test
@dummy
indirect enum Tree { // ERROR: indirect enums are not yet supported
    case Leaf(value: lang.i64)
    case Node(left: Tree, right: Tree)
}
