// test: diagnostics
// stdlib: false

module Test

struct Box[T] { }
type Boxed[T] = Box[T];
type BoxedInt = Boxed[lang.i64];
