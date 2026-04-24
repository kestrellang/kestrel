// test: diagnostics
// stdlib: false

module Test

struct Box[T] { }
type Deep = Box[Box[Box[Box[lang.i64]]]];
