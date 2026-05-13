// test: diagnostics
// stdlib: false
// skip: unbounded recursion in CopyBehavior over indirect enum — separate bug

module Main

indirect enum List {
    case Cons(head: lang.i64, tail: List)
    case Nil
}
