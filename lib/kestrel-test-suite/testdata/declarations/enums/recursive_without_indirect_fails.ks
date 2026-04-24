// test: diagnostics
// stdlib: false

module Test

enum Tree { // ERROR: recursive enum requires `indirect`
    case Leaf(value: lang.i64)
    case Node(left: Tree, right: Tree)
}
