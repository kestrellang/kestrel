// test: diagnostics
// stdlib: false

module Main

indirect enum List[T] {
    case Cons(head: T, tail: List[T])
    case Nil
}
