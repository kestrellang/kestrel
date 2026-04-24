// test: diagnostics
// stdlib: false

module Test

struct Box[T] { }
type Boxed[T] = Box[T];
type DoubleBoxed[T] = Boxed[Boxed[T]];
