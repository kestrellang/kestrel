// test: diagnostics
// stdlib: false

module Test

protocol Factory {
    init()
}
struct Container[T] where T: Factory {
    func makeOne() -> T {
        return T()
    }
}
