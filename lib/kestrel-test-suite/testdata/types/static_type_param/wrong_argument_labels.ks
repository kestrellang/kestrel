// test: diagnostics
// stdlib: false

module Test

protocol Factory {
    init(value: lang.i64)
}
func make[T]() -> T where T: Factory {
    return T(wrong: 1) // ERROR:
}
