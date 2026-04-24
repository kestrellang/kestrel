// test: diagnostics
// stdlib: false

module Test

struct SomeStruct { }
struct Container[T] where T: SomeStruct { } // ERROR: 'SomeStruct' is not a protocol
