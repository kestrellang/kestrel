// test: diagnostics
// stdlib: false

module Test

protocol Factory {
    init(x: lang.i64, y: lang.i64)
}
func make[T](a: lang.i64, b: lang.i64) -> T where T: Factory {
    return T(a, b)
}
