// test: diagnostics
// stdlib: false

module Test

protocol Display { }
struct Printer[T] where Tx: Display { } // ERROR: undeclared type parameter
