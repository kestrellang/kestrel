// test: diagnostics
// stdlib: false

module Test

protocol Container[T] { }
struct Box[T]: Container[T] { }
