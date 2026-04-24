// test: diagnostics
// stdlib: false

module Test

protocol Factory {
    init()
    init(value: lang.i64)
}
func makeDefault[T]() -> T where T: Factory {
    return T()
}
func makeWithValue[T](v: lang.i64) -> T where T: Factory {
    return T(v)
}
