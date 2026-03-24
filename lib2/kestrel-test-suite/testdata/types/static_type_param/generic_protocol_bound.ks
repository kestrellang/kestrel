// test: diagnostics
// stdlib: false

module Test

protocol Container[E] {
    static func empty() -> Self
}
func makeEmpty[T, E]() -> T where T: Container[E] {
    return T.empty()
}
