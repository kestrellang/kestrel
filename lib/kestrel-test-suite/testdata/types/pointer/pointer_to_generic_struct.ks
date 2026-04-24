// test: diagnostics
// stdlib: false

module Test

struct Box[T] { let value: T }
type BoxPtr = lang.ptr[Box[lang.i64]];
