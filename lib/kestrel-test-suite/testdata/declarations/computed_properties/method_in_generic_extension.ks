// test: diagnostics
// stdlib: false

module Test

struct Container[T] {
    var item: T
}

extend Container[T] {
    func wrapped() -> T {
        self.item
    }
}
