// test: diagnostics
// stdlib: false
// include: _prelude_literal_protocols.ks

module Test
struct MyInt: Prelude.ExpressibleByIntegerLiteral {
    var value: lang.i64

    init(intLiteral value: lang.i64) {
        self.value = value
    }
}
func test() -> MyInt {
    42
}
