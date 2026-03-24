// test: diagnostics
// stdlib: false

module Test

protocol Container[E] {
    func first() -> E
}
func getFirst[T, E](c: T) -> E where T: Container[E] {
    c.first()
}
