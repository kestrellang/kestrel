// test: diagnostics
// stdlib: false

module Test

protocol Factory[P] {
    func produce() -> P
}

struct Container[T] {
    var item: T

    init[F](factory: F) where F: Factory[T] {
        self.item = factory.produce()
    }
}
