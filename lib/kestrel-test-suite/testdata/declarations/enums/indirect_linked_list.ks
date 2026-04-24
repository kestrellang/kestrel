// test: diagnostics
// stdlib: false

module Test

indirect enum List[T] {
    case Cons(head: T, tail: List[T])
    case Nil
}
