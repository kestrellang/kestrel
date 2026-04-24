// test: diagnostics
// stdlib: false
module Test

protocol Equatable { }
type Foo: Equatable = lang.i64; // ERROR: type alias cannot have bounds
