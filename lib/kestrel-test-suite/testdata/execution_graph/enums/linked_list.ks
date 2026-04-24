// test: diagnostics
// stdlib: false

module Main

indirect enum List {
    case Cons(head: lang.i64, tail: List)
    case Nil
}
