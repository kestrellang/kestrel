// test: diagnostics
// stdlib: false

module Test

type MyAlias = lang.i64;
struct Container[T] where T: MyAlias { } // ERROR: 'MyAlias' is not a protocol
