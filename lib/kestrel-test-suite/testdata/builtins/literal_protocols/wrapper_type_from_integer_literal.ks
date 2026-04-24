// test: diagnostics
// stdlib: false
// include: _prelude_literal_protocols.ks

module Test
struct Percentage: Prelude.ExpressibleByIntegerLiteral {
    var value: lang.i64

    init(intLiteral value: lang.i64) {
        self.value = value
    }

    func asDecimal() -> lang.f64 {
        lang.f64_div(lang.cast_i64_f64(self.value), 100.0)
    }
}
func test() -> Percentage {
    let p: Percentage = 50;
    p
}
