// test: diagnostics
// stdlib: false

module Main

indirect enum Tree[T] {
    case Node(value: T, left: Tree[T], right: Tree[T])
    case Leaf
}
