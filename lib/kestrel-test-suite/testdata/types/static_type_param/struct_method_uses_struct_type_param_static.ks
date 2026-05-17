// test: diagnostics
// stdlib: false

module Test

protocol Factory {
    static func create() -> Self
}
struct Container[T] where T: Factory {
    func makeOne() -> T {
        return T.create()
    }
}
