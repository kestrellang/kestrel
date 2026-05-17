// test: diagnostics
// stdlib: false
// include: try_prelude.ks

module Test
struct MyResult {
    var value: lang.i64
}
extend MyResult: Prelude.FromResidual[lang.str] {
    static func fromResidual(residual: lang.str) -> MyResult {
        MyResult(value: 0)
    }
}
