// test: diagnostics
// stdlib: false

module Test
struct Number: Prelude.ExpressibleByIntegerLiteral, Prelude.ExpressibleByFloatLiteral {
    var value: lang.f64

    init(intLiteral value: lang.i64) {
        self.value = lang.cast_i64_f64(value)
    }

    init(floatLiteral value: lang.f64) {
        self.value = value
    }
}
func test() {
    let a: Number = 42;
    let b: Number = 3.14;
}
