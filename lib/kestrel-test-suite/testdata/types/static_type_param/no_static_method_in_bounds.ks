// test: diagnostics
// stdlib: false

module Test

protocol Factory {
    func instanceMethod() -> lang.i64
}
func make[T]() -> T where T: Factory {
    return T.create() // ERROR: create
}
