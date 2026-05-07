// test: diagnostics
// stdlib: false

module Test

indirect enum List[T] { // ERROR: indirect enums are not yet supported
    case Cons(head: T, tail: List[T])
    case Nil
}
