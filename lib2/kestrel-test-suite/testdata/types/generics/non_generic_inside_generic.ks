// test: diagnostics
// stdlib: false

module Test

struct Inner { var value: lang.i64 }
struct Outer[T] { var inner: Inner }
