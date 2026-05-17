// test: diagnostics
// stdlib: false

module Test

protocol Factory {
    init(value: lang.i64)
}
func make[T](v: lang.i64) -> T where T: Factory {
    return T(v)
}
