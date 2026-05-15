// test: diagnostics
// stdlib: false
// skip: unbounded recursion in CopyBehavior over indirect enum — separate bug

module Main

indirect enum List { // ERROR: indirect enums are not yet supported
    case Cons(head: lang.i64, tail: List)
    case Nil
}
