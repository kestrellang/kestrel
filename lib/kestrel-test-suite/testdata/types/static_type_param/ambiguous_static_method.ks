// test: diagnostics
// stdlib: false

module Test

protocol Factory1 {
    static func create() -> Self
}
protocol Factory2 {
    static func create() -> Self
}
func make[T]() -> T where T: Factory1, T: Factory2 {
    return T.create() // ERROR: ambiguous
}
