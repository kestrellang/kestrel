// test: diagnostics
// stdlib: false
//
// Boxing a recursive case needs the standard library's `OwnedBox`.

module Test

indirect enum List { // ERROR: indirect enums require the standard library
    case Nil
    case Cons(lang.i64, List)
}
