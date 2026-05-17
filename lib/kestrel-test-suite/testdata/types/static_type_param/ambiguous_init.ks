// test: diagnostics
// stdlib: false

module Test

protocol Factory1 {
    init()
}
protocol Factory2 {
    init()
}
func make[T]() -> T where T: Factory1, T: Factory2 {
    return T() // ERROR: ambiguous
}
