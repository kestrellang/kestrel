// test: diagnostics
// stdlib: false
// include: _prelude_literal_protocols.ks

module Test
struct MyType {
    var value: lang.i64
}
func test() -> MyType {
    42 // ERROR: type mismatch
}
