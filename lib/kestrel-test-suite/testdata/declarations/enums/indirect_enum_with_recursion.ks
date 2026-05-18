// test: diagnostics
// stdlib: false

module Test

indirect enum Tree[T] { // ERROR: indirect enums are not yet supported
    case Leaf(value: T)
    case Node(left: Tree[T], right: Tree[T])
}
