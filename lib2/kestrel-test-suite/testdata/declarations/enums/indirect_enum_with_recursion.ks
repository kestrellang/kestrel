// test: diagnostics
// stdlib: false

module Test

indirect enum Tree[T] {
    case Leaf(value: T)
    case Node(left: Tree[T], right: Tree[T])
}
