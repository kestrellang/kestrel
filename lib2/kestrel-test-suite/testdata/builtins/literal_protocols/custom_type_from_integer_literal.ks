// test: diagnostics
// stdlib: false

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
