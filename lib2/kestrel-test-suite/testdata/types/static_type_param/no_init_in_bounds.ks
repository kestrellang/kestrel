// test: diagnostics
// stdlib: false

module Test

protocol Empty {
    func doSomething() -> lang.i64
}
func make[T]() -> T where T: Empty {
    return T() // ERROR: init
}
