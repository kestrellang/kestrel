// test: diagnostics
// stdlib: false
module Test

struct Box[T] { var value: T }
extend Box[T, U] { func foo() { } } // ERROR: type parameter
