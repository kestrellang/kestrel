// test: diagnostics
// stdlib: false

module Test

struct Box[T] { }
type Bad = Box[lang.i64, lang.str]; // ERROR: too many type arguments
