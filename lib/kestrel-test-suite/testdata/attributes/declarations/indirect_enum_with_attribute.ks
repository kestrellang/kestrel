// test: diagnostics
// stdlib: false
// skip: unbounded recursion in CopyBehavior over indirect enum — separate bug

module Test
@dummy
indirect enum Tree { // ERROR: indirect enums are not yet supported
    case Leaf(value: lang.i64)
    case Node(left: Tree, right: Tree)
}
