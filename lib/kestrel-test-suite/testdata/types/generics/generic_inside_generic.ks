// test: diagnostics
// stdlib: false

module Test

struct Inner[T] { var value: T }
struct Outer[U] { var inner: Inner[U] }
