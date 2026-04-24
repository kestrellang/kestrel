// test: diagnostics
// stdlib: false
module Test

protocol Container[T] {
    func fetch() -> T
}
extend Container {
    func doNothing() { }
}
