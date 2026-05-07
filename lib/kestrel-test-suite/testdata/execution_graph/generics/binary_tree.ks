// test: diagnostics
// stdlib: false

module Main

indirect enum Tree[T] { // ERROR: indirect enums are not yet supported
    case Node(value: T, left: Tree[T], right: Tree[T])
    case Leaf
}
