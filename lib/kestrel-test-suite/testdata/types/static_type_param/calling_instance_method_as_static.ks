// test: diagnostics
// stdlib: false

module Test

protocol Factory {
    func instanceMethod() -> Self
}
func make[T]() -> T where T: Factory {
    return T.instanceMethod() // ERROR:
}
