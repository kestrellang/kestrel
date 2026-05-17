// test: diagnostics
// stdlib: false

module Test

struct Box[T] {
    func identity[U](value: U) -> U { value }
}
